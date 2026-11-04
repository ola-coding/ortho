//! The logical view's layout: a work-breakdown chart, without ELK.
use super::{LaidOutDiagram, LaidOutEdge, LaidOutNode, Point};
use crate::model::graph::{DiagramGraph, EdgeKind, GraphEdge, GraphNode, USAGE_RADIUS, is_use_case_node};
use crate::render::page::scale_to_page;
use std::collections::HashMap;

/// Clearance around the drawing; matches ELK's default root padding.
const MARGIN: f64 = 12.0;
/// Horizontal gap between sibling subtrees.
const SIBLING_GAP: f64 = 28.0;
/// Gap between a parent and the row of children below it; the bus runs at its middle.
const LEVEL_GAP: f64 = 44.0;
/// Gap between a parent and the first leaf listed below it.
const LIST_TOP_GAP: f64 = 14.0;
/// Vertical gap between listed leaves.
const LIST_GAP: f64 = 10.0;
/// Distance of a list's spine from its parent's left edge.
const SPINE_INSET: f64 = 18.0;
/// Distance from the spine to a listed leaf's left edge.
const BRANCH: f64 = 16.0;
/// How far a family wrapped onto several rows stands in from the trunk that feeds
/// them; the trunk runs down the middle of it.
const WRAP_INDENT: f64 = 24.0;
/// Each narrower width tried for a row, as a share of the one before.
const NARROWING: f64 = 0.9;
/// A wrapped layout must print a tenth bigger to be taken over a wider one: less
/// is not worth a tree that no longer reads in one sweep.
const WORTH_WRAPPING: f64 = 1.1;
/// The narrowest row tried.
const MIN_ROW: f64 = 150.0;

#[derive(Clone, Copy)]
struct Measure {
    width: f64,
    height: f64,
    /// Offset of the node's own box from the left edge of its subtree.
    box_left: f64,
    /// Offset of the children's row from the left edge of the subtree.
    row_left: f64,
    /// Children listed vertically beneath the node rather than spread in a row.
    listed: bool,
}

fn without_repeats(points: Vec<Point>) -> Vec<Point> {
    let mut kept: Vec<Point> = Vec::with_capacity(points.len());
    for p in points {
        if kept.last().is_none_or(|last| p.x != last.x || p.y != last.y) {
            kept.push(p);
        }
    }
    kept
}

fn laid_out(node: &GraphNode, x: f64, y: f64) -> LaidOutNode {
    LaidOutNode {
        id: node.id.clone(),
        shape: node.shape,
        keyword: node.keyword.clone(),
        name: node.name.clone(),
        compartments: node.compartments.clone(),
        ports: Vec::new(),
        has_children: false,
        x,
        y,
        width: node.width,
        height: node.height,
    }
}

fn edge_along(edge: &GraphEdge, points: Vec<Point>) -> LaidOutEdge {
    LaidOutEdge {
        id: edge.id.clone(),
        kind: edge.kind,
        source_id: edge.source_id.clone(),
        target_id: edge.target_id.clone(),
        label: edge.label.clone(),
        points,
        label_box: None,
    }
}

struct Tree<'a> {
    by_id: HashMap<&'a str, &'a GraphNode>,
    children: HashMap<&'a str, Vec<&'a str>>,
    tree_edge: HashMap<&'a str, &'a GraphEdge>,
    measures: HashMap<&'a str, Measure>,
    /// The widest a row of children may be before it wraps onto another row.
    max_row: f64,
    /// The number of children on each row, for a family wrapped onto several.
    wraps: HashMap<&'a str, Vec<usize>>,
    nodes: Vec<LaidOutNode>,
    edges: Vec<LaidOutEdge>,
    position: HashMap<&'a str, Point>,
}

impl<'a> Tree<'a> {
    fn kids(&self, id: &str) -> &[&'a str] {
        self.children.get(id).map_or(&[], Vec::as_slice)
    }

