//! What the extractors share: the part a connector end names, a usage as declared,
//! and how big a box must be for what it holds.
use crate::model::graph::{BOX_HEADER_HEIGHT, Compartment, compartments_height, effective_name};
use crate::parser::ast::{Ast, NodeId, Prop, Type};
use crate::render::text_metrics::{Weight, measure_text};

/// Resolves a connector end to the part usage it names. A trailing port segment
/// falls back to the part that owns the port.
pub fn end_part(ast: &Ast, end: Option<NodeId>) -> Option<NodeId> {
    let end = end?;
    for segment in ast.references_of(end, Prop::Segments).into_iter().rev() {
        let Some(resolved) = ast.reference_target(segment) else {
            continue;
        };
        match ast[resolved].ty {
            Type::PartUsage => return Some(resolved),
            Type::PortUsage => {
                if let Some(owner) = ast[resolved].parent.filter(|owner| ast[*owner].ty == Type::PartUsage) {
                    return Some(owner);
                }
            }
            _ => {}
        }
    }
    None
}

/// The type a usage names: the name of what it resolves to, or else as written.
pub fn type_name(ast: &Ast, usage: NodeId) -> Option<&str> {
    let reference = ast.reference_of(usage, Prop::Type)?;
    let named = ast.reference_target(reference).and_then(|target| ast.text(target, Prop::Name));
    Some(named.unwrap_or_else(|| ast.reference_text(reference)))
}

/// A declaration as written (§8.2.3.6): `name : Type`, or the name or `: Type` alone.
pub fn declaration(name: &str, ty: Option<&str>) -> String {
    match ty {
        None => name.to_string(),
        Some(ty) if name.is_empty() => format!(": {ty}"),
        Some(ty) => format!("{name} : {ty}"),
    }
}

/// A usage as declared: `name : Type`, or its name or `: Type` alone.
pub fn usage_label(ast: &Ast, usage: NodeId) -> String {
    declaration(effective_name(ast, usage).unwrap_or(""), type_name(ast, usage))
}

/// The functions a part performs, as the spec's *perform actions* compartment
/// (§7.17.6): one line per `perform`, named by the function it reaches. This is how
/// realization is drawn: on the part that realizes, not as an edge.
pub fn perform_compartments(ast: &Ast, members: &[NodeId]) -> Vec<Compartment> {
    let lines: Vec<String> = members
        .iter()
        .filter(|member| ast[**member].ty == Type::PerformUsage)
        .map(|perform| performed_name(ast, *perform))
        .collect();
    if lines.is_empty() { Vec::new() } else { vec![Compartment { title: Some("perform actions".to_string()), lines }] }
}

fn performed_name(ast: &Ast, perform: NodeId) -> String {
    let last = ast.child(perform, Prop::Target).and_then(|target| ast.references_of(target, Prop::Segments).pop());
    let named = last.map(|reference| {
        let target_name = ast.reference_target(reference).and_then(|target| ast.text(target, Prop::Name));
        target_name.unwrap_or_else(|| ast.reference_text(reference))
    });
    named.or_else(|| effective_name(ast, perform)).unwrap_or("").to_string()
}

/// A box's size once its keyword, name and compartments have to fit inside it.
pub fn box_size(
    name: &str,
    compartments: &[Compartment],
    min_width: f64,
    plain_height: f64,
    keyword: &str,
) -> (f64, f64) {
    let mut width = min_width
        .max(measure_text(name, 12.0, Weight::Bold) + 28.0)
        .max(measure_text(&format!("«{keyword}»"), 9.0, Weight::Regular) + 28.0);
    for c in compartments {
        if let Some(title) = &c.title {
            width = width.max(measure_text(title, 9.0, Weight::Regular) + 16.0);
        }
        for line in &c.lines {
            width = width.max(measure_text(line, 11.0, Weight::Regular) + 16.0);
        }
    }
    let height =
        if compartments.is_empty() { plain_height } else { BOX_HEADER_HEIGHT + compartments_height(compartments) };
    (width, height)
}
