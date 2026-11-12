//! `ortho lsp`: ortho as a language server, over stdin and stdout, for any editor that
//! speaks the Language Server Protocol. The `.sysml` files in a folder are one model,
//! as for `ortho render`: the server reports the problems of every file in the folder
//! of each file the editor opens, as it is typed, goes to what a name names, and shows
//! the model's diagrams in a page in the browser, drawn again on every save. It
//! completes and renames nothing: it helps check and review a model.
mod preview;
mod protocol;

use crate::model::views::{declared_views, view_file_name, view_heading};
use crate::parser::ast::{NodeId, Prop, Value as Field};
use crate::parser::workspace::{Severity, Workspace};
use crate::pipeline::{relative_to_working_folder, render_views};
use crate::render::theme::{LIGHT, Theme};
use preview::{Note, Preview, Shown};
use protocol::{offset_at, path_to_uri, position_of, read_message, uri_to_path, write_message};
use serde_json::{Value, json};
use std::collections::HashMap;
use std::io::{self, BufRead, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

/// The command a code action runs to show a view in the preview.
const SHOW_IN_PREVIEW: &str = "ortho.showInPreview";

/// Serves one editor until it says `exit` or closes the stream. Says whether it was
/// told to shut down first, as a clean exit requires.
pub fn serve(input: impl BufRead, output: impl Write + Send + 'static) -> io::Result<bool> {
    let mut input = input;
    let mut server = Server::new(Editor::new(output));
    while let Some(message) = read_message(&mut input)? {
        if message.get("method").and_then(Value::as_str) == Some("exit") {
            break;
        }
        server.handle(&message)?;
    }
    Ok(server.shut_down)
}

/// The editor served, to write to from the server and from the preview's page alike.
#[derive(Clone)]
struct Editor {
    output: Arc<Mutex<dyn Write + Send>>,
    /// How many requests of its own the server has sent.
    sent: Arc<AtomicU64>,
    /// Whether the editor opens a document or an address when asked to.
    shows_documents: Arc<AtomicBool>,
}

impl Editor {
    fn new(output: impl Write + Send + 'static) -> Self {
        Editor { output: Arc::new(Mutex::new(output)), sent: Arc::default(), shows_documents: Arc::default() }
    }

    fn send(&self, message: Value) -> io::Result<()> {
        write_message(&mut *self.output.lock().unwrap(), &message)
    }

    fn notify(&self, method: &str, params: Value) -> io::Result<()> {
        self.send(json!({ "jsonrpc": "2.0", "method": method, "params": params }))
    }

    fn request(&self, method: &str, params: Value) -> io::Result<()> {
        let id = format!("ortho-{}", self.sent.fetch_add(1, Ordering::Relaxed) + 1);
        self.send(json!({ "jsonrpc": "2.0", "id": id, "method": method, "params": params }))
    }

    /// Opens an address in the browser, or says where it is if the editor opens none.
    fn open_url(&self, url: &str) -> io::Result<()> {
        if self.shows_documents.load(Ordering::Relaxed) {
            self.request("window/showDocument", json!({ "uri": url, "external": true }))
        } else {
            self.notify("window/showMessage", json!({ "type": 3, "message": format!("ortho preview: {url}") }))
        }
    }
}

/// A problem at a place in a file.
struct Problem {
    severity: Severity,
    offset: usize,
    message: String,
}

/// A problem, with the file it is in and the file's text.
type Found<'a> = (&'a PathBuf, &'a String, &'a Problem);

/// A model: the `.sysml` files of one folder.
#[derive(Default)]
struct Model {
    /// The model as last checked, while every file parses.
    workspace: Option<Workspace>,
    /// What rendering found, by file, until the model is changed again.
    rendering: HashMap<PathBuf, Vec<Problem>>,
}

struct Server {
    editor: Editor,
    /// The text of each file the editor has open, which may not be saved.
    open: HashMap<PathBuf, String>,
    /// Each model with a file open, by its folder.
    models: HashMap<PathBuf, Model>,
    /// What was last reported for each file, so a file is reported again only when
    /// that changes.
    reported: HashMap<PathBuf, Value>,
    /// The page that shows the diagrams, once the editor has said where the project is.
    preview: Option<Preview>,
    /// Whether the page is opened in the browser when the first model file is.
    opens_preview: bool,
    theme: &'static Theme,
    shut_down: bool,
}

