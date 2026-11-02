use crate::parser::ast::{Ast, NodeId, Prop, Type};

/// An element's name, or the one an unnamed element takes on: a redefinition,
/// `part :>> engine`, is named after what it redefines, and a perform,
/// `perform providePower.generateTorque`, after what it performs.
pub fn effective_name(ast: &Ast, id: NodeId) -> Option<&str> {
    if let Some(name) = ast.text(id, Prop::Name) {
        return Some(name);
    }
    let named = ast.texts(id, Prop::Redefines).next().or_else(|| {
        if ast[id].ty != Type::PerformUsage {
            return None;
        }
        ast.child(id, Prop::Target).and_then(|target| ast.texts(target, Prop::Segments).last())
    })?;
    let after_colons = named.rfind("::").map_or(0, |i| i + 2);
    let after_dot = named.rfind('.').map_or(0, |i| i + 1);
    Some(&named[after_colons.max(after_dot)..])
}
