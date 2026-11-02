pub mod elk;

use crate::model::graph::{Compartment, EdgeKind, NodeShape};

pub const PORT_SIZE: f64 = 10.0;
/// Port labels' font size.
pub const PORT_LABEL_FONT: f64 = 9.0;

pub struct LaidOutPort {
    pub id: String,
    pub label: String,
    pub x: f64,
    pub y: f64,
    /// Top-left of the label's box, relative to the port, as ELK placed it.
    pub label_at: Option<(f64, f64)>,
}

pub struct LaidOutNode {
    pub id: String,
    pub shape: NodeShape,
    pub keyword: String,
    pub name: String,
    pub compartments: Vec<Compartment>,
    pub ports: Vec<LaidOutPort>,
    pub has_children: bool,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

pub struct Point {
    pub x: f64,
    pub y: f64,
}

/// The box ELK reserved for an edge's label, in root coordinates.
pub struct LabelBox {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

pub struct LaidOutEdge {
    pub id: String,
    pub kind: EdgeKind,
    pub source_id: String,
    pub target_id: String,
    pub label: Option<String>,
    pub points: Vec<Point>,
    pub label_box: Option<LabelBox>,
}

pub struct LaidOutDiagram {
    pub width: f64,
    pub height: f64,
    pub nodes: Vec<LaidOutNode>,
    pub edges: Vec<LaidOutEdge>,
}