    fn node(&self, id: &str) -> &'a GraphNode {
        self.by_id[id]
    }

    fn measure(&mut self, id: &'a str) -> Measure {
        let node = self.node(id);
        let list: Vec<&'a str> = self.kids(id).to_vec();
        let m = if list.is_empty() {
            Measure { width: node.width, height: node.height, box_left: 0.0, row_left: 0.0, listed: false }
        } else if list.iter().all(|k| self.kids(k).is_empty()) {
            for k in &list {
                self.measure(k);
            }
            let leaves: Vec<&GraphNode> = list.iter().map(|k| self.node(k)).collect();
            let widest = leaves.iter().map(|l| l.width).fold(f64::NEG_INFINITY, f64::max);
            let list_width = SPINE_INSET + BRANCH + widest;
            let list_height = leaves.iter().fold(0.0, |sum, l| sum + l.height) + LIST_GAP * (leaves.len() as f64 - 1.0);
            Measure {
                width: node.width.max(list_width),
                height: node.height + LIST_TOP_GAP + list_height,
                box_left: 0.0,
                row_left: 0.0,
                listed: true,
            }
        } else {
            let row: Vec<Measure> = list.iter().map(|k| self.measure(k)).collect();
            let sizes = self.rows_of(&row);
            let wrapped = sizes.len() > 1;
            let stem_at = self.stem_offset(&list[..sizes[0]], &row[..sizes[0]]);
            let stem_at = if wrapped { WRAP_INDENT + stem_at } else { stem_at };
            let mut box_left = stem_at - node.width / 2.0;
            let mut row_left = 0.0;
            if box_left < 0.0 {
                row_left = -box_left;
                box_left = 0.0;
            }
            let mut rows_width: f64 = 0.0;
            let mut height = node.height;
            let mut start = 0;
            for size in &sizes {
                let chunk = &row[start..start + size];
                rows_width = rows_width.max(row_width(chunk));
                height = height + LEVEL_GAP + chunk.iter().map(|r| r.height).fold(f64::NEG_INFINITY, f64::max);
                start += size;
            }
            let rows_width = if wrapped { WRAP_INDENT + rows_width } else { rows_width };
            if wrapped {
                self.wraps.insert(id, sizes);
            }
            Measure {
                width: (row_left + rows_width).max(box_left + node.width),
                height,
                box_left,
                row_left,
                listed: false,
            }
        };
        self.measures.insert(id, m);
        m
    }

    /// Where the parent's stem meets a row of its children, from the row's left.
    /// An odd row has a middle child, and the stem drops straight onto it:
    /// centring on the row instead is off by however much the box widths differ,
    /// which reads as a wobble. An even row has no middle child, so the parent
    /// stays over the middle of the row.
    fn stem_offset(&self, list: &[&'a str], row: &[Measure]) -> f64 {
        if row.len() % 2 == 1 {
            let middle = (row.len() - 1) / 2;
            row[..middle].iter().fold(0.0, |sum, r| sum + r.width + SIBLING_GAP)
                + row[middle].box_left
                + self.node(list[middle]).width / 2.0
        } else {
            let first = self.node(list[0]);
            let last = self.node(list[list.len() - 1]);
            let first_centre = row[0].box_left + first.width / 2.0;
            let end = row[row.len() - 1];
            let last_centre = row_width(row) - end.width + end.box_left + last.width / 2.0;
            (first_centre + last_centre) / 2.0
        }
    }

    /// How many children go on each row: all on one while they fit `max_row`, else
    /// as few rows as fit, the children shared out evenly and the earlier rows
    /// taking any one over.
    fn rows_of(&self, row: &[Measure]) -> Vec<usize> {
        let n = row.len();
        if row_width(row) <= self.max_row {
            return vec![n];
        }
        for count in 2..=n {
            let sizes: Vec<usize> = (0..count).map(|i| n / count + usize::from(i < n % count)).collect();
            let mut start = 0;
            let fits = sizes.iter().all(|size| {
                let chunk = &row[start..start + size];
                start += size;
                WRAP_INDENT + row_width(chunk) <= self.max_row
            });
            if fits {
                return sizes;
            }
        }
        vec![1; n]
    }

    fn put(&mut self, node: &'a GraphNode, x: f64, y: f64) {
        self.nodes.push(laid_out(node, x, y));
        self.position.insert(&node.id, Point { x, y });
    }

    fn branch(&mut self, child_id: &str, points: Vec<Point>) {
        let edge = self.tree_edge[child_id];
        self.edges.push(edge_along(edge, without_repeats(points)));
    }

    fn place(&mut self, id: &'a str, left: f64, top: f64) {
        let node = self.node(id);
        let m = self.measures[id];
        let x = left + m.box_left;
        self.put(node, x, top);

        let bottom = top + node.height;
        let list: Vec<&'a str> = self.kids(id).to_vec();
        if m.listed {
            let spine_x = x + SPINE_INSET;
            let mut y = bottom + LIST_TOP_GAP;
            for leaf_id in list {
                let leaf = self.node(leaf_id);
                self.place(leaf_id, spine_x + BRANCH, y);
                let middle = y + leaf.height / 2.0;
                let points = vec![
                    Point { x: spine_x, y: bottom },
                    Point { x: spine_x, y: middle },
                    Point { x: spine_x + BRANCH, y: middle },
                ];
                self.branch(leaf_id, points);
                y += leaf.height + LIST_GAP;
            }
        } else if let Some(sizes) = self.wraps.get(id).cloned() {
            self.place_rows(id, &list, &sizes, left, top);
        } else if !list.is_empty() {
            let bus_y = bottom + LEVEL_GAP / 2.0;
            let row_top = bottom + LEVEL_GAP;
            let mut lefts = Vec::with_capacity(list.len());
            let mut child_left = left + m.row_left;
            for child_id in &list {
                lefts.push(child_left);
                child_left += self.measures[child_id].width + SIBLING_GAP;
            }
            let centre_of =
                |tree: &Self, i: usize| lefts[i] + tree.measures[list[i]].box_left + tree.node(list[i]).width / 2.0;
            // Over an odd row the stem is taken from the middle child itself, not
            // from the parent's centre, so the two meet exactly rather than to within
            // rounding, and the middle branch is one straight line.
            let stem_x = if list.len() % 2 == 1 { centre_of(self, (list.len() - 1) / 2) } else { x + node.width / 2.0 };
            for (i, child_id) in list.iter().enumerate() {
                self.place(child_id, lefts[i], row_top);
                let child_x = centre_of(self, i);
                let points = vec![
                    Point { x: stem_x, y: bottom },
                    Point { x: stem_x, y: bus_y },
                    Point { x: child_x, y: bus_y },
                    Point { x: child_x, y: row_top },
                ];
                self.branch(child_id, points);
            }
        }
    }

    /// A family wrapped onto several rows. The stem drops from the parent to the
    /// first row's bus, as for one row; the bus runs on left to a trunk, down which
    /// each later row hangs off a bus of its own.
    fn place_rows(&mut self, id: &'a str, list: &[&'a str], sizes: &[usize], left: f64, top: f64) {
        let node = self.node(id);
        let m = self.measures[id];
        let x = left + m.box_left;
        let bottom = top + node.height;
        let trunk_x = left + m.row_left + WRAP_INDENT / 2.0;
        let rows_left = left + m.row_left + WRAP_INDENT;
        let first_bus = bottom + LEVEL_GAP / 2.0;
        let mut stem_x = x + node.width / 2.0;
        let mut row_top = bottom + LEVEL_GAP;
        let mut start = 0;
        for (k, size) in sizes.iter().enumerate() {
            let chunk = &list[start..start + size];
            start += size;
            let bus_y = row_top - LEVEL_GAP / 2.0;
            let mut lefts = Vec::with_capacity(chunk.len());
            let mut child_left = rows_left;
            for child_id in chunk {
                lefts.push(child_left);
                child_left += self.measures[child_id].width + SIBLING_GAP;
            }
            let centre_of =
                |tree: &Self, i: usize| lefts[i] + tree.measures[chunk[i]].box_left + tree.node(chunk[i]).width / 2.0;
            if k == 0 && chunk.len() % 2 == 1 {
                stem_x = centre_of(self, (chunk.len() - 1) / 2);
            }
            let tallest = chunk.iter().map(|c| self.measures[c].height).fold(f64::NEG_INFINITY, f64::max);
            for (i, child_id) in chunk.iter().enumerate() {
                self.place(child_id, lefts[i], row_top);
                let child_x = centre_of(self, i);
                let mut points = vec![Point { x: stem_x, y: bottom }, Point { x: stem_x, y: first_bus }];
                if k > 0 {
                    points.push(Point { x: trunk_x, y: first_bus });
                    points.push(Point { x: trunk_x, y: bus_y });
                }
                points.push(Point { x: child_x, y: bus_y });
                points.push(Point { x: child_x, y: row_top });
                self.branch(child_id, points);
            }
            row_top += tallest + LEVEL_GAP;
        }
    }
}

