use crate::parser::ast::{Ast, NodeId, Prop, Type};

/// How a node is drawn. A definition is a box with square corners and a usage one with
/// rounded corners (§8.2.3.6); the rest have shapes of their own.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum NodeShape {
    Definition,
    Usage,
    Actor,
    Package,
}

pub struct Compartment {
    /// Drawn small and grey above the lines, as the spec labels a compartment.
    pub title: Option<String>,
    pub lines: Vec<String>,
}

/// Box metrics shared by the renderer, the layouter and the extractors.
pub const BOX_HEADER_HEIGHT: f64 = 32.0;
/// A usage's corners; a definition's are square (§8.2.3.6).
pub const USAGE_RADIUS: f64 = 10.0;
pub const COMPARTMENT_LINE_HEIGHT: f64 = 15.0;
pub const COMPARTMENT_TITLE_HEIGHT: f64 = 13.0;
pub const COMPARTMENT_PADDING: f64 = 8.0;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum EdgeKind {
    Connection,
    Association,
    Include,
    Import,
    Decomposition,
    Dependency,
}

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
