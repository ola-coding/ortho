//! The port against what the TypeScript produces, which `npm run oracle` records in
//! `rust/oracle/`. Ignored until asked for: `cargo test --test oracle -- --ignored`.
use ortho::parser::ast::{Ast, Field, NodeId, Prop, Type, Value};
use ortho::parser::parse;
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

/// A property as the grammar names it: `ShortName` is `shortName`.
fn prop_name(prop: Prop) -> String {
    let name = format!("{prop:?}");
    format!("{}{}", name[..1].to_lowercase(), &name[1..])
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
            names.push((prop_name(field.prop), field.prop));
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
        Value::Reference(value) => {
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
