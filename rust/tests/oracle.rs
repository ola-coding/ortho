//! The port against what the TypeScript produces, which `npm run oracle` records in
//! `rust/oracle/`. Ignored until asked for: `cargo test --test oracle -- --ignored`.
use ortho::diagrams::logical::extract_logical_graph;
use ortho::diagrams::sequence::{SequenceLifeline, SequenceMessage, SequenceModel, extract_sequence_model};
use ortho::diagrams::use_case::extract_use_case_graph;
use ortho::layout::tree::layout_tree;
use ortho::layout::use_case::layout_use_cases;
use ortho::layout::{LabelBox, LaidOutDiagram, LaidOutEdge, LaidOutNode, LaidOutPort, Point};
use ortho::model::graph::{Compartment, DiagramGraph, EdgeKind, GraphNode, NodeShape};
use ortho::model::views::{
    DiagramKind, StandardView, declared_views, diagram_kind, diagram_kind_of, exposed_elements, view_file_name,
    why_undrawable,
};
use ortho::parser::ast::{Ast, Field, NodeId, Prop, Type, Value};
use ortho::parser::workspace::Severity;
use ortho::parser::workspace::Workspace;
use ortho::parser::{parse, position};
use ortho::pipeline::{render_exposed, render_view};
use ortho::render::number::Js;
use ortho::render::sequence::render_sequence_svg;
use ortho::render::svg::{DiagramTitle, render_svg};
use ortho::render::theme::{DARK_GLASS, LIGHT};
use std::fmt::Write;
use std::path::{Path, PathBuf};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap().to_path_buf()
}

fn write_string(text: &str, out: &mut String) {
    out.push('"');
    for c in text.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\u{8}' => out.push_str("\\b"),
            '\u{c}' => out.push_str("\\f"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => write!(out, "\\u{:04x}", c as u32).unwrap(),
            c => out.push(c),
        }
    }
    out.push('"');
}

/// An offset as JavaScript counts it, in UTF-16 units.
fn utf16(text: &str, offset: u32) -> usize {
    if text.is_ascii() { offset as usize } else { text[..offset as usize].encode_utf16().count() }
}

/// A node as JSON: its offset, its type, and its properties by name. A binary
/// expression's offset is left out: Langium gives all of one expression's the same.
fn write_node(ast: &Ast, id: NodeId, text: &str, out: &mut String) {
    let node = &ast[id];
    out.push('{');
    if node.ty != Type::OperatorExpression {
        write!(out, "\"$offset\":{},", utf16(text, node.offset)).unwrap();
    }
    write!(out, "\"$type\":\"{:?}\"", node.ty).unwrap();
    let mut names: Vec<(String, Prop)> = Vec::new();
    for field in &node.fields {
        if !names.iter().any(|(_, prop)| *prop == field.prop) {
            names.push((field.prop.name(), field.prop));
        }
    }
    names.sort_by(|a, b| a.0.cmp(&b.0));
    for (name, prop) in names {
        write!(out, ",\"{name}\":").unwrap();
        let of_prop = || node.fields.iter().filter(|field| field.prop == prop);
        if of_prop().next().unwrap().list {
            out.push('[');
            for (i, field) in of_prop().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                write_value(ast, field, text, out);
            }
            out.push(']');
        } else {
            // Assigned again, the last one stands.
            write_value(ast, of_prop().next_back().unwrap(), text, out);
        }
    }
    out.push('}');
}

fn write_value(ast: &Ast, field: &Field, text: &str, out: &mut String) {
    match &field.value {
        Value::Text(value) => write_string(value, out),
        Value::Flag => out.push_str("true"),
        Value::Reference { text: value, .. } => {
            write!(out, "{{\"$offset\":{},\"$ref\":", utf16(text, field.offset)).unwrap();
            write_string(value, out);
            out.push('}');
        }
        Value::Child(child) => write_node(ast, *child, text, out),
    }
}

