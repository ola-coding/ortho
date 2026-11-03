//! The implementation, physical and deployment views, laid out by the Eclipse Layout
//! Kernel, given and giving back ELK's JSON graph format.
use super::{LabelBox, LaidOutDiagram, LaidOutEdge, LaidOutNode, LaidOutPort, PORT_LABEL_FONT, PORT_SIZE, Point};
use crate::model::graph::{DiagramGraph, EdgeKind, GraphNode, NodeShape, compartments_height};
use crate::render::number::Js;
use crate::render::text_metrics::{Weight, measure_text};
use org_eclipse_elk_graph_json::org::eclipse::elk::graph::json::layout_api::layout_value;
use serde_json::{Map, Value, json};
use std::collections::{HashMap, HashSet};

/// `graph` with its nodes, ports, edges and labels placed.
pub fn layout(graph: Value) -> Result<Value, String> {
    layout_value(graph, "{}")
}

/// The height of the box ELK reserves for a port's label.
const PORT_LABEL_HEIGHT: f64 = 11.0;

/// Wire labels ELK places (interface names on the physical view).
const EDGE_LABEL_FONT: f64 = 9.0;
const EDGE_LABEL_HEIGHT: f64 = 12.0;

/// Target width-to-height ratio when packing.
const PACKING_ASPECT: &str = "1.6";

/// Gaps between the cells of a grid.
const GRID_GAP_X: f64 = 40.0;
const GRID_GAP_Y: f64 = 20.0;

#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub enum Algorithm {
    #[default]
    Layered,
    /// Packs boxes into rows at a page-shaped aspect ratio, level by level, instead
    /// of layering them. It is for edge-free views (deployment), where layering would
    /// put every box in one long row.
    RectPacking,
}

#[derive(Clone, Copy, Default)]
pub struct LayoutOptions {
    pub algorithm: Algorithm,
    /// Lay out the contents of each innermost container, one whose children hold
    /// nothing and no edge touches, as a grid ortho computes, instead of ELK's single
    /// row. For the implementation view, where a package's modules never carry an
    /// edge, and a long row of them set the whole drawing's width.
    pub grid_leaves: bool,
}

/// Contents ortho places itself (see `grid_leaves`), relative to the container's top-left.
type Grid<'a> = Vec<(&'a GraphNode, f64, f64)>;

struct Conversion<'a> {
    packing: bool,
    /// Present when innermost containers are gridded; collects each one's grid.
    grids: Option<HashMap<&'a str, Grid<'a>>>,
    /// Every node an edge touches.
    endpoints: HashSet<&'a str>,
}

struct Padding {
    top: f64,
    left: f64,
    bottom: f64,
    right: f64,
}

/// Room round a container's contents. The extra at the top leaves room for its
/// title, and its own compartments sit under the title, so its contents start below
/// them.
fn padding_of(n: &GraphNode) -> Padding {
    Padding { top: 45.0 + compartments_height(&n.compartments), left: 25.0, bottom: 25.0, right: 25.0 }
}

/// The sum of `sizes` before `index`, each followed by `gap`.
fn before(sizes: &[f64], index: usize, gap: f64) -> f64 {
    sizes[..index].iter().fold(0.0, |sum, size| sum + size + gap)
}

/// A container's contents as a grid, in declaration order: one row of up to three,
/// else as square as it goes, each box centred in its cell. A container whose name is
/// wider than its grid keeps the width of its name, with the grid centred under it.
fn grid_of<'a>(n: &GraphNode, children: &'a [GraphNode]) -> (Grid<'a>, f64, f64) {
    let count = children.len();
    let columns = if count <= 3 { count } else { (count as f64).sqrt().ceil() as usize };
    let rows = count.div_ceil(columns);
    let widths: Vec<f64> = (0..columns)
        .map(|column| children.iter().skip(column).step_by(columns).fold(f64::NEG_INFINITY, |m, c| m.max(c.width)))
        .collect();
    let heights: Vec<f64> = (0..rows)
        .map(|row| children.iter().skip(row * columns).take(columns).fold(f64::NEG_INFINITY, |m, c| m.max(c.height)))
        .collect();
    let pad = padding_of(n);
    let grid_width = pad.left + before(&widths, columns, GRID_GAP_X) - GRID_GAP_X + pad.right;
    let width = grid_width.max(n.width);
    let indent = (width - grid_width) / 2.0;
    let cells = children
        .iter()
        .enumerate()
        .map(|(i, node)| {
            let (column, row) = (i % columns, i / columns);
            let x = indent + pad.left + before(&widths, column, GRID_GAP_X) + (widths[column] - node.width) / 2.0;
            let y = pad.top + before(&heights, row, GRID_GAP_Y) + (heights[row] - node.height) / 2.0;
            (node, x, y)
        })
        .collect();
    let height = pad.top + before(&heights, rows, GRID_GAP_Y) - GRID_GAP_Y + pad.bottom;
    (cells, width, height)
}

