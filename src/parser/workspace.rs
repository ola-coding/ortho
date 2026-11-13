//! A model's files read as one, so names resolve across them.
use super::ast::{Ast, NodeId, Prop, Type};
use super::scope::Linker;
use super::validator::{Gaps, validate};
use super::{SyntaxError, parse, position};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Severity {
    Error = 1,
    Warning = 2,
}

#[derive(Debug)]
pub struct Diagnostic {
    pub severity: Severity,
    /// Where, in bytes.
    pub offset: usize,
    pub message: String,
}

#[derive(Debug)]
pub struct Document {
    pub path: String,
    pub text: String,
    pub root: NodeId,
    pub diagnostics: Vec<Diagnostic>,
}

#[derive(Debug)]
pub struct Workspace {
    pub ast: Ast,
    pub documents: Vec<Document>,
}

/// `file line n: message`
fn report(path: &str, text: &str, offset: usize, message: &str) -> String {
    format!("{path} line {}: {message}", position(text, offset).0)
}

impl Workspace {
    /// Parses, links and validates files given as path and text. If any does not
    /// parse, says which, and goes no further.
    pub fn build(sources: Vec<(String, String)>) -> Result<Workspace, Vec<String>> {
        let texts: Vec<(String, String)> = sources.clone();
        Workspace::load(sources).map_err(|failures| {
            let report_one = |(index, error): (usize, SyntaxError)| {
                let (path, text) = &texts[index];
                report(path, text, error.offset, &error.message)
            };
            failures.into_iter().map(report_one).collect()
        })
    }

    /// As `build`, but a file that does not parse is given by its place among the
    /// sources and its error, as an editor marks it.
    pub fn load(sources: Vec<(String, String)>) -> Result<Workspace, Vec<(usize, SyntaxError)>> {
        let mut ast = Ast::default();
        let mut documents = Vec::with_capacity(sources.len());
        let mut failures = Vec::new();
        for (index, (path, text)) in sources.into_iter().enumerate() {
            match parse(&mut ast, &text) {
                Ok(root) => documents.push(Document { path, text, root, diagnostics: Vec::new() }),
                Err(error) => failures.push((index, error)),
            }
        }
        if !failures.is_empty() {
            return Err(failures);
        }
        let roots: Vec<NodeId> = documents.iter().map(|document| document.root).collect();
        let linked = Linker::new(&ast, &roots).link();
        for (reference, target) in &linked.targets {
            ast.set_target(*reference, *target);
        }
        let gaps = Gaps::of(&ast, &roots);
        for (document, log) in documents.iter_mut().zip(&linked.logs) {
            document.diagnostics = validate(&ast, document.root, &document.text, log, &gaps);
        }
        Ok(Workspace { ast, documents })
    }

    /// What the files report of `severity`, each as `file line n: message`.
    pub fn reported(&self, severity: Severity) -> Vec<String> {
        let mut reports = Vec::new();
        for document in &self.documents {
            for diagnostic in document.diagnostics.iter().filter(|d| d.severity == severity) {
                reports.push(report(&document.path, &document.text, diagnostic.offset, &diagnostic.message));
            }
        }
        reports
    }

    /// The packages of every file, in file order.
    pub fn packages(&self) -> Vec<NodeId> {
        self.documents.iter().flat_map(|document| self.ast.children(document.root, Prop::Packages)).collect()
    }

    /// The file that declares an element, and the line its declaration starts on.
    pub fn declared_at(&self, id: NodeId) -> Option<(&str, usize)> {
        let mut root = id;
        while let Some(parent) = self.ast[root].parent {
            root = parent;
        }
        let document = self.documents.iter().find(|document| document.root == root)?;
        Some((&document.path, position(&document.text, self.ast[id].offset as usize).0))
    }

    pub fn is_model(&self, id: NodeId) -> bool {
        self.ast[id].ty == Type::Model
    }
}

/// A model that loads, and what it warns of.
#[derive(Debug)]
pub struct ParsedModel {
    pub workspace: Workspace,
    /// `file line n: message`. None of it stops a model rendering.
    pub warnings: Vec<String>,
}

/// Parses and links files as a single workspace, so names resolve across them.
pub fn parse_sysml_files(paths: &[String]) -> Result<ParsedModel, String> {
    let mut sources = Vec::with_capacity(paths.len());
    for path in paths {
        let bytes = std::fs::read(path).map_err(|error| format!("{path}: {error}"))?;
        sources.push((path.clone(), String::from_utf8_lossy(&bytes).into_owned()));
    }
    let failed = |failures: Vec<String>| {
        let lines: Vec<String> = failures.iter().map(|failure| format!("  {failure}")).collect();
        format!("Failed to parse:\n{}", lines.join("\n"))
    };
    let workspace = Workspace::build(sources).map_err(failed)?;
    let errors = workspace.reported(Severity::Error);
    if !errors.is_empty() {
        return Err(failed(errors));
    }
    let warnings = workspace.reported(Severity::Warning);
    Ok(ParsedModel { workspace, warnings })
}