impl Server {
    fn new(editor: Editor) -> Self {
        Server {
            editor,
            open: HashMap::new(),
            models: HashMap::new(),
            reported: HashMap::new(),
            preview: None,
            opens_preview: true,
            theme: &LIGHT,
            shut_down: false,
        }
    }

    fn send(&mut self, message: Value) -> io::Result<()> {
        self.editor.send(message)
    }

    fn handle(&mut self, message: &Value) -> io::Result<()> {
        let Some(method) = message.get("method").and_then(Value::as_str) else {
            // The editor's answer to a request of ours, such as to show a document.
            return Ok(());
        };
        let params = message.get("params").cloned().unwrap_or(Value::Null);
        let id = message.get("id").cloned();
        let result = match method {
            "initialize" => Some(self.initialize(&params)),
            "shutdown" => {
                self.shut_down = true;
                Some(Value::Null)
            }
            "textDocument/didOpen" => {
                if let Some(path) = document_path(&params) {
                    let text = params["textDocument"]["text"].as_str().unwrap_or("").to_string();
                    self.open.insert(path.clone(), text);
                    self.check(&path, true)?;
                    if std::mem::take(&mut self.opens_preview)
                        && let Some(preview) = &self.preview
                    {
                        preview.open_unless_watched();
                    }
                }
                None
            }
            "textDocument/didChange" => {
                let text = params["contentChanges"].as_array().and_then(|changes| changes.last());
                if let (Some(path), Some(text)) = (document_path(&params), text.and_then(|c| c["text"].as_str())) {
                    self.open.insert(path.clone(), text.to_string());
                    self.check(&path, false)?;
                }
                None
            }
            "textDocument/didSave" => {
                if let Some(path) = document_path(&params) {
                    if let Some(text) = params["text"].as_str() {
                        self.open.insert(path.clone(), text.to_string());
                    }
                    self.check(&path, true)?;
                }
                None
            }
            "textDocument/didClose" => {
                if let Some(path) = document_path(&params) {
                    self.open.remove(&path);
                    self.check(&path, false)?;
                }
                None
            }
            "textDocument/definition" => Some(self.definition(&params)),
            "textDocument/codeAction" => Some(self.code_actions(&params)),
            "workspace/executeCommand" => Some(self.execute(&params)?),
            _ if id.is_some() => {
                let error = json!({ "code": -32601, "message": format!("ortho does not answer {method}") });
                return self.send(json!({ "jsonrpc": "2.0", "id": id, "error": error }));
            }
            _ => None,
        };
        match (id, result) {
            (Some(id), Some(result)) => self.send(json!({ "jsonrpc": "2.0", "id": id, "result": result })),
            (Some(id), None) => self.send(json!({ "jsonrpc": "2.0", "id": id, "result": null })),
            _ => Ok(()),
        }
    }

    fn initialize(&mut self, params: &Value) -> Value {
        // A diagram names its model file from the working folder, as `ortho render`
        // run there does, so the diagrams match those rendered at the project's root.
        let root = params["workspaceFolders"][0]["uri"].as_str().or(params["rootUri"].as_str());
        if let Some(root) = root.and_then(uri_to_path) {
            let _ = std::env::set_current_dir(root);
        }
        let options = &params["initializationOptions"];
        if let Some(theme) = options["theme"].as_str().and_then(Theme::named) {
            self.theme = theme;
        }
        self.opens_preview = options["preview"].as_bool().unwrap_or(true);
        let shows = params["capabilities"]["window"]["showDocument"]["support"].as_bool().unwrap_or(false);
        self.editor.shows_documents.store(shows, Ordering::Relaxed);
        let save_to = options["diagrams"].as_str().map(str::trim).filter(|folder| !folder.is_empty());
        let root = std::env::current_dir().unwrap_or_default();
        // Without the page, as when the machine lends it no port, the rest still serves.
        self.preview = Preview::start(&root, save_to.unwrap_or("diagrams"), self.editor.clone()).ok();
        let commands = if self.preview.is_some() { vec![SHOW_IN_PREVIEW] } else { vec![] };
        json!({
            "capabilities": {
                "textDocumentSync": { "openClose": true, "change": 1, "save": { "includeText": false } },
                "definitionProvider": true,
                "codeActionProvider": self.preview.is_some(),
                "executeCommandProvider": { "commands": commands },
            },
            "serverInfo": { "name": "ortho", "version": env!("CARGO_PKG_VERSION") },
        })
    }