/// Every file in a recorded set parses to the tree Langium built, or fails as it did.
fn check_trees(set: &str) {
    let root = root();
    let path = root.join(format!("rust/oracle/trees/{set}.tsv"));
    let recorded =
        std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}; run npm run oracle", path.display()));
    let mut compared = 0;
    let mut different = Vec::new();
    for line in recorded.lines() {
        let (file, expected) = line.split_once('\t').unwrap();
        let text = String::from_utf8_lossy(&std::fs::read(root.join(file)).unwrap()).into_owned();
        let mut ast = Ast::default();
        let mut actual = String::new();
        match parse(&mut ast, &text) {
            Ok(model) => write_node(&ast, model, &text, &mut actual),
            Err(_) => actual.push_str("{\"$rejected\":true}"),
        }
        compared += 1;
        if actual != expected {
            let at = actual.bytes().zip(expected.bytes()).take_while(|(a, b)| a == b).count();
            let around = |s: &str| s.get(at.saturating_sub(80)..(at + 80).min(s.len())).unwrap_or("").to_string();
            different.push(format!("{file}\n  rust   ...{}\n  oracle ...{}", around(&actual), around(expected)));
        }
    }
    assert!(compared > 0, "{set}: nothing recorded");
    assert!(
        different.is_empty(),
        "{set}: {} of {compared} differ\n{}",
        different.len(),
        different.iter().take(8).cloned().collect::<Vec<_>>().join("\n")
    );
    println!("{set}: {compared} files, identical trees");
}

/// One reference and one diagnostic, as the recording's JSON writes them.
fn link_lines(workspace: &Workspace) -> (Vec<String>, Vec<String>) {
    let ast = &workspace.ast;
    let file_of = |node: NodeId| {
        let root = ast.root_of(node);
        workspace.documents.iter().find(|document| document.root == root).unwrap().path.as_str()
    };
    let mut links = Vec::new();
    let mut diagnostics = Vec::new();
    for document in &workspace.documents {
        for node in ast.with_all_contents(document.root) {
            for (reference, index) in ast.references(node) {
                let mut line = String::from("{\"file\":");
                write_string(&document.path, &mut line);
                line.push_str(",\"node\":");
                write_string(&ast.node_path(node), &mut line);
                line.push_str(",\"property\":");
                write_string(&ast[node].fields[reference.field as usize].prop.name(), &mut line);
                if let Some(index) = index {
                    write!(line, ",\"index\":{index}").unwrap();
                }
                line.push_str(",\"text\":");
                write_string(ast.reference_text(reference), &mut line);
                line.push_str(",\"target\":");
                match ast.reference_target(reference) {
                    Some(target) => write_string(&format!("{}#{}", file_of(target), ast.node_path(target)), &mut line),
                    None => line.push_str("null"),
                }
                line.push('}');
                links.push(line);
            }
        }
        for diagnostic in &document.diagnostics {
            let (at, character) = position(&document.text, diagnostic.offset);
            let mut line = String::from("{\"file\":");
            write_string(&document.path, &mut line);
            write!(
                line,
                ",\"severity\":{},\"line\":{at},\"character\":{character},\"message\":",
                diagnostic.severity as u8
            )
            .unwrap();
            write_string(&diagnostic.message, &mut line);
            line.push('}');
            diagnostics.push(line);
        }
    }
    (links, diagnostics)
}

fn load(files: &[String]) -> Workspace {
    let root = root();
    let sources = files.iter().map(|file| {
        let text = String::from_utf8_lossy(&std::fs::read(root.join(file)).unwrap()).into_owned();
        (file.clone(), text)
    });
    Workspace::build(sources.collect()).unwrap_or_else(|failures| panic!("{failures:?}"))
}

fn read_recorded(name: &str) -> String {
    let path = root().join(format!("rust/oracle/{name}"));
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}; run npm run oracle", path.display()))
}

