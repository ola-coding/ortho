//! The implementation, physical and deployment views, laid out by the Eclipse Layout
//! Kernel, given and giving back ELK's JSON graph format.
use super::{LabelBox, LaidOutDiagram, LaidOutEdge, LaidOutNode, LaidOutPort, PORT_LABEL_FONT, PORT_SIZE, Point};
use crate::model::graph::{DiagramGraph, EdgeKind, GraphEdge, GraphNode, NodeShape, compartments_height};
use crate::render::number::Js;
use crate::render::page::{PAGE_HEIGHT, PAGE_WIDTH};
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
    /// Lay out the contents of each container that no edge or port reaches into as
    /// a grid ortho computes, in declaration order, instead of ELK's single row; a
    /// container inside it is a grid of its own. For the implementation view, where
    /// a package's modules never carry an edge, and packages that import nothing
    /// from each other stood in one long row that set the whole drawing's width.
    pub grid_leaves: bool,
}

/// A box ortho places itself (see `grid_leaves`), relative to the top-left of the
/// container ELK placed, at the size its own contents give it.
struct Cell<'a> {
    node: &'a GraphNode,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
}

/// Contents ortho places itself, each container before what it holds.
type Grid<'a> = Vec<Cell<'a>>;

/// Siblings drawn as one block, each joined by a single line to the same box
/// outside it, all one way: packages that all import one package, say. ELK lays every
/// one of them in the same layer, so one row; ortho sets them out as a grid in
/// declaration order, and ELK sees one box with a port for each line. A line leaves
/// a box in the bottom row straight down; one from a row above runs down the gap to
/// the right of its column. Lines into the block come in mirrored, from the top.
struct Bundle<'a> {
    members: Vec<Cell<'a>>,
    /// Each line's way inside the block, from its member to its port, or from its
    /// port to its member: relative to the block.
    routes: HashMap<&'a str, Vec<Point>>,
}

const BUNDLE: &str = "bundle:";
/// Between lines running side by side down a gap.
const LANE: f64 = 10.0;
/// Between the rows of a block: room for a line to turn.
const BUNDLE_GAP_Y: f64 = 40.0;