    /// The model files of a folder, each with its text: what the editor has open, as
    /// typed, and the rest as saved.
    fn sources(&self, folder: &Path) -> Vec<(PathBuf, String)> {
        let mut files: Vec<PathBuf> = std::fs::read_dir(folder)
            .map(|entries| {
                entries
                    .filter_map(|entry| entry.ok().map(|entry| entry.path()))
                    .filter(|path| path.extension().is_some_and(|ext| ext == "sysml") && path.is_file())
                    .collect()
            })
            .unwrap_or_default();
        for path in self.open.keys() {
            if path.parent() == Some(folder) && !files.contains(path) {
                files.push(path.clone());
            }
        }
        files.sort();
        files
            .into_iter()
            .filter_map(|path| {
                let text = match self.open.get(&path) {
                    Some(text) => text.clone(),
                    None => String::from_utf8_lossy(&std::fs::read(&path).ok()?).into_owned(),
                };
                Some((path, text))
            })
            .collect()
    }

    /// Checks the model a file is part of and reports the problems of each of its
    /// files; when it is opened or saved, draws it in the preview too.
    fn check(&mut self, file: &Path, draw: bool) -> io::Result<()> {
        let Some(folder) = file.parent().map(Path::to_path_buf) else {
            return Ok(());
        };
        let sources = self.sources(&folder);
        let paths: Vec<PathBuf> = sources.iter().map(|(path, _)| path.clone()).collect();
        let texts: Vec<String> = sources.iter().map(|(_, text)| text.clone()).collect();
        let named = sources.into_iter().map(|(path, text)| (path.to_string_lossy().into_owned(), text)).collect();
        let mut problems: HashMap<PathBuf, (Vec<Problem>, String)> = HashMap::new();
        let workspace = match Workspace::load(named) {
            Ok(workspace) => {
                for document in &workspace.documents {
                    let found = document.diagnostics.iter().map(|diagnostic| Problem {
                        severity: diagnostic.severity,
                        offset: diagnostic.offset,
                        message: diagnostic.message.clone(),
                    });
                    problems.insert(PathBuf::from(&document.path), (found.collect(), document.text.clone()));
                }
                Some(workspace)
            }
            Err(failures) => {
                for (index, error) in failures {
                    let problem = Problem { severity: Severity::Error, offset: error.offset, message: error.message };
                    problems.insert(paths[index].clone(), (vec![problem], texts[index].clone()));
                }
                None
            }
        };
        let model = self.models.entry(folder.clone()).or_default();
        model.workspace = workspace;
        model.rendering.clear();
        if draw {
            let found = paths.iter().filter_map(|path| Some((path, problems.get(path)?)));
            let notes = found.flat_map(|(path, (found, text))| found.iter().map(move |problem| (path, text, problem)));
            let (errors, notes) = notes.partition(|(_, _, problem)| problem.severity == Severity::Error);
            self.draw(&folder, errors, notes);
        }
        let model = &self.models[&folder];
        let mut reports = Vec::new();
        for path in &paths {
            let (found, text) = problems.remove(path).unwrap_or_default();
            let rendering = model.rendering.get(path).map(Vec::as_slice).unwrap_or_default();
            let diagnostics: Vec<Value> =
                found.iter().chain(rendering).map(|problem| diagnostic(&text, problem)).collect();
            reports.push((path.clone(), Value::Array(diagnostics)));
        }
        for (path, diagnostics) in reports {
            if self.reported.get(&path) != Some(&diagnostics) {
                self.reported.insert(path.clone(), diagnostics.clone());
                self.editor.notify(
                    "textDocument/publishDiagnostics",
                    json!({ "uri": path_to_uri(&path), "diagnostics": diagnostics }),
                )?;
            }
        }
        Ok(())
    }