fn first_difference(actual: &[String], expected: &[String]) -> Option<String> {
    let at = actual.iter().zip(expected).position(|(a, e)| a != e);
    match at {
        Some(i) => Some(format!("line {i}\n  rust   {}\n  oracle {}", actual[i], expected[i])),
        None if actual.len() != expected.len() => {
            let (a, e) = (actual.len(), expected.len());
            let extra = actual.get(e).or(expected.get(a)).unwrap();
            Some(format!("rust has {a} lines, the oracle {e}; first extra: {extra}"))
        }
        None => None,
    }
}

/// The files of a recorded workspace.
fn strings_in(json: &str) -> Vec<String> {
    let value: serde_json::Value = serde_json::from_str(json).unwrap();
    value["files"].as_array().unwrap().iter().map(|file| file.as_str().unwrap().to_string()).collect()
}

/// Every workspace recorded on its own, every reference finding what Langium found.
#[test]
#[ignore = "needs npm run oracle"]
fn links_of_whole_workspaces() {
    let mut compared = 0;
    for line in read_recorded("workspaces.jsonl").lines() {
        let record: serde_json::Value = serde_json::from_str(line).unwrap();
        let name = record["name"].as_str().unwrap();
        let workspace = load(&strings_in(line));
        let (links, diagnostics) = link_lines(&workspace);
        let expected_links: Vec<String> =
            read_recorded(&format!("links/{name}.jsonl")).lines().map(String::from).collect();
        let expected_diagnostics: Vec<String> =
            read_recorded(&format!("diagnostics/{name}.jsonl")).lines().map(String::from).collect();
        if let Some(difference) = first_difference(&links, &expected_links) {
            panic!("{name}: {difference}");
        }
        if let Some(difference) = first_difference(&diagnostics, &expected_diagnostics) {
            panic!("{name}: {difference}");
        }
        compared += links.len();
    }
    println!("{compared} references, each finding the same target");
}

/// `{"files":[...],"links":[...],"diagnostics":[...]}`, one workspace per line.
fn check_small_workspaces(set: &str) {
    let recorded = read_recorded(&format!("links/{set}.jsonl"));
    let (mut workspaces, mut references, mut reported) = (0, 0, 0);
    let mut different = Vec::new();
    for line in recorded.lines() {
        let record: serde_json::Value = serde_json::from_str(line).unwrap();
        let files = strings_in(line);
        let workspace = load(&files);
        let (links, diagnostics) = link_lines(&workspace);
        let lines_of = |key: &str| -> Vec<String> {
            record[key].as_array().unwrap().iter().map(serde_json::Value::to_string).collect()
        };
        let (expected_links, expected_diagnostics) = (lines_of("links"), lines_of("diagnostics"));
        let reparse = |lines: &[String]| -> Vec<String> {
            lines.iter().map(|line| serde_json::from_str::<serde_json::Value>(line).unwrap().to_string()).collect()
        };
        workspaces += 1;
        references += links.len();
        reported += diagnostics.len();
        let difference = first_difference(&reparse(&links), &expected_links)
            .or_else(|| first_difference(&reparse(&diagnostics), &expected_diagnostics));
        if let Some(difference) = difference {
            different.push(format!("{}: {difference}", files.join(", ")));
        }
    }
    assert!(workspaces > 0, "{set}: nothing recorded");
    assert!(
        different.is_empty(),
        "{set}: {} of {workspaces} differ\n{}",
        different.len(),
        different.iter().take(6).cloned().collect::<Vec<_>>().join("\n")
    );
    println!("{set}: {workspaces} workspaces, {references} references and {reported} diagnostics, all the same");
}

#[test]
#[ignore = "needs npm run oracle"]
fn links_of_each_file_alone() {
    check_small_workspaces("alone");
}

#[test]
#[ignore = "needs npm run oracle"]
fn links_of_each_construct_alone() {
    check_small_workspaces("constructs-alone");
}

#[test]
#[ignore = "needs npm run oracle"]
fn links_of_workspaces_written_by_hand() {
    check_small_workspaces("workspaces");
}

