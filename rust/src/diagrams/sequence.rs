//! The process view's model: the lifelines a scenario's messages pass between.
use super::connector_ends::{end_part, usage_label};
use crate::model::graph::qualified_name;
use crate::parser::ast::{Ast, NodeId, Prop, Type};

/// A lifeline, headed by its part as declared: `uiApp : UiApp`, or a person's `user`.
#[derive(Debug)]
pub struct SequenceLifeline {
    pub id: String,
    pub label: String,
}

#[derive(Debug)]
pub struct SequenceMessage {
    pub source_id: String,
    pub target_id: String,
    pub label: String,
}

#[derive(Debug)]
pub struct SequenceModel {
    pub lifelines: Vec<SequenceLifeline>,
    pub messages: Vec<SequenceMessage>,
}

/// A message as declared (§8.2.3.16): `name of Payload`, its name or its payload
/// alone, or «message» for one with neither. The payload is as written, so a
/// qualified name stays qualified.
fn message_label(ast: &Ast, message: NodeId) -> String {
    let name = ast.text(message, Prop::Name).unwrap_or("");
    match ast.reference_of(message, Prop::Payload).map(|payload| ast.reference_text(payload)) {
        Some(payload) if name.is_empty() => payload.to_string(),
        Some(payload) => format!("{name} of {payload}"),
        None if name.is_empty() => "«message»".to_string(),
        None => name.to_string(),
    }
}

/// Lifelines are the parts referenced by message ends, in order of first
/// appearance, a person's as any other (§8.2.3.9); messages keep document order
/// (`then` is explicit notation for the same ordering).
pub fn extract_sequence_model(ast: &Ast, exposed: &[NodeId]) -> SequenceModel {
    let mut lifelines: Vec<SequenceLifeline> = Vec::new();
    let mut messages = Vec::new();
    for element in exposed {
        for node in ast.with_all_contents(*element).into_iter().skip(1) {
            if ast[node].ty != Type::MessageUsage {
                continue;
            }
            let source = end_part(ast, ast.child(node, Prop::Source));
            let target = end_part(ast, ast.child(node, Prop::Target));
            let (Some(source), Some(target)) = (source, target) else {
                continue;
            };
            for part in [source, target] {
                let id = qualified_name(ast, part);
                if !lifelines.iter().any(|lifeline| lifeline.id == id) {
                    lifelines.push(SequenceLifeline { id, label: usage_label(ast, part) });
                }
            }
            messages.push(SequenceMessage {
                source_id: qualified_name(ast, source),
                target_id: qualified_name(ast, target),
                label: message_label(ast, node),
            });
        }
    }
    SequenceModel { lifelines, messages }
}