fn options(pairs: &[(&str, String)]) -> Map<String, Value> {
    pairs.iter().map(|(key, value)| (key.to_string(), Value::String(value.clone()))).collect()
}

fn to_elk_node<'a>(n: &'a GraphNode, conversion: &mut Conversion<'a>) -> Value {
    let packing = conversion.packing;
    let mut node = Map::new();
    node.insert("id".into(), json!(n.id));
    let mut layout_options = options(&[("elk.portConstraints", "FREE".into())]);
    let children = n.children.as_deref().unwrap_or(&[]);
    let gridded = conversion.grids.is_some()
        && !children.is_empty()
        && children.iter().all(|c| {
            c.children.as_ref().is_none_or(|inner| inner.is_empty())
                && c.ports.is_empty()
                && !conversion.endpoints.contains(c.id.as_str())
        });
    if gridded {
        // ELK sees a plain box of the grid's size; flattening fills it in.
        let (cells, width, height) = grid_of(n, children);
        conversion.grids.as_mut().unwrap().insert(&n.id, cells);
        node.insert("width".into(), json!(width));
        node.insert("height".into(), json!(height));
    } else if !children.is_empty() {
        let elk_children: Vec<Value> = children.iter().map(|child| to_elk_node(child, conversion)).collect();
        node.insert("children".into(), Value::Array(elk_children));
        let pad = padding_of(n);
        let padding =
            format!("[top={},left={},bottom={},right={}]", Js(pad.top), Js(pad.left), Js(pad.bottom), Js(pad.right));
        // ELK sizes a container to its contents; the container's own size, worked
        // out from its name, keeps a long name inside its box, and the contents
        // centre under the name when they are the narrower. Layering downwards, ELK
        // reads both with their axes swapped; packing has no direction and takes them
        // as they are.
        let (first, second) = if packing { (n.width, n.height) } else { (n.height, n.width) };
        layout_options.extend(options(&[
            ("elk.padding", padding),
            ("elk.nodeSize.constraints", "MINIMUM_SIZE".into()),
            ("elk.nodeSize.minimum", format!("({}, {})", Js(first), Js(second))),
            ("elk.contentAlignment", if packing { "H_CENTER V_TOP" } else { "H_LEFT V_CENTER" }.into()),
            ("elk.spacing.nodeNode", if packing { "24" } else { "40" }.into()),
        ]));
        if packing {
            // Packing lays out each level on its own, so every container names the
            // algorithm for its children.
            layout_options.extend(options(&[
                ("elk.algorithm", "rectpacking".into()),
                ("elk.aspectRatio", PACKING_ASPECT.into()),
            ]));
        }
    } else {
        node.insert("width".into(), json!(n.width));
        node.insert("height".into(), json!(n.height));
    }
    if !n.ports.is_empty() {
        // ELK places each port's label outside the box beside its port, flips it to
        // the other side when a neighbouring label would collide, and grows the box
        // until the ports fit.
        layout_options.insert("elk.portLabels.placement".into(), json!("OUTSIDE"));
        layout_options.insert("elk.nodeSize.constraints".into(), json!("PORTS PORT_LABELS MINIMUM_SIZE"));
        layout_options
            .entry("elk.nodeSize.minimum")
            .or_insert_with(|| json!(format!("({}, {})", Js(n.width), Js(n.height))));
        let ports: Vec<Value> = n
            .ports
            .iter()
            .map(|p| {
                json!({
                    "id": p.id,
                    "width": PORT_SIZE,
                    "height": PORT_SIZE,
                    "labels": [{
                        "id": format!("{}@label", p.id),
                        "text": p.label,
                        "width": measure_text(&p.label, PORT_LABEL_FONT, Weight::Regular) + 2.0,
                        "height": PORT_LABEL_HEIGHT,
                    }],
                    // straddle the node border, EA-style
                    "layoutOptions": { "elk.port.borderOffset": format!("{}", Js(-PORT_SIZE / 2.0)) },
                })
            })
            .collect();
        node.insert("ports".into(), Value::Array(ports));
    }
    node.insert("layoutOptions".into(), Value::Object(layout_options));
    Value::Object(node)
}

