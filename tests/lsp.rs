//! `ortho lsp`, driven over stdio as an editor drives it.
use serde_json::{Value, json};
use std::io::{BufRead, BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

mod common;
use common::repository;

/// An empty folder of the test's own.
fn scratch(name: &str) -> PathBuf {
    let folder = Path::new(env!("CARGO_TARGET_TMPDIR")).join("lsp").join(name);
    let _ = std::fs::remove_dir_all(&folder);
    std::fs::create_dir_all(&folder).unwrap();
    folder
}

/// `file:///home/a/m.sysml`, or `file:///C:/a/m.sysml` on Windows.
fn uri(path: &Path) -> String {
    let path = path.to_str().unwrap().replace('\\', "/").replace(' ', "%20");
    if path.starts_with('/') { format!("file://{path}") } else { format!("file:///{path}") }
}

fn frame(message: &Value) -> String {
    let body = message.to_string();
    format!("Content-Length: {}\r\n\r\n{body}", body.len())
}

/// Starts a session in `root`, sends `messages` after `initialize`, shuts down, and
/// returns everything the server sent, with how it exited.
fn session(root: &Path, messages: &[Value]) -> (Vec<Value>, bool) {
    let mut child = Command::new(env!("CARGO_BIN_EXE_ortho"))
        .arg("lsp")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    let mut input = String::new();
    let initialize = json!({ "jsonrpc": "2.0", "id": 0, "method": "initialize",
        "params": { "rootUri": uri(root), "capabilities": {} } });
    input.push_str(&frame(&initialize));
    input.push_str(&frame(&json!({ "jsonrpc": "2.0", "method": "initialized", "params": {} })));
    for message in messages {
        input.push_str(&frame(message));
    }
    input.push_str(&frame(&json!({ "jsonrpc": "2.0", "id": 999, "method": "shutdown" })));
    input.push_str(&frame(&json!({ "jsonrpc": "2.0", "method": "exit" })));
    child.stdin.take().unwrap().write_all(input.as_bytes()).unwrap();
    let mut output = BufReader::new(child.stdout.take().unwrap());
    let mut sent = Vec::new();
    loop {
        let mut header = String::new();
        if output.read_line(&mut header).unwrap() == 0 {
            break;
        }
        let length: usize = header.trim().strip_prefix("Content-Length: ").unwrap().parse().unwrap();
        output.read_line(&mut String::new()).unwrap();
        let mut body = vec![0; length];
        output.read_exact(&mut body).unwrap();
        sent.push(serde_json::from_slice(&body).unwrap());
    }
    (sent, child.wait().unwrap().success())
}

fn open(path: &Path, text: &str) -> Value {
    json!({ "jsonrpc": "2.0", "method": "textDocument/didOpen", "params": {
        "textDocument": { "uri": uri(path), "languageId": "sysml", "version": 1, "text": text } } })
}

fn change(path: &Path, text: &str) -> Value {
    json!({ "jsonrpc": "2.0", "method": "textDocument/didChange", "params": {
        "textDocument": { "uri": uri(path), "version": 2 }, "contentChanges": [{ "text": text }] } })
}

fn save(path: &Path) -> Value {
    json!({ "jsonrpc": "2.0", "method": "textDocument/didSave", "params": { "textDocument": { "uri": uri(path) } } })
}

fn request(id: u64, method: &str, params: Value) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "method": method, "params": params })
}

fn at(path: &Path, line: u64, character: u64) -> Value {
    json!({ "textDocument": { "uri": uri(path) }, "position": { "line": line, "character": character } })
}

fn response(sent: &[Value], id: u64) -> &Value {
    &sent.iter().find(|message| message["id"] == id && message.get("method").is_none()).unwrap()["result"]
}

/// What the server last reported for a file.
fn diagnostics(sent: &[Value], path: &Path) -> Vec<(u64, u64, String)> {
    let mut reports = sent.iter().filter(|message| {
        message["method"] == "textDocument/publishDiagnostics" && message["params"]["uri"] == uri(path)
    });
    let last = reports.next_back().unwrap_or_else(|| panic!("nothing reported for {}", path.display()));
    let found = last["params"]["diagnostics"].as_array().unwrap().iter();
    found
        .map(|d| {
            let line = d["range"]["start"]["line"].as_u64().unwrap();
            (d["severity"].as_u64().unwrap(), line, d["message"].as_str().unwrap().to_string())
        })
        .collect()
}

