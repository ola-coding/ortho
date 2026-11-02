//! The port against what the TypeScript produces, which `npm run oracle` records in
//! `rust/oracle/`. Ignored until asked for: `cargo test --test oracle -- --ignored`.
use ortho::diagrams::sequence::{SequenceLifeline, SequenceMessage, SequenceModel};
use ortho::layout::{LabelBox, LaidOutDiagram, LaidOutEdge, LaidOutNode, LaidOutPort, Point};
use ortho::model::graph::{Compartment, EdgeKind, NodeShape};
use ortho::parser::ast::{Ast, Field, NodeId, Prop, Type, Value};
use ortho::parser::workspace::Workspace;
use ortho::parser::{parse, position};
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