/// The JSON ELK is given for `graph`, and what ortho places itself.
fn to_elk_graph<'a>(graph: &'a DiagramGraph, layout_options: LayoutOptions) -> (Value, Conversion<'a>) {
    let packing = layout_options.algorithm == Algorithm::RectPacking;
    let mut conversion = Conversion {
        packing,
        grids: layout_options.grid_leaves.then(HashMap::new),
        endpoints: graph.edges.iter().flat_map(|e| [e.source_id.as_str(), e.target_id.as_str()]).collect(),
    };
    let root_options = if packing {
        json!({ "elk.algorithm": "rectpacking", "elk.aspectRatio": PACKING_ASPECT, "elk.spacing.nodeNode": "30" })
    } else {
        json!({
            // ELK otherwise reports each edge, and the labels on it, relative to the
            // node that contains it.
            "org.eclipse.elk.json.edgeCoords": "ROOT",
            "elk.algorithm": "layered",
            "elk.direction": "DOWN",
            "elk.hierarchyHandling": "INCLUDE_CHILDREN",
            "elk.spacing.nodeNode": "50",
            "elk.layered.spacing.nodeNodeBetweenLayers": "70",
            "elk.spacing.edgeNode": "30",
            "elk.spacing.edgeEdge": "20",
            "elk.edgeRouting": "ORTHOGONAL",
        })
    };
    let children: Vec<Value> = graph.nodes.iter().map(|n| to_elk_node(n, &mut conversion)).collect();
    let edges: Vec<Value> = graph
        .edges
        .iter()
        .map(|e| {
            let mut edge = json!({
                "id": e.id,
                "sources": [e.source_port_id.as_ref().unwrap_or(&e.source_id)],
                "targets": [e.target_port_id.as_ref().unwrap_or(&e.target_id)],
            });
            // Wire labels on the physical view (interface names) and «import» on the
            // implementation view are real ELK labels, so the layout keeps them clear
            // of boxes and other lines; a midpoint label lands wherever the route
            // happens to be busiest, or on the arrowhead of a short edge.
            if let Some(label) = &e.label {
                edge["labels"] = json!([{
                    "id": format!("{}@label", e.id),
                    "text": label,
                    "width": measure_text(label, EDGE_LABEL_FONT, Weight::Regular) + 4.0,
                    "height": EDGE_LABEL_HEIGHT,
                }]);
            }
            edge
        })
        .collect();
    let elk_graph = json!({ "id": "root", "layoutOptions": root_options, "children": children, "edges": edges });
    (elk_graph, conversion)
}

/// The JSON ELK is given for `graph`.
pub fn elk_graph(graph: &DiagramGraph, layout_options: LayoutOptions) -> Value {
    to_elk_graph(graph, layout_options).0
}

fn number_or(value: &Value, key: &str, default: f64) -> f64 {
    value.get(key).and_then(Value::as_f64).unwrap_or(default)
}

fn index_hierarchy<'a>(nodes: &'a [GraphNode], by_id: &mut HashMap<&'a str, &'a GraphNode>) {
    for node in nodes {
        by_id.insert(&node.id, node);
        index_hierarchy(node.children.as_deref().unwrap_or(&[]), by_id);
    }
}

fn list(value: &Value, key: &str) -> Vec<Value> {
    value.get(key).and_then(Value::as_array).cloned().unwrap_or_default()
}

fn point(value: &Value) -> Point {
    Point { x: number_or(value, "x", 0.0), y: number_or(value, "y", 0.0) }
}

/// Lays out a graph with ELK, every box, port and route in root coordinates.
pub fn layout_graph(graph: &DiagramGraph, layout_options: LayoutOptions) -> Result<LaidOutDiagram, String> {
    let (elk_graph, conversion) = to_elk_graph(graph, layout_options);
    let result = layout(elk_graph)?;
    Ok(laid_out(graph, &conversion, &result))
}