/// The width of children side by side.
fn row_width(row: &[Measure]) -> f64 {
    row.iter().fold(0.0, |sum, r| sum + r.width) + SIBLING_GAP * (row.len() as f64 - 1.0)
}

/// Lays out a capability tree as a work-breakdown chart, deterministically and
/// without ELK. A family whose children are all leaves is listed vertically beneath
/// its parent, hanging off a spine. Any other family is spread in a row, with a
/// single stem dropping to a bus that feeds every child. The parent stands over its
/// middle child when the row has one, and over the middle of the row when it does
/// not. Listing the lowest level is what keeps a broad tree page-shaped: a row
/// holding every leaf is a strip. A tree still too big for landscape A4 has its
/// widest rows wrapped onto several, at whichever width prints it biggest there.
///
/// A use case the view exposes stands in a row above the trees, over the functions
/// that depend on it, as OMG's course draws a dependency used for traceability: the
/// client below, the supplier above, the arrow pointing up.
///
/// The graph is expected to be a forest. Any other edge, one that would give a
/// function a second parent, close a cycle, or be a dependency, is drawn as a
/// straight line but takes no part in the layout.
pub fn layout_tree(graph: &DiagramGraph) -> LaidOutDiagram {
    let mut best = layout_within(graph, f64::INFINITY);
    let mut best_scale = scale_to_page(best.width, best.height);
    if best_scale >= 1.0 {
        return best;
    }
    // Narrower and narrower rows, keeping the layout that prints biggest; a wider
    // one wins a tie, since it wraps less.
    let mut max_row = best.width * NARROWING;
    while max_row > MIN_ROW {
        let laid_out = layout_within(graph, max_row);
        let scale = scale_to_page(laid_out.width, laid_out.height);
        if scale > best_scale * WORTH_WRAPPING {
            best = laid_out;
            best_scale = scale;
        }
        max_row *= NARROWING;
    }
    best
}