#[test]
#[ignore = "needs npm run oracle"]
fn links_of_each_broken_model_alone() {
    check_small_workspaces("broken-alone");
}

#[test]
#[ignore = "needs npm run oracle"]
fn links_of_a_broken_file_among_good_ones() {
    check_small_workspaces("broken-in-context");
}

#[test]
#[ignore = "needs npm run oracle"]
fn trees_of_omg_models() {
    check_trees("omg");
}

#[test]
#[ignore = "needs npm run oracle"]
fn trees_of_our_examples_and_fixtures() {
    check_trees("ours");
}

#[test]
#[ignore = "needs npm run oracle"]
fn trees_of_tricky_constructs() {
    check_trees("constructs");
}

#[test]
#[ignore = "needs npm run oracle"]
fn trees_of_broken_models() {
    check_trees("broken");
}

// ---------------------------------------------------------------- Rendering

type Json = serde_json::Value;

fn number(json: &Json, key: &str) -> f64 {
    json[key].as_f64().unwrap_or_else(|| panic!("{key} in {json}"))
}

fn text(json: &Json, key: &str) -> String {
    json[key].as_str().unwrap_or_else(|| panic!("{key} in {json}")).to_string()
}

fn optional_text(json: &Json, key: &str) -> Option<String> {
    json[key].as_str().map(String::from)
}

fn list<T>(json: &Json, key: &str, read: impl Fn(&Json) -> T) -> Vec<T> {
    json[key].as_array().unwrap_or_else(|| panic!("{key} in {json}")).iter().map(read).collect()
}

fn read_node(json: &Json) -> LaidOutNode {
    LaidOutNode {
        id: text(json, "id"),
        shape: match json["shape"].as_str().unwrap() {
            "definition" => NodeShape::Definition,
            "usage" => NodeShape::Usage,
            "actor" => NodeShape::Actor,
            "package" => NodeShape::Package,
            other => panic!("shape {other}"),
        },
        keyword: text(json, "keyword"),
        name: text(json, "name"),
        compartments: list(json, "compartments", |c| Compartment {
            title: optional_text(c, "title"),
            lines: list(c, "lines", |line| line.as_str().unwrap().to_string()),
        }),
        ports: list(json, "ports", |p| LaidOutPort {
            id: text(p, "id"),
            label: text(p, "label"),
            x: number(p, "x"),
            y: number(p, "y"),
            label_at: p["labelX"].as_f64().zip(p["labelY"].as_f64()),
        }),
        has_children: json["hasChildren"].as_bool().unwrap(),
        x: number(json, "x"),
        y: number(json, "y"),
        width: number(json, "width"),
        height: number(json, "height"),
    }
}

fn read_edge(json: &Json) -> LaidOutEdge {
    LaidOutEdge {
        id: text(json, "id"),
        kind: match json["kind"].as_str().unwrap() {
            "connection" => EdgeKind::Connection,
            "association" => EdgeKind::Association,
            "include" => EdgeKind::Include,
            "import" => EdgeKind::Import,
            "decomposition" => EdgeKind::Decomposition,
            "dependency" => EdgeKind::Dependency,
            other => panic!("kind {other}"),
        },
        source_id: text(json, "sourceId"),
        target_id: text(json, "targetId"),
        label: optional_text(json, "label"),
        points: list(json, "points", |p| Point { x: number(p, "x"), y: number(p, "y") }),
        label_box: json.get("labelBox").map(|b| LabelBox {
            x: number(b, "x"),
            y: number(b, "y"),
            width: number(b, "width"),
            height: number(b, "height"),
        }),
    }
}

fn read_diagram(json: &Json) -> LaidOutDiagram {
    LaidOutDiagram {
        width: number(json, "width"),
        height: number(json, "height"),
        nodes: list(json, "nodes", read_node),
        edges: list(json, "edges", read_edge),
    }
}