/// What ELK placed, as ortho draws it.
fn laid_out(graph: &DiagramGraph, conversion: &Conversion, result: &Value) -> LaidOutDiagram {
    let mut by_id = HashMap::new();
    index_hierarchy(&graph.nodes, &mut by_id);

    // Flatten the hierarchy to absolute coordinates, parents before children, so
    // paint order keeps containers behind their contents.
    let mut nodes = Vec::new();
    fn flatten(
        children: &[Value],
        offset: (f64, f64),
        by_id: &HashMap<&str, &GraphNode>,
        conversion: &Conversion,
        nodes: &mut Vec<LaidOutNode>,
    ) {
        for c in children {
            let id = c["id"].as_str().unwrap_or("");
            let source = by_id.get(id);
            let x = number_or(c, "x", 0.0) + offset.0;
            let y = number_or(c, "y", 0.0) + offset.1;
            let ports = list(c, "ports")
                .iter()
                .map(|p| {
                    let port_id = p["id"].as_str().unwrap_or("");
                    let label = source.and_then(|s| s.ports.iter().find(|q| q.id == port_id)).map(|q| q.label.clone());
                    let label_place = p.get("labels").and_then(|labels| labels.get(0));
                    let label_x = label_place.and_then(|l| l.get("x")).and_then(Value::as_f64);
                    let label_y = label_place.and_then(|l| l.get("y")).and_then(Value::as_f64);
                    LaidOutPort {
                        id: port_id.to_string(),
                        label: label.unwrap_or_else(|| port_id.to_string()),
                        x: number_or(p, "x", 0.0),
                        y: number_or(p, "y", 0.0),
                        label_at: label_x.zip(label_y),
                    }
                })
                .collect();
            nodes.push(LaidOutNode {
                id: id.to_string(),
                shape: source.map_or(NodeShape::Usage, |s| s.shape),
                keyword: source.map(|s| s.keyword.clone()).unwrap_or_default(),
                name: source.map_or_else(|| id.to_string(), |s| s.name.clone()),
                compartments: source.map(|s| s.compartments.clone()).unwrap_or_default(),
                ports,
                has_children: source.is_some_and(|s| s.children.as_ref().is_some_and(|c| !c.is_empty())),
                x,
                y,
                width: number_or(c, "width", 80.0),
                height: number_or(c, "height", 40.0),
            });
            let cells = conversion.grids.as_ref().and_then(|grids| grids.get(id));
            for (cell, cell_x, cell_y) in cells.into_iter().flatten() {
                nodes.push(LaidOutNode {
                    id: cell.id.clone(),
                    shape: cell.shape,
                    keyword: cell.keyword.clone(),
                    name: cell.name.clone(),
                    compartments: cell.compartments.clone(),
                    ports: Vec::new(),
                    has_children: false,
                    x: x + cell_x,
                    y: y + cell_y,
                    width: cell.width,
                    height: cell.height,
                });
            }
            flatten(&list(c, "children"), (x, y), by_id, conversion, nodes);
        }
    }
    flatten(&list(result, "children"), (0.0, 0.0), &by_id, conversion, &mut nodes);

    // Edges arrive in root coordinates (`edgeCoords: ROOT`), labels included.
    let edges = list(result, "edges")
        .iter()
        .map(|e| {
            let id = e["id"].as_str().unwrap_or("");
            let source = graph.edges.iter().find(|edge| edge.id == id);
            let points = match e.get("sections").and_then(|s| s.get(0)) {
                Some(section) => {
                    let mut points = vec![point(&section["startPoint"])];
                    points.extend(list(section, "bendPoints").iter().map(point));
                    points.push(point(&section["endPoint"]));
                    points
                }
                None => Vec::new(),
            };
            let label = e.get("labels").and_then(|labels| labels.get(0));
            let label_box = label.filter(|l| l.get("x").is_some() && l.get("y").is_some()).map(|l| LabelBox {
                x: number_or(l, "x", 0.0),
                y: number_or(l, "y", 0.0),
                width: number_or(l, "width", 0.0),
                height: number_or(l, "height", 0.0),
            });
            LaidOutEdge {
                id: id.to_string(),
                kind: source.map_or(EdgeKind::Connection, |s| s.kind),
                source_id: source.map(|s| s.source_id.clone()).unwrap_or_default(),
                target_id: source.map(|s| s.target_id.clone()).unwrap_or_default(),
                label: source.and_then(|s| s.label.clone()),
                points,
                label_box,
            }
        })
        .collect();

    LaidOutDiagram {
        width: number_or(result, "width", 400.0),
        height: number_or(result, "height", 300.0),
        nodes,
        edges,
    }
}