/// The tree laid out with no row of children wider than `max_row` where it can wrap.
fn layout_within(graph: &DiagramGraph, max_row: f64) -> LaidOutDiagram {
    let by_id: HashMap<&str, &GraphNode> = graph.nodes.iter().map(|n| (n.id.as_str(), n)).collect();
    let use_cases: Vec<&GraphNode> = graph.nodes.iter().filter(|n| is_use_case_node(&n.keyword)).collect();
    let mut tree = Tree {
        by_id,
        children: HashMap::new(),
        tree_edge: HashMap::new(),
        measures: HashMap::new(),
        max_row,
        wraps: HashMap::new(),
        nodes: Vec::new(),
        edges: Vec::new(),
        position: HashMap::new(),
    };
    let mut parent_of: HashMap<&str, &str> = HashMap::new();
    let mut extra: Vec<&GraphEdge> = Vec::new();

    for edge in &graph.edges {
        let (parent, child) = (edge.source_id.as_str(), edge.target_id.as_str());
        let closes_cycle = || {
            let mut p = Some(parent);
            while let Some(at) = p {
                if at == child {
                    return true;
                }
                p = parent_of.get(at).copied();
            }
            false
        };
        if edge.kind != EdgeKind::Decomposition
            || !tree.by_id.contains_key(parent)
            || !tree.by_id.contains_key(child)
            || parent_of.contains_key(child)
            || closes_cycle()
        {
            extra.push(edge);
            continue;
        }
        parent_of.insert(child, parent);
        tree.children.entry(parent).or_default().push(child);
        tree.tree_edge.insert(child, edge);
    }

    let roots: Vec<&str> = graph
        .nodes
        .iter()
        .filter(|n| !parent_of.contains_key(n.id.as_str()) && !is_use_case_node(&n.keyword))
        .map(|n| n.id.as_str())
        .collect();
    let use_case_height = use_cases.iter().map(|u| u.height).fold(0.0, f64::max);
    let trees_top = MARGIN + if use_cases.is_empty() { 0.0 } else { use_case_height + LEVEL_GAP };
    let mut left = MARGIN;
    let mut height: f64 = 0.0;
    for root in &roots {
        let m = tree.measure(root);
        tree.place(root, left, trees_top);
        left += m.width + SIBLING_GAP * 2.0;
        height = height.max(m.height);
    }

    // Each use case centred over the functions that depend on it, as far as the one
    // before it allows; one nothing depends on goes last, a visible gap.
    let middle: HashMap<&str, f64> = use_cases
        .iter()
        .map(|use_case| {
            let clients: Vec<f64> = extra
                .iter()
                .filter(|e| e.target_id == use_case.id)
                .filter_map(|e| {
                    let at = tree.position.get(e.source_id.as_str())?;
                    Some(at.x + tree.node(&e.source_id).width / 2.0)
                })
                .collect();
            let centre = if clients.is_empty() {
                f64::INFINITY
            } else {
                clients.iter().fold(0.0, |sum, x| sum + x) / clients.len() as f64
            };
            (use_case.id.as_str(), centre)
        })
        .collect();
    let mut right = MARGIN - SIBLING_GAP;
    let mut ordered = use_cases.clone();
    // As JavaScript sorts by a difference: two that nothing depends on are equal.
    ordered.sort_by(|a, b| {
        let difference = middle[a.id.as_str()] - middle[b.id.as_str()];
        difference.partial_cmp(&0.0).unwrap_or(std::cmp::Ordering::Equal)
    });
    for use_case in ordered {
        let centred = middle[use_case.id.as_str()] - use_case.width / 2.0;
        let x = (if centred.is_finite() { centred } else { f64::NEG_INFINITY }).max(right + SIBLING_GAP);
        tree.put(use_case, x, MARGIN + use_case_height - use_case.height);
        right = x + use_case.width;
    }

    for edge in extra {
        let (Some(s), Some(t)) =
            (tree.position.get(edge.source_id.as_str()).copied(), tree.position.get(edge.target_id.as_str()).copied())
        else {
            continue;
        };
        let source = tree.node(&edge.source_id);
        let target = tree.node(&edge.target_id);
        // Up from a function to a use case, straight where the two overlap; any
        // other line from the source's bottom to the target's top.
        let source_x = s.x + source.width / 2.0;
        let points = if is_use_case_node(&target.keyword) && !is_use_case_node(&source.keyword) {
            let along = source_x.min(t.x + target.width - USAGE_RADIUS).max(t.x + USAGE_RADIUS);
            vec![Point { x: source_x, y: s.y }, Point { x: along, y: t.y + target.height }]
        } else {
            vec![Point { x: source_x, y: s.y + source.height }, Point { x: t.x + target.width / 2.0, y: t.y }]
        };
        tree.edges.push(edge_along(edge, points));
    }

    let trees_right = if roots.is_empty() { 2.0 * MARGIN } else { left - SIBLING_GAP * 2.0 + MARGIN };
    LaidOutDiagram {
        width: trees_right.max(right + MARGIN),
        height: trees_top + height + MARGIN,
        nodes: tree.nodes,
        edges: tree.edges,
    }
}
