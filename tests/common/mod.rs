//! What the tests of the views share: models to load, and ways to look at what they draw.
// Each test file uses some of these, and is compiled on its own.
#![allow(dead_code)]
use ortho::model::graph::{Compartment, GraphNode};
use ortho::model::views::{declared_views, exposed_elements};
use ortho::parser::ast::NodeId;
use ortho::parser::workspace::{Severity, Workspace, parse_sysml_files};
use ortho::render::svg::DiagramTitle;
use std::path::PathBuf;

/// The repository, which holds `examples/` and `tests/fixtures/`.
pub fn repository() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// A model of one file that links with no error.
pub fn parse_ok(text: &str) -> Workspace {
    let workspace = Workspace::build(vec![("model.sysml".to_string(), text.to_string())])
        .unwrap_or_else(|failures| panic!("{failures:?}"));
    assert_eq!(workspace.reported(Severity::Error), Vec::<String>::new());
    workspace
}

/// The errors a model of one file reports.
pub fn errors_of(text: &str) -> Vec<String> {
    let workspace = Workspace::build(vec![("model.sysml".to_string(), text.to_string())])
        .unwrap_or_else(|failures| panic!("{failures:?}"));
    workspace.reported(Severity::Error)
}

/// One of the models in `tests/fixtures/`, which links with no error.
pub fn fixture(name: &str) -> Workspace {
    let path = repository().join("tests/fixtures").join(name);
    parse_ok(&std::fs::read_to_string(path).unwrap())
}

/// Files of an example, read as one model, the first file first.
pub fn example(example: &str, files: &[&str]) -> Workspace {
    let folder = repository().join("examples").join(example);
    let paths: Vec<String> = files.iter().map(|file| folder.join(file).to_string_lossy().into_owned()).collect();
    parse_sysml_files(&paths).unwrap().workspace
}

/// The packages of every file: what a view of the whole model exposes.
pub fn everything(workspace: &Workspace) -> Vec<NodeId> {
    workspace.packages()
}

/// The views the model's first file declares.
pub fn views(workspace: &Workspace) -> Vec<NodeId> {
    declared_views(&workspace.ast, workspace.documents[0].root)
}

/// What the first view of the model's first file exposes.
pub fn exposed_by_view(workspace: &Workspace) -> Vec<NodeId> {
    exposed_elements(&workspace.ast, views(workspace)[0])
}

/// Every box, nested ones included, each before what it holds.
pub fn all_nodes(nodes: &[GraphNode]) -> Vec<&GraphNode> {
    let mut all = Vec::new();
    for node in nodes {
        all.push(node);
        all.extend(all_nodes(node.children.as_deref().unwrap_or_default()));
    }
    all
}

/// The names of boxes, in order.
pub fn names<'a>(nodes: impl IntoIterator<Item = &'a GraphNode>) -> Vec<&'a str> {
    nodes.into_iter().map(|node| node.name.as_str()).collect()
}

/// The names of boxes, sorted.
pub fn sorted_names<'a>(nodes: impl IntoIterator<Item = &'a GraphNode>) -> Vec<&'a str> {
    let mut names = names(nodes);
    names.sort();
    names
}

pub fn compartment(title: &str, lines: &[&str]) -> Compartment {
    Compartment { title: Some(title.to_string()), lines: lines.iter().map(|line| line.to_string()).collect() }
}

/// The lines of a box's compartment of `title`, none if it has none.
pub fn lines_of<'a>(node: &'a GraphNode, title: &str) -> Vec<&'a str> {
    node.compartments
        .iter()
        .find(|c| c.title.as_deref() == Some(title))
        .map(|c| c.lines.iter().map(String::as_str).collect())
        .unwrap_or_default()
}

pub fn title(heading: &str, source: &str) -> DiagramTitle {
    DiagramTitle { heading: heading.to_string(), definition: None, source: source.to_string() }
}

/// The middle of a box, across.
pub fn centre(x: f64, width: f64) -> f64 {
    x + width / 2.0
}

/// Whether two numbers are the same to `digits` decimal places.
pub fn close_to(actual: f64, expected: f64, digits: i32) -> bool {
    (actual - expected).abs() < 10f64.powi(-digits) / 2.0
}

/// How often `pattern` occurs in `text`.
pub fn count(text: &str, pattern: &str) -> usize {
    text.matches(pattern).count()
}