fn read_sequence(json: &Json) -> SequenceModel {
    SequenceModel {
        lifelines: list(json, "lifelines", |l| SequenceLifeline { id: text(l, "id"), label: text(l, "label") }),
        messages: list(json, "messages", |m| SequenceMessage {
            source_id: text(m, "sourceId"),
            target_id: text(m, "targetId"),
            label: text(m, "label"),
        }),
    }
}

/// Where two SVGs part, with what surrounds it.
fn svg_difference(actual: &str, expected: &str) -> Option<String> {
    if actual == expected {
        return None;
    }
    let at = actual.bytes().zip(expected.bytes()).take_while(|(a, b)| a == b).count();
    let around =
        |s: &str| String::from_utf8_lossy(&s.as_bytes()[at.saturating_sub(100)..(at + 100).min(s.len())]).into_owned();
    Some(format!("at byte {at}\n  rust   {}\n  oracle {}", around(actual), around(expected)))
}

fn check_svgs(name: &str, stem: &str, render: impl Fn(&ortho::render::theme::Theme) -> String) {
    for (theme, extension) in [(&LIGHT, "svg"), (&DARK_GLASS, "dark.svg")] {
        let expected = read_recorded(&format!("{stem}.{extension}"));
        if let Some(difference) = svg_difference(&render(theme), &expected) {
            panic!("{name}, {extension}: {difference}");
        }
    }
}

/// Every view the examples and fixtures declare, drawn from its recorded layout or
/// model, byte for byte what the TypeScript draws, in both themes.
#[test]
#[ignore = "needs npm run oracle"]
fn svgs_of_recorded_views() {
    let mut compared = 0;
    for line in read_recorded("views/index.jsonl").lines() {
        let record: Json = serde_json::from_str(line).unwrap();
        let Some(kind) = record["kind"].as_str() else {
            continue;
        };
        assert!(record.get("error").is_none(), "{line}");
        let id = text(&record, "id");
        let title = DiagramTitle {
            heading: optional_text(&record, "name").unwrap_or_default(),
            definition: optional_text(&record, "definition"),
            source: text(&record, "source"),
        };
        let stem = format!("views/{id}");
        if kind == "process" {
            let model = read_sequence(&serde_json::from_str(&read_recorded(&format!("{stem}.graph.json"))).unwrap());
            check_svgs(&id, &stem, |theme| render_sequence_svg(&model, &title, theme));
        } else {
            let diagram = read_diagram(&serde_json::from_str(&read_recorded(&format!("{stem}.layout.json"))).unwrap());
            check_svgs(&id, &stem, |theme| render_svg(&diagram, &title, theme));
        }
        compared += 1;
    }
    assert!(compared > 0, "no view recorded");
    println!("{compared} views, identical in both themes");
}

/// Diagrams written by hand, for what no model draws.
#[test]
#[ignore = "needs npm run oracle"]
fn svgs_of_diagrams_written_by_hand() {
    let cases: Json = serde_json::from_str(include_str!("renderings.json")).unwrap();
    for (i, case) in cases.as_array().unwrap().iter().enumerate() {
        let title = &case["title"];
        let title = DiagramTitle {
            heading: text(title, "heading"),
            definition: optional_text(title, "definition"),
            source: text(title, "source"),
        };
        let (name, stem) = (format!("case {i}"), format!("renderings/{i}"));
        if let Some(sequence) = case.get("sequence") {
            let model = read_sequence(sequence);
            check_svgs(&name, &stem, |theme| render_sequence_svg(&model, &title, theme));
        } else {
            let diagram = read_diagram(&case["diagram"]);
            check_svgs(&name, &stem, |theme| render_svg(&diagram, &title, theme));
        }
    }
}

// ---------------------------------------------------------------- Views

/// What JSON.stringify writes, field by field in the order the TypeScript builds them.
struct JsonObject<'a> {
    out: &'a mut String,
    empty: bool,
}

