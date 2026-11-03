use crate::parser::ast::{Ast, NodeId, Prop, Type};

/// How a node is drawn. A definition is a box with square corners and a usage one with
/// rounded corners (§8.2.3.6); the rest have shapes of their own.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum NodeShape {
    Definition,
    Usage,
    Actor,
    Package,
}

#[derive(Clone, PartialEq, Debug)]
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

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum EdgeKind {
    Connection,
    Association,
    Include,
    Import,
    Decomposition,
    Dependency,
}

/// How much height a box's compartments take below its header.
pub fn compartments_height(compartments: &[Compartment]) -> f64 {
    compartments.iter().fold(0.0, |sum, c| {
        sum + COMPARTMENT_PADDING
            + if c.title.is_some() { COMPARTMENT_TITLE_HEIGHT } else { 0.0 }
            + c.lines.len() as f64 * COMPARTMENT_LINE_HEIGHT
    })
}

#[derive(Clone, Debug)]
pub struct GraphPort {
    pub id: String,
    pub label: String,
}

#[derive(Clone, Debug)]
pub struct GraphNode {
    pub id: String,
    pub shape: NodeShape,
    /// The keyword drawn above the name, such as `part def`; empty for none.
    pub keyword: String,
    pub name: String,
    pub compartments: Vec<Compartment>,
    pub ports: Vec<GraphPort>,
    /// Nested nodes, such as the parts of an assembly; sized by the layouter, not the extractor.
    pub children: Option<Vec<GraphNode>>,
    pub width: f64,
    pub height: f64,
}

#[derive(Clone, Debug)]
pub struct GraphEdge {
    pub id: String,
    pub kind: EdgeKind,
    pub source_id: String,
    pub target_id: String,
    pub source_port_id: Option<String>,
    pub target_port_id: Option<String>,
    /// Drawn beside the line as written: a keyword with its guillemets, «include».
    pub label: Option<String>,
}

impl GraphEdge {
    pub fn new(id: String, kind: EdgeKind, source_id: String, target_id: String) -> GraphEdge {
        GraphEdge { id, kind, source_id, target_id, source_port_id: None, target_port_id: None, label: None }
    }
}

#[derive(Clone, Debug, Default)]
pub struct DiagramGraph {
    pub nodes: Vec<GraphNode>,
    pub edges: Vec<GraphEdge>,
}

impl NodeShape {
    /// As the recordings and the TypeScript name it.
    pub fn name(self) -> &'static str {
        match self {
            NodeShape::Definition => "definition",
            NodeShape::Usage => "usage",
            NodeShape::Actor => "actor",
            NodeShape::Package => "package",
        }
    }
}

impl EdgeKind {
    /// As the recordings and the TypeScript name it.
    pub fn name(self) -> &'static str {
        match self {
            EdgeKind::Connection => "connection",
            EdgeKind::Association => "association",
            EdgeKind::Include => "include",
            EdgeKind::Import => "import",
            EdgeKind::Decomposition => "decomposition",
            EdgeKind::Dependency => "dependency",
        }
    }
}

/// The keyword of each element a view draws as a box (§8.2.3).
fn keyword(ty: Type) -> &'static str {
    match ty {
        Type::PartDef => "part def",
        Type::PartUsage => "part",
        Type::ActionDef => "action def",
        Type::ActionUsage => "action",
        Type::UseCaseDef => "use case def",
        Type::UseCaseUsage | Type::IncludeUsage => "use case",
        Type::SubjectUsage => "subject",
        _ => "",
    }
}

/// Whether a node stands for a use case, a usage or a definition.
pub fn is_use_case_node(keyword: &str) -> bool {
    keyword == "use case" || keyword == "use case def"
}

/// How SysML v2 draws an element as a box: its corners, and the keyword above its name.
pub fn box_of(ast: &Ast, element: NodeId) -> (NodeShape, &'static str) {
    let ty = ast[element].ty;
    (if ty.is_definition() { NodeShape::Definition } else { NodeShape::Usage }, keyword(ty))
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

/// An element's name after those of what contains it: `Pkg::Def::usage`. A container
/// without a name adds nothing.
pub fn qualified_name(ast: &Ast, id: NodeId) -> String {
    let mut qualified = effective_name(ast, id).unwrap_or("").to_string();
    let mut container = ast[id].parent;
    while let Some(at) = container {
        if let Some(name) = effective_name(ast, at).filter(|name| !name.is_empty()) {
            qualified = format!("{name}::{qualified}");
        }
        container = ast[at].parent;
    }
    qualified
}