struct Conversion<'a> {
    packing: bool,
    edges: &'a [GraphEdge],
    bundles: HashMap<String, Bundle<'a>>,
    /// For each line rerouted through a block's port: the port, and whether the
    /// line leaves from it.
    ports_of: HashMap<&'a str, (String, bool)>,
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
/// A container among the contents is a grid of its own, sized to it.
fn grid_of<'a>(n: &GraphNode, children: &'a [GraphNode]) -> (Grid<'a>, f64, f64) {
    // Each child at its size, with what it holds placed relative to it.
    let laid: Vec<(f64, f64, Grid<'a>)> = children
        .iter()
        .map(|c| match c.children.as_deref() {
            Some(inner) if !inner.is_empty() => {
                let (cells, width, height) = grid_of(c, inner);
                (width, height, cells)
            }
            _ => (c.width, c.height, Vec::new()),
        })
        .collect();
    let count = children.len();
    let columns = if count <= 3 { count } else { (count as f64).sqrt().ceil() as usize };
    let rows = count.div_ceil(columns);
    let widths: Vec<f64> = (0..columns)
        .map(|column| laid.iter().skip(column).step_by(columns).fold(f64::NEG_INFINITY, |m, c| m.max(c.0)))
        .collect();
    let heights: Vec<f64> = (0..rows)
        .map(|row| laid.iter().skip(row * columns).take(columns).fold(f64::NEG_INFINITY, |m, c| m.max(c.1)))
        .collect();
    let pad = padding_of(n);
    let grid_width = pad.left + before(&widths, columns, GRID_GAP_X) - GRID_GAP_X + pad.right;
    let width = grid_width.max(n.width);
    let indent = (width - grid_width) / 2.0;
    let mut cells = Vec::new();
    for (i, (node, (cell_width, cell_height, inner))) in children.iter().zip(laid).enumerate() {
        let (column, row) = (i % columns, i / columns);
        let x = indent + pad.left + before(&widths, column, GRID_GAP_X) + (widths[column] - cell_width) / 2.0;
        let y = pad.top + before(&heights, row, GRID_GAP_Y) + (heights[row] - cell_height) / 2.0;
        cells.push(Cell { node, x, y, width: cell_width, height: cell_height });
        cells.extend(inner.into_iter().map(|c| Cell { x: x + c.x, y: y + c.y, ..c }));
    }
    let height = pad.top + before(&heights, rows, GRID_GAP_Y) - GRID_GAP_Y + pad.bottom;
    (cells, width, height)
}

/// Whether no edge or port reaches anything `n` holds, at any depth.
fn closed(n: &GraphNode, endpoints: &HashSet<&str>) -> bool {
    n.children
        .as_deref()
        .unwrap_or(&[])
        .iter()
        .all(|c| c.ports.is_empty() && !endpoints.contains(c.id.as_str()) && closed(c, endpoints))
}

fn options(pairs: &[(&str, String)]) -> Map<String, Value> {
    pairs.iter().map(|(key, value)| (key.to_string(), Value::String(value.clone()))).collect()
}

/// A box's size to ELK, with the grid of what it holds, if anything.
fn plain_size(n: &GraphNode) -> (f64, f64, Grid<'_>) {
    match n.children.as_deref() {
        Some(inner) if !inner.is_empty() => {
            let (cells, width, height) = grid_of(n, inner);
            (width, height, cells)
        }
        _ => (n.width, n.height, Vec::new()),
    }
}

/// The groups of siblings to draw as blocks: more than three, each a box ortho lays
/// out itself, with no port, and joined by one line to the same box, the same way.
fn bundles_among<'a>(children: &'a [GraphNode], conversion: &Conversion<'a>) -> Vec<Vec<&'a GraphNode>> {
    if conversion.grids.is_none() {
        return Vec::new();
    }
    let mut groups: Vec<((&str, bool), Vec<&'a GraphNode>)> = Vec::new();
    for c in children {
        let inner = c.children.as_deref().unwrap_or(&[]);
        if !c.ports.is_empty() || !(inner.is_empty() || closed(c, &conversion.endpoints)) {
            continue;
        }
        let lines: Vec<&GraphEdge> =
            conversion.edges.iter().filter(|e| e.source_id == c.id || e.target_id == c.id).collect();
        let [line] = lines[..] else { continue };
        if line.source_port_id.is_some() || line.target_port_id.is_some() || line.source_id == line.target_id {
            continue;
        }
        let outgoing = line.source_id == c.id;
        let key = (if outgoing { line.target_id.as_str() } else { line.source_id.as_str() }, outgoing);
        match groups.iter_mut().find(|(k, _)| *k == key) {
            Some((_, members)) => members.push(c),
            None => groups.push((key, vec![c])),
        }
    }
    groups.into_iter().filter(|(_, members)| members.len() > 3).map(|(_, members)| members).collect()
}

/// The ELK box for a block of `members`, ortho's grid of them recorded.
fn bundle_node<'a>(members: &[&'a GraphNode], conversion: &mut Conversion<'a>) -> Value {
    let sized: Vec<(&'a GraphNode, f64, f64, Grid<'a>)> = members
        .iter()
        .map(|m| {
            let (w, h, cells) = plain_size(m);
            (*m, w, h, cells)
        })
        .collect();
    let line_of = |m: &GraphNode| conversion.edges.iter().find(|e| e.source_id == m.id || e.target_id == m.id).unwrap();
    let outgoing = line_of(members[0]).source_id == members[0].id;
    let count = sized.len();
    // The block's measures with this many columns.
    let measure = |columns: usize| {
        let rows = count.div_ceil(columns);
        let gutter = GRID_GAP_X.max(LANE * rows as f64 + 12.0);
        let widths: Vec<f64> =
            (0..columns).map(|c| sized.iter().skip(c).step_by(columns).fold(0.0, |m, s| f64::max(m, s.1))).collect();
        let heights: Vec<f64> = (0..rows)
            .map(|r| sized.iter().skip(r * columns).take(columns).fold(0.0, |m, s| f64::max(m, s.2)))
            .collect();
        let width = before(&widths, columns, gutter);
        let height = before(&heights, rows, BUNDLE_GAP_Y) - BUNDLE_GAP_Y;
        (rows, gutter, widths, heights, width, height)
    };
    // As many columns as bring the block nearest a shape a half wider than the page,
    // landscape A4: the block shares the page with the box its lines join, a layer
    // below or above it.
    let off_page = |columns: usize| {
        let (_, _, _, _, width, height) = measure(columns);
        (width / height / (1.5 * PAGE_WIDTH / PAGE_HEIGHT)).ln().abs()
    };
    let columns = (1..=count).min_by(|a, b| off_page(*a).total_cmp(&off_page(*b))).unwrap_or(1);
    let (rows, gutter, widths, heights, width, height) = measure(columns);
    let id = format!("{BUNDLE}{}", members[0].id);
    let mut bundle = Bundle { members: Vec::new(), routes: HashMap::new() };
    let mut ports = Vec::new();
    for (i, (m, w, h, cells)) in sized.into_iter().enumerate() {
        let (column, row) = (i % columns, i / columns);
        let x = before(&widths, column, gutter) + (widths[column] - w) / 2.0;
        let row_top = before(&heights, row, BUNDLE_GAP_Y);
        let y = row_top + (heights[row] - h) / 2.0;
        let middle = x + w / 2.0;
        let lane = |k: usize| before(&widths, column, gutter) + widths[column] + LANE * k as f64;
        let route = if outgoing {
            let below = i + columns < count;
            if below {
                let turn = row_top + heights[row] + BUNDLE_GAP_Y / 2.0;
                let at = lane(rows - 1 - row);
                vec![
                    Point { x: middle, y: y + h },
                    Point { x: middle, y: turn },
                    Point { x: at, y: turn },
                    Point { x: at, y: height },
                ]
            } else {
                vec![Point { x: middle, y: y + h }, Point { x: middle, y: height }]
            }
        } else if row > 0 {
            let turn = row_top - BUNDLE_GAP_Y / 2.0;
            let at = lane(row);
            vec![
                Point { x: at, y: 0.0 },
                Point { x: at, y: turn },
                Point { x: middle, y: turn },
                Point { x: middle, y },
            ]
        } else {
            vec![Point { x: middle, y: 0.0 }, Point { x: middle, y }]
        };
        let port = if outgoing { route[route.len() - 1] } else { route[0] };
        let line = line_of(m);
        let port_id = format!("{id}:{i}");
        ports.push(json!({
            "id": port_id,
            "x": port.x,
            "y": port.y,
            "width": 0.0,
            "height": 0.0,
            "layoutOptions": { "elk.port.side": if outgoing { "SOUTH" } else { "NORTH" } },
        }));
        conversion.ports_of.insert(&line.id, (port_id, outgoing));
        bundle.routes.insert(&line.id, route);
        if !cells.is_empty() {
            conversion.grids.as_mut().unwrap().insert(&m.id, cells);
        }
        bundle.members.push(Cell { node: m, x, y, width: w, height: h });
    }
    conversion.bundles.insert(id.clone(), bundle);
    json!({
        "id": id,
        "width": width,
        "height": height,
        "ports": ports,
        "layoutOptions": { "elk.portConstraints": "FIXED_POS" },
    })
}