/// A copy of an example for a test of its own, to write into.
fn example(name: &str, test: &str) -> PathBuf {
    let folder = scratch(&format!("{test}/{name}"));
    let source = repository().join("examples").join(name);
    for entry in std::fs::read_dir(&source).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().is_some_and(|ext| ext == "sysml") {
            std::fs::copy(&path, folder.join(path.file_name().unwrap())).unwrap();
        }
    }
    folder
}

#[test]
fn starts_and_shuts_down() {
    let folder = scratch("starts");
    let (sent, clean) = session(&folder, &[]);
    assert!(clean);
    let capabilities = &response(&sent, 0)["capabilities"];
    assert_eq!(capabilities["definitionProvider"], true);
    assert_eq!(response(&sent, 0)["serverInfo"]["name"], "ortho");
}

#[test]
fn reports_the_problems_of_every_file_in_the_folder_as_typed() {
    let folder = example("coffee-machine", "problems");
    let logical = folder.join("logical.sysml");
    let text = std::fs::read_to_string(&logical).unwrap();
    // Renamed as typed, unsaved: the files that perform it no longer resolve.
    let renamed = text.replace("action grindBeans;", "action millBeans;");
    let (sent, _) = session(&folder, &[open(&logical, &text), change(&logical, &renamed)]);
    let physical = diagnostics(&sent, &folder.join("physical.sysml"));
    assert!(
        physical.iter().any(|(severity, _, message)| *severity == 1 && message.contains("grindBeans")),
        "{physical:?}"
    );
    assert_eq!(diagnostics(&sent, &folder.join("use-case.sysml")), vec![]);
}

#[test]
fn reports_a_syntax_error_where_it_is() {
    let folder = scratch("syntax");
    let model = folder.join("model.sysml");
    let (sent, _) = session(&folder, &[open(&model, "package P {\n    part def X Y;\n}\n")]);
    let found = diagnostics(&sent, &model);
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!((found[0].0, found[0].1), (1, 1));
}

#[test]
fn goes_to_what_a_name_names_in_another_file() {
    let folder = example("coffee-machine", "definition");
    let physical = folder.join("physical.sysml");
    let text = std::fs::read_to_string(&physical).unwrap();
    let (line, column) = text
        .lines()
        .enumerate()
        .find_map(|(i, line)| line.find("grindBeans").filter(|_| line.contains("perform")).map(|c| (i, c)))
        .unwrap();
    let (sent, _) = session(
        &folder,
        &[open(&physical, &text), request(1, "textDocument/definition", at(&physical, line as u64, column as u64 + 2))],
    );
    let location = response(&sent, 1);
    assert_eq!(location["uri"], uri(&folder.join("logical.sysml")));
    let logical = std::fs::read_to_string(folder.join("logical.sysml")).unwrap();
    let declared = logical.lines().position(|line| line.trim() == "action grindBeans;").unwrap();
    assert_eq!(location["range"]["start"]["line"], declared as u64);
    let start = location["range"]["start"]["character"].as_u64().unwrap() as usize;
    assert_eq!(&logical.lines().nth(declared).unwrap()[start..start + 10], "grindBeans");
}

#[test]
fn writes_the_diagrams_on_save_as_ortho_render_does() {
    let folder = example("coffee-machine", "save");
    let logical = folder.join("logical.sysml");
    let text = std::fs::read_to_string(&logical).unwrap();
    let (_, clean) = session(&folder, &[open(&logical, &text), save(&logical)]);
    assert!(clean);
    // As `ortho render` run at the project's root, where the server runs.
    let rendered = Command::new(env!("CARGO_BIN_EXE_ortho"))
        .args(["render", ".", "-o", "expected"])
        .current_dir(&folder)
        .output()
        .unwrap();
    assert!(rendered.status.success());
    let mut names = 0;
    for entry in std::fs::read_dir(folder.join("expected")).unwrap() {
        let name = entry.unwrap().file_name();
        let written = std::fs::read_to_string(folder.join("diagrams").join(&name)).unwrap();
        assert_eq!(written, std::fs::read_to_string(folder.join("expected").join(&name)).unwrap(), "{name:?}");
        names += 1;
    }
    assert_eq!(names, 6);
}