    /// Draws a model in the preview: its diagrams if it loads, and otherwise what keeps
    /// it from loading, with the last diagrams left on the page. What rendering warns
    /// of is kept to be reported on the view it is about, too.
    fn draw(&mut self, folder: &Path, errors: Vec<Found>, warnings: Vec<Found>) {
        let (Some(preview), Some(model)) = (&self.preview, self.models.get_mut(folder)) else { return };
        let note = |(path, text, problem): &Found| {
            let line = crate::parser::position(text, problem.offset.min(text.len())).0;
            Note {
                place: Some((relative_to_working_folder(&path.to_string_lossy()), line)),
                message: problem.message.clone(),
            }
        };
        let mut shown = Shown {
            folder: folder.to_path_buf(),
            views: Vec::new(),
            errors: errors.iter().map(note).collect(),
            notes: warnings.iter().map(note).collect(),
        };
        if let Some(workspace) = model.workspace.as_ref().filter(|_| shown.errors.is_empty()) {
            let ast = &workspace.ast;
            let mut found: Vec<(NodeId, String)> = Vec::new();
            match render_views(workspace, self.theme) {
                Ok(rendering) => {
                    for skipped in &rendering.skipped {
                        let heading = view_heading(ast, skipped.view);
                        found.push((skipped.view, format!("ortho does not draw '{heading}': {}", skipped.reason)));
                    }
                    if rendering.rendered.is_empty() && rendering.skipped.is_empty() {
                        let message = "The model declares no view. ortho draws the views a model declares, such as \
                                       view 'Functions' : GeneralView { expose Functions; }.";
                        shown.notes.push(Note { place: None, message: message.to_string() });
                    }
                    for rendered in rendering.rendered {
                        if let Some(warning) = rendered.warning {
                            found.push((rendered.view, warning));
                        }
                        let heading = view_heading(ast, rendered.view).to_string();
                        shown.views.push([rendered.file, heading, rendered.result.summary, rendered.result.svg]);
                    }
                }
                Err(message) => shown.errors.push(Note { place: None, message }),
            }
            for (view, message) in found {
                let root = ast.root_of(view);
                let Some(document) = workspace.documents.iter().find(|document| document.root == root) else {
                    continue;
                };
                let offset = ast[view].offset as usize;
                let line = crate::parser::position(&document.text, offset.min(document.text.len())).0;
                let place = Some((relative_to_working_folder(&document.path), line));
                shown.notes.push(Note { place, message: message.clone() });
                let problem = Problem { severity: Severity::Warning, offset, message };
                model.rendering.entry(PathBuf::from(&document.path)).or_default().push(problem);
            }
        }
        preview.show(shown);
    }

    /// Where the name at a position is declared.
    fn definition(&self, params: &Value) -> Value {
        let Some((workspace, document, offset)) = self.at(params) else { return Value::Null };
        let ast = &workspace.ast;
        let text = &document.text;
        // A chain such as `beverage.prepareCoffee.grindBeans` may be one reference or
        // one for each segment; the reference starting last at the position is the
        // segment the position is on.
        let mut found: Option<(usize, NodeId)> = None;
        for node in ast.with_all_contents(document.root) {
            for field in &ast[node].fields {
                let Field::Reference { target: Some(target), .. } = field.value else { continue };
                let start = field.offset as usize;
                if (start..=reference_end(text, start)).contains(&offset) && found.is_none_or(|(at, _)| start > at) {
                    found = Some((start, target));
                }
            }
        }
        let Some((_, target)) = found else { return Value::Null };
        let Some((file, at)) = declaration(workspace, target) else { return Value::Null };
        let file_text = &workspace.documents.iter().find(|d| d.path == file).unwrap().text;
        let range =
            json!({ "start": position_of(file_text, at), "end": position_of(file_text, name_end(file_text, at)) });
        json!({ "uri": path_to_uri(Path::new(file)), "range": range })
    }

    /// A code action on a view's declaration that shows the view in the preview.
    fn code_actions(&self, params: &Value) -> Value {
        let Some((workspace, document, offset)) = self.at(params) else { return json!([]) };
        let ast = &workspace.ast;
        let text = &document.text;
        let line = |offset: usize| crate::parser::position(text, offset.min(text.len())).0;
        let mut actions = Vec::new();
        for view in declared_views(ast, document.root) {
            // A view's declaration runs from its keyword to the last thing it holds.
            let start = ast[view].offset as usize;
            let last = ast.with_all_contents(view).iter().flat_map(|&n| ast[n].fields.iter()).map(|f| f.offset).max();
            let end = last.map_or(start, |last| last as usize);
            if (line(start)..=line(end)).contains(&line(offset)) {
                let title = format!("Show '{}' in the preview", view_heading(ast, view));
                let arguments = json!([view_file_name(ast, view), params["textDocument"]["uri"]]);
                let command = json!({ "title": title, "command": SHOW_IN_PREVIEW, "arguments": arguments });
                actions.push(json!({ "title": title, "command": command }));
            }
        }
        Value::Array(actions)
    }