/// The ELK boxes for `children`, a block in place of the first of its members.
fn elk_children<'a>(children: &'a [GraphNode], conversion: &mut Conversion<'a>) -> Vec<Value> {
    let bundles = bundles_among(children, conversion);
    let mut out = Vec::new();
    for child in children {
        match bundles.iter().find(|b| b.iter().any(|m| m.id == child.id)) {
            Some(bundle) if bundle[0].id == child.id => out.push(bundle_node(bundle, conversion)),
            Some(_) => {}
            None => out.push(to_elk_node(child, conversion)),
        }
    }
    out
}

fn to_elk_node<'a>(n: &'a GraphNode, conversion: &mut Conversion<'a>) -> Value {
    let packing = conversion.packing;
    let mut node = Map::new();
    node.insert("id".into(), json!(n.id));
    let mut layout_options = options(&[("elk.portConstraints", "FREE".into())]);
    let children = n.children.as_deref().unwrap_or(&[]);
    let gridded = conversion.grids.is_some() && !children.is_empty() && closed(n, &conversion.endpoints);
    if gridded {
        // ELK sees a plain box of the grid's size; flattening fills it in.
        let (cells, width, height) = grid_of(n, children);
        conversion.grids.as_mut().unwrap().insert(&n.id, cells);
        node.insert("width".into(), json!(width));
        node.insert("height".into(), json!(height));
    } else if !children.is_empty() {
        let elk_children = elk_children(children, conversion);
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
        if !packing {
            layout_options.extend(options(&[
                ("org.eclipse.elk.alg.layered.considerModelOrder.strategy", "NODES_AND_EDGES".into()),
                ("org.eclipse.elk.alg.layered.considerModelOrder.components", "MODEL_ORDER".into()),
            ]));
        }
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
        edges: &graph.edges,
        bundles: HashMap::new(),
        ports_of: HashMap::new(),
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
            "org.eclipse.elk.alg.layered.considerModelOrder.strategy": "NODES_AND_EDGES",
            "org.eclipse.elk.alg.layered.considerModelOrder.components": "MODEL_ORDER",
        })
    };
    let children = elk_children(&graph.nodes, &mut conversion);
    let edges: Vec<Value> = graph
        .edges
        .iter()
        .map(|e| {
            let mut source = e.source_port_id.as_ref().unwrap_or(&e.source_id);
            let mut target = e.target_port_id.as_ref().unwrap_or(&e.target_id);
            if let Some((port, outgoing)) = conversion.ports_of.get(e.id.as_str()) {
                if *outgoing { source = port } else { target = port }
            }
            let mut edge = json!({ "id": e.id, "sources": [source], "targets": [target] });
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

/// A box ortho placed itself, at (x, y).
fn cell_node(cell: &Cell, x: f64, y: f64) -> LaidOutNode {
    let n = cell.node;
    LaidOutNode {
        id: n.id.clone(),
        shape: n.shape,
        keyword: n.keyword.clone(),
        name: n.name.clone(),
        compartments: n.compartments.clone(),
        ports: Vec::new(),
        has_children: n.children.as_ref().is_some_and(|inner| !inner.is_empty()),
        x,
        y,
        width: cell.width,
        height: cell.height,
        element: n.element,
        source: None,
    }
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
        placed: &mut HashMap<String, (f64, f64)>,
    ) {
        for c in children {
            let id = c["id"].as_str().unwrap_or("");
            let source = by_id.get(id);
            let x = number_or(c, "x", 0.0) + offset.0;
            let y = number_or(c, "y", 0.0) + offset.1;
            if let Some(bundle) = conversion.bundles.get(id) {
                placed.insert(id.to_string(), (x, y));
                for member in &bundle.members {
                    let (mx, my) = (x + member.x, y + member.y);
                    nodes.push(cell_node(member, mx, my));
                    let cells = conversion.grids.as_ref().and_then(|grids| grids.get(member.node.id.as_str()));
                    nodes.extend(cells.into_iter().flatten().map(|cell| cell_node(cell, mx + cell.x, my + cell.y)));
                }
                continue;
            }
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
                element: source.and_then(|s| s.element),
                source: None,
            });
            let cells = conversion.grids.as_ref().and_then(|grids| grids.get(id));
            nodes.extend(cells.into_iter().flatten().map(|cell| cell_node(cell, x + cell.x, y + cell.y)));
            flatten(&list(c, "children"), (x, y), by_id, conversion, nodes, placed);
        }
    }
    let mut placed = HashMap::new();
    flatten(&list(result, "children"), (0.0, 0.0), &by_id, conversion, &mut nodes, &mut placed);

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
            // A line through a block's port runs on inside it, to or from its member.
            let points = match conversion.ports_of.get(id) {
                Some((port, outgoing)) => {
                    let block = port.rsplit_once(':').map_or("", |(block, _)| block);
                    let (bx, by) = placed.get(block).copied().unwrap_or((0.0, 0.0));
                    let inside: Vec<Point> = conversion.bundles[block].routes[id]
                        .iter()
                        .map(|p| Point { x: p.x + bx, y: p.y + by })
                        .collect();
                    if *outgoing {
                        inside.into_iter().chain(points.into_iter().skip(1)).collect()
                    } else {
                        let keep = points.len().saturating_sub(1);
                        points.into_iter().take(keep).chain(inside).collect()
                    }
                }
                None => points,
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