#[test]
fn opens_a_views_diagram_from_a_code_action() {
    let folder = example("coffee-machine", "action");
    let logical = folder.join("logical.sysml");
    let text = std::fs::read_to_string(&logical).unwrap();
    let line = text.lines().position(|line| line.trim_start().starts_with("view ")).unwrap() as u64;
    let range = json!({ "start": { "line": line, "character": 0 }, "end": { "line": line, "character": 0 } });
    let params = json!({ "textDocument": { "uri": uri(&logical) }, "range": range, "context": { "diagnostics": [] } });
    let (sent, _) = session(&folder, &[open(&logical, &text), request(1, "textDocument/codeAction", params.clone())]);
    let actions = response(&sent, 1).as_array().unwrap();
    assert_eq!(actions.len(), 1, "{actions:?}");
    let command = actions[0]["command"].clone();
    let execute = json!({ "command": command["command"], "arguments": command["arguments"] });
    let (sent, _) = session(&folder, &[open(&logical, &text), request(2, "workspace/executeCommand", execute)]);
    let show = sent.iter().find(|message| message["method"] == "window/showDocument").unwrap();
    let svg = folder.join("diagrams/logical-view-coffee-machine.svg");
    assert_eq!(show["params"]["uri"], uri(&svg));
    assert!(svg.exists());
    // Elsewhere in the file, no action.
    let mut elsewhere = params;
    elsewhere["range"]["start"]["line"] = json!(1);
    elsewhere["range"]["end"]["line"] = json!(1);
    let (sent, _) = session(&folder, &[open(&logical, &text), request(3, "textDocument/codeAction", elsewhere)]);
    assert_eq!(response(&sent, 3), &json!([]));
}

fn first_line_actions(file: &Path, text: &str, root: &Path) -> Vec<Value> {
    let range = json!({ "start": { "line": 0, "character": 0 }, "end": { "line": 0, "character": 0 } });
    let params = json!({ "textDocument": { "uri": uri(file) }, "range": range, "context": { "diagnostics": [] } });
    let (sent, _) = session(root, &[open(file, text), request(1, "textDocument/codeAction", params)]);
    response(&sent, 1).as_array().unwrap().clone()
}

fn run(root: &Path, file: &Path, text: &str, action: &Value) -> Vec<Value> {
    let command = &action["command"];
    let execute = json!({ "command": command["command"], "arguments": command["arguments"] });
    session(root, &[open(file, text), request(2, "workspace/executeCommand", execute)]).0
}

fn titled<'a>(actions: &'a [Value], start: &str) -> Option<&'a Value> {
    actions.iter().find(|action| action["title"].as_str().unwrap().starts_with(start))
}

#[test]
fn adds_the_ai_instructions_from_an_empty_file() {
    let root = scratch("instructions");
    let file = root.join("new.sysml");
    let actions = first_line_actions(&file, "", &root);
    let add = titled(&actions, "Add AI instructions").unwrap_or_else(|| panic!("{actions:?}"));
    let sent = run(&root, &file, "", add);
    let skill = root.join(".claude/skills/sysml-v2-ortho/SKILL.md");
    assert!(std::fs::read_to_string(&skill).unwrap().contains("ortho render <model folder>"));
    assert!(root.join(".claude/skills/sysml-v2-ortho/LANGUAGE.md").exists());
    assert!(root.join(".github/instructions/sysml-v2.instructions.md").exists());
    let show = sent.iter().find(|message| message["method"] == "window/showDocument").unwrap();
    assert_eq!(show["params"]["uri"], uri(&skill));
    // Once there, as ortho writes them, they are not offered again; changed, they are
    // offered to be replaced.
    assert!(titled(&first_line_actions(&file, "", &root), "Add AI instructions").is_none());
    assert!(titled(&first_line_actions(&file, "", &root), "Replace the AI instructions").is_none());
    std::fs::write(&skill, "mine").unwrap();
    assert!(titled(&first_line_actions(&file, "", &root), "Replace the AI instructions").is_some());
}

#[test]
fn copies_an_example_into_a_free_folder_and_opens_it() {
    let root = scratch("examples");
    std::fs::create_dir(root.join("coffee-machine")).unwrap();
    let file = root.join("new.sysml");
    let actions = first_line_actions(&file, "", &root);
    let copy = titled(&actions, "Copy the coffee machine example into coffee-machine-2/")
        .unwrap_or_else(|| panic!("{actions:?}"));
    assert!(titled(&actions, "Copy the survey drone example into survey-drone/").is_some());
    let sent = run(&root, &file, "", copy);
    let copied = root.join("coffee-machine-2");
    for entry in std::fs::read_dir(repository().join("examples/coffee-machine")).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().is_some_and(|ext| ext == "sysml") {
            let name = path.file_name().unwrap();
            assert_eq!(std::fs::read(copied.join(name)).unwrap(), std::fs::read(&path).unwrap(), "{name:?}");
        }
    }
    let show = sent.iter().find(|message| message["method"] == "window/showDocument").unwrap();
    assert_eq!(show["params"]["uri"], uri(&copied.join("use-case.sysml")));
}