impl<'a> JsonObject<'a> {
    fn new(out: &'a mut String) -> JsonObject<'a> {
        out.push('{');
        JsonObject { out, empty: true }
    }

    fn key(&mut self, key: &str) -> &mut String {
        if !self.empty {
            self.out.push(',');
        }
        self.empty = false;
        write!(self.out, "\"{key}\":").unwrap();
        self.out
    }

    fn text(&mut self, key: &str, value: &str) -> &mut Self {
        write_string(value, self.key(key));
        self
    }

    /// Left out when there is none, as JSON.stringify leaves out `undefined`.
    fn optional(&mut self, key: &str, value: Option<&str>) -> &mut Self {
        if let Some(value) = value {
            self.text(key, value);
        }
        self
    }

    fn number(&mut self, key: &str, value: f64) -> &mut Self {
        assert!(value.is_finite(), "{key} is {value}");
        write!(self.key(key), "{}", Js(value)).unwrap();
        self
    }

    fn list<T>(&mut self, key: &str, items: &[T], write: impl Fn(&T, &mut String)) -> &mut Self {
        let out = self.key(key);
        out.push('[');
        for (i, item) in items.iter().enumerate() {
            if i > 0 {
                out.push(',');
            }
            write(item, out);
        }
        out.push(']');
        self
    }

    fn raw(&mut self, key: &str, json: &str) -> &mut Self {
        self.key(key).push_str(json);
        self
    }

    fn end(&mut self) {
        self.out.push('}');
    }
}

fn compartment_json(c: &Compartment, out: &mut String) {
    JsonObject::new(out).optional("title", c.title.as_deref()).list("lines", &c.lines, |l, o| write_string(l, o)).end();
}

fn graph_node_json(n: &GraphNode, out: &mut String) {
    let mut o = JsonObject::new(out);
    o.text("id", &n.id).text("shape", n.shape.name()).text("keyword", &n.keyword).text("name", &n.name);
    o.list("compartments", &n.compartments, compartment_json);
    o.list("ports", &n.ports, |p, out| JsonObject::new(out).text("id", &p.id).text("label", &p.label).end());
    if let Some(children) = &n.children {
        o.list("children", children, graph_node_json);
    }
    o.number("width", n.width).number("height", n.height).end();
}

fn graph_json(graph: &DiagramGraph) -> String {
    let mut out = String::new();
    let mut o = JsonObject::new(&mut out);
    o.list("nodes", &graph.nodes, graph_node_json);
    o.list("edges", &graph.edges, |e, out| {
        let mut o = JsonObject::new(out);
        o.text("id", &e.id).text("kind", e.kind.name()).text("sourceId", &e.source_id).text("targetId", &e.target_id);
        o.optional("sourcePortId", e.source_port_id.as_deref()).optional("targetPortId", e.target_port_id.as_deref());
        o.optional("label", e.label.as_deref()).end();
    });
    o.end();
    out
}

fn point_json(p: &Point, out: &mut String) {
    JsonObject::new(out).number("x", p.x).number("y", p.y).end();
}

/// A layout ortho makes itself, which has neither ports nor ELK's label boxes.
fn layout_json(diagram: &LaidOutDiagram) -> String {
    let mut out = String::new();
    let mut o = JsonObject::new(&mut out);
    o.number("width", diagram.width).number("height", diagram.height);
    o.list("nodes", &diagram.nodes, |n, out| {
        assert!(n.ports.is_empty(), "{}: ports", n.id);
        let mut o = JsonObject::new(out);
        o.text("id", &n.id).text("shape", n.shape.name()).text("keyword", &n.keyword).text("name", &n.name);
        o.list("compartments", &n.compartments, compartment_json).raw("ports", "[]");
        o.raw("hasChildren", if n.has_children { "true" } else { "false" });
        o.number("x", n.x).number("y", n.y).number("width", n.width).number("height", n.height).end();
    });
    o.list("edges", &diagram.edges, |e, out| {
        assert!(e.label_box.is_none(), "{}: label box", e.id);
        let mut o = JsonObject::new(out);
        o.text("id", &e.id).text("kind", e.kind.name()).text("sourceId", &e.source_id).text("targetId", &e.target_id);
        o.optional("label", e.label.as_deref()).list("points", &e.points, point_json).end();
    });
    o.end();
    out
}

fn sequence_json(model: &SequenceModel) -> String {
    let mut out = String::new();
    let mut o = JsonObject::new(&mut out);
    o.list("lifelines", &model.lifelines, |l, out| {
        JsonObject::new(out).text("id", &l.id).text("label", &l.label).end()
    });
    o.list("messages", &model.messages, |m, out| {
        JsonObject::new(out)
            .text("sourceId", &m.source_id)
            .text("targetId", &m.target_id)
            .text("label", &m.label)
            .end();
    });
    o.end();
    out
}

/// The graph, the layout if ortho makes it, and the light SVG of a view ELK does not
/// lay out, each as JSON, or nothing for one ELK lays out.
fn stages(workspace: &Workspace, kind: DiagramKind, exposed: &[NodeId], title: &DiagramTitle) -> Option<Stages> {
    let ast = &workspace.ast;
    let (graph, layout) = match kind {
        DiagramKind::Process => (sequence_json(&extract_sequence_model(ast, exposed)), None),
        DiagramKind::UseCase => {
            let graph = extract_use_case_graph(ast, exposed);
            (graph_json(&graph), Some(layout_json(&layout_use_cases(&graph))))
        }
        DiagramKind::Logical => {
            let graph = extract_logical_graph(ast, exposed, &workspace.packages());
            (graph_json(&graph), Some(layout_json(&layout_tree(&graph))))
        }
        _ => return None,
    };
    let svg = render_exposed(workspace, kind, exposed, title, &LIGHT).unwrap().svg;
    Some(Stages { graph, layout, svg })
}

struct Stages {
    graph: String,
    layout: Option<String>,
    svg: String,
}

fn text_difference(name: &str, actual: &str, expected: &str) -> Option<String> {
    svg_difference(actual, expected).map(|difference| format!("{name}: {difference}"))
}

/// Recorded workspaces whose models have no errors, which are those whose views were recorded.
fn view_workspaces() -> Vec<(String, Workspace)> {
    let mut loaded = Vec::new();
    for line in read_recorded("workspaces.jsonl").lines() {
        let record: Json = serde_json::from_str(line).unwrap();
        let workspace = load(&strings_in(line));
        if workspace.reported(Severity::Error).is_empty() {
            loaded.push((text(&record, "name"), workspace));
        }
    }
    loaded
}

/// Every view the examples and fixtures declare: what it is called, whether it is
/// drawn and as what, and for those ortho lays out itself each stage, byte for byte
/// what the TypeScript produces, the SVG in both themes and from the model.
#[test]
#[ignore = "needs npm run oracle"]
fn views_of_recorded_models() {
    let mut lines = Vec::new();
    let mut compared = 0;
    for (name, workspace) in view_workspaces() {
        let ast = &workspace.ast;
        let mut taken = std::collections::HashSet::new();
        for document in &workspace.documents {
            for view in declared_views(ast, document.root) {
                let base = format!("{name}/{}", view_file_name(ast, view).trim_end_matches(".svg"));
                let mut id = base.clone();
                for n in 2.. {
                    if taken.insert(id.clone()) {
                        break;
                    }
                    id = format!("{base}-{n}");
                }
                let mut line = String::new();
                let mut o = JsonObject::new(&mut line);
                o.text("id", &id).text("source", &document.path);
                o.optional("name", ast.text(view, Prop::Name)).optional("definition", ast.text(view, Prop::Definition));
                let skipped = why_undrawable(ast, view);
                match &skipped {
                    Some(reason) => o.text("skipped", reason),
                    None => o.text("kind", diagram_kind(ast, view).name()),
                };
                o.end();
                lines.push(line);
                if skipped.is_some() {
                    continue;
                }
                let title = DiagramTitle {
                    heading: ast.text(view, Prop::Name).unwrap_or("").to_string(),
                    definition: ast.text(view, Prop::Definition).map(String::from),
                    source: document.path.clone(),
                };
                let kind = diagram_kind(ast, view);
                let Some(stages) = stages(&workspace, kind, &exposed_elements(ast, view), &title) else {
                    continue;
                };
                let stem = format!("views/{id}");
                let difference = text_difference("graph", &stages.graph, &read_recorded(&format!("{stem}.graph.json")))
                    .or_else(|| {
                        let layout = stages.layout.as_deref()?;
                        text_difference("layout", layout, &read_recorded(&format!("{stem}.layout.json")))
                    });
                if let Some(difference) = difference {
                    panic!("{id}, {difference}");
                }
                check_svgs(&id, &stem, |theme| render_view(&workspace, view, &title, theme).unwrap().svg);
                compared += 1;
            }
        }
    }
    let expected: Vec<String> = read_recorded("views/index.jsonl").lines().map(String::from).collect();
    if let Some(difference) = first_difference(&lines, &expected) {
        panic!("views/index.jsonl: {difference}");
    }
    assert!(compared > 0, "no view ortho lays out itself recorded");
    println!("{} views declared, {compared} laid out by ortho, identical at every stage", lines.len());
}

/// Views no model declares, of each element of every recorded model a view may
/// expose on its own, and of each package's together, as `npm run oracle` makes
/// them: every stage byte for byte what the TypeScript produces.
#[test]
#[ignore = "needs npm run oracle"]
fn generated_views_of_recorded_models() {
    let mut compared = 0;
    for (name, workspace) in view_workspaces() {
        let ast = &workspace.ast;
        let mut lines = Vec::new();
        for document in &workspace.documents {
            for node in ast.with_all_contents(document.root).into_iter().skip(1) {
                let mut views: Vec<(StandardView, Vec<NodeId>)> = Vec::new();
                for definition in [StandardView::GeneralView, StandardView::SequenceView] {
                    if definition.may_expose(ast, node) {
                        views.push((definition, vec![node]));
                    }
                }
                if ast[node].ty == Type::PackageDecl {
                    let exposed: Vec<NodeId> = ast
                        .children(node, Prop::Members)
                        .filter(|m| ast[*m].ty != Type::PackageDecl && StandardView::GeneralView.may_expose(ast, *m))
                        .collect();
                    if exposed.len() > 1 {
                        views.push((StandardView::GeneralView, exposed));
                    }
                }
                for (definition, exposed) in views {
                    let kind = diagram_kind_of(ast, Some(definition), &exposed);
                    let title = DiagramTitle {
                        heading: "generated".to_string(),
                        definition: Some(definition.name().to_string()),
                        source: document.path.clone(),
                    };
                    let Some(stages) = stages(&workspace, kind, &exposed, &title) else {
                        continue;
                    };
                    let mut line = String::new();
                    let mut o = JsonObject::new(&mut line);
                    o.text("file", &document.path).text("node", &ast.node_path(node));
                    o.text("definition", definition.name()).number("exposed", exposed.len() as f64);
                    o.text("kind", kind.name()).raw("graph", &stages.graph);
                    if let Some(layout) = &stages.layout {
                        o.raw("layout", layout);
                    }
                    o.text("svg", &stages.svg).end();
                    lines.push(line);
                }
            }
        }
        let expected: Vec<String> =
            read_recorded(&format!("generated-views/{name}.jsonl")).lines().map(String::from).collect();
        if let Some(i) = lines.iter().zip(&expected).position(|(a, e)| a != e) {
            panic!("{name}, line {i}: {}", svg_difference(&lines[i], &expected[i]).unwrap());
        }
        assert_eq!(lines.len(), expected.len(), "{name}: how many views");
        compared += lines.len();
    }
    assert!(compared > 0, "no generated view recorded");
    println!("{compared} generated views, identical at every stage");
}