    /// Runs the command of a code action: draws the view's model in the preview, and
    /// turns the page to the view.
    fn execute(&mut self, params: &Value) -> io::Result<Value> {
        let arguments = &params["arguments"];
        if params["command"].as_str() == Some(SHOW_IN_PREVIEW)
            && let (Some(view), Some(file)) = (arguments[0].as_str(), arguments[1].as_str().and_then(uri_to_path))
        {
            self.check(&file, true)?;
            if let Some(preview) = &self.preview {
                preview.open_at(view)?;
            }
        }
        Ok(Value::Null)
    }

    /// The checked model, document and byte offset a request's position is in.
    fn at(&self, params: &Value) -> Option<(&Workspace, &crate::parser::workspace::Document, usize)> {
        let path = document_path(params)?;
        let workspace = self.models.get(path.parent()?)?.workspace.as_ref()?;
        // Compared as paths, since Windows writes a folder with `/` and a file in it with `\`.
        let document = workspace.documents.iter().find(|document| Path::new(&document.path) == path)?;
        let position = if params["position"].is_object() { &params["position"] } else { &params["range"]["start"] };
        let line = position["line"].as_u64()? as usize;
        let character = position["character"].as_u64()? as usize;
        Some((workspace, document, offset_at(&document.text, line, character)))
    }
}

/// The file a request or notification is about.
fn document_path(params: &Value) -> Option<PathBuf> {
    uri_to_path(params["textDocument"]["uri"].as_str()?)
}

fn diagnostic(text: &str, problem: &Problem) -> Value {
    let severity = match problem.severity {
        Severity::Error => 1,
        Severity::Warning => 2,
    };
    let start = problem.offset.min(text.len());
    let range = json!({ "start": position_of(text, start), "end": position_of(text, name_end(text, start)) });
    json!({ "range": range, "severity": severity, "source": "ortho", "message": problem.message })
}

/// Where an element is declared: its file, and where its name is, or where its
/// declaration starts if it has none.
fn declaration(workspace: &Workspace, element: NodeId) -> Option<(&str, usize)> {
    let ast = &workspace.ast;
    let root = ast.root_of(element);
    let document = workspace.documents.iter().find(|document| document.root == root)?;
    let named = ast[element].fields.iter().rev().find(|field| matches!(field.prop, Prop::Name | Prop::ShortName));
    let offset = named.map_or(ast[element].offset, |field| field.offset);
    Some((&document.path, offset as usize))
}

fn is_name_char(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_'
}

/// The end of the name at `start`, plain or quoted, or of the one character there if
/// it is not a name.
fn name_end(text: &str, start: usize) -> usize {
    let bytes = text.as_bytes();
    let mut i = start;
    if bytes.get(i) == Some(&b'\'') {
        i += 1;
        while i < bytes.len() && bytes[i] != b'\'' && bytes[i] != b'\n' {
            i += if bytes[i] == b'\\' { 2 } else { 1 };
        }
        return (i + 1).min(bytes.len());
    }
    while i < bytes.len() && is_name_char(bytes[i]) {
        i += 1;
    }
    if i == start && i < bytes.len() && bytes[i] != b'\n' && bytes[i] != b'\r' {
        i += text[i..].chars().next().map_or(1, char::len_utf8);
    }
    i
}

/// The end of the reference written at `start`: names joined by `::` and `.`, as
/// `Pkg::Engine` or `engine.speed`, after a `~` if it is conjugated.
fn reference_end(text: &str, start: usize) -> usize {
    let bytes = text.as_bytes();
    let starts_name = |at: usize| bytes.get(at).is_some_and(|&b| is_name_char(b) || b == b'\'');
    let mut i = start;
    if bytes.get(i) == Some(&b'~') {
        i += 1;
    }
    loop {
        i = name_end(text, i);
        if text[i..].starts_with("::") && starts_name(i + 2) {
            i += 2;
        } else if bytes.get(i) == Some(&b'.') && starts_name(i + 1) {
            i += 1;
        } else {
            return i;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reference_extent() {
        let text = "perform Pkg::'my action'.step; x";
        assert_eq!(&text[8..reference_end(text, 8)], "Pkg::'my action'.step");
        assert_eq!(&text[0..name_end(text, 0)], "perform");
        assert_eq!(name_end(text, 29), 30);
        let conjugated = "port p : ~Fuel;";
        assert_eq!(&conjugated[9..reference_end(conjugated, 9)], "~Fuel");
    }
}
