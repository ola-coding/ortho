//! The use case view's layout, without ELK.
//!
//! Each use case is drawn as SysML v2 draws one (Part 1, Table 23 and §8.2.3.25): its
//! «subject» above it, its actors beside it, primary ones on the left and secondary
//! ones on the right, and the use cases it includes below it, each joined by an
//! «include» arrow. The subject and the actors are the use case's own parameters, so
//! every such group carries its own; a view of several use cases sets the groups out
//! in rows, in the order the model declares them, as many to a row as landscape A4
//! takes.
use super::{LaidOutDiagram, LaidOutEdge, LaidOutNode, Point};
use crate::model::graph::{DiagramGraph, EdgeKind, GraphEdge, GraphNode, NodeShape, USAGE_RADIUS, is_use_case_node};
use crate::render::page::PAGE_WIDTH;
use std::collections::{HashMap, HashSet};

/// Clearance around the drawing; matches ELK's default root padding.
const MARGIN: f64 = 12.0;
/// Between the subject and its use case.
const SUBJECT_GAP: f64 = 48.0;
/// Between a use case and the use cases it includes: room for «include».
const INCLUDE_GAP: f64 = 64.0;
/// Between use cases side by side under the one including them.
const BOX_GAP: f64 = 32.0;
/// Between a use case and the actors beside it.
const SIDE_GAP: f64 = 56.0;
/// Between actors stacked on the same side of a use case.
const ACTOR_GAP: f64 = 12.0;
/// Between groups, side by side and row above row.
const GROUP_GAP: f64 = 56.0;
/// The widest a row of groups may be: landscape A4, less the frame around the diagram.
const ROW_WIDTH: f64 = PAGE_WIDTH - 40.0;
/// Where an actor's arms are, below the top of its figure, and how tall the figure is.
const ARMS: f64 = 24.0;
const ACTOR_HEIGHT: f64 = 76.0;
/// A line to an actor stops this far from its hand.
const HAND_GAP: f64 = 3.0;
/// How far an actor's arm reaches either side of its middle.
const ARM_REACH: f64 = 13.0;

fn centre_x(n: &LaidOutNode) -> f64 {
    n.x + n.width / 2.0
}

fn clamp(v: f64, lo: f64, hi: f64) -> f64 {
    lo.max(hi.min(v))
}

fn laid_out(n: &GraphNode, id: String, x: f64, y: f64) -> LaidOutNode {
    LaidOutNode {
        id,
        shape: n.shape,
        keyword: n.keyword.clone(),
        name: n.name.clone(),
        compartments: n.compartments.clone(),
        ports: Vec::new(),
        has_children: false,
        x,
        y,
        width: n.width,
        height: n.height,
    }
}

/// A use case, the actors drawn beside it, and the use cases it includes below it.
struct Tree<'g> {
    use_case: &'g GraphNode,
    /// The actor and the line joining it, as the graph has it.
    left: Vec<(&'g GraphNode, &'g GraphEdge)>,
    right: Vec<(&'g GraphNode, &'g GraphEdge)>,
    includes: Vec<(Tree<'g>, &'g GraphEdge)>,
}

impl Tree<'_> {
    fn column_width(actors: &[(&GraphNode, &GraphEdge)]) -> f64 {
        actors.iter().map(|(a, _)| a.width).fold(0.0, f64::max)
    }

    /// How far a stack of actors reaches above and below the middle of the use case
    /// beside it, its arms centred on that middle.
    fn stack_reach(n: usize) -> (f64, f64) {
        if n == 0 {
            return (0.0, 0.0);
        }
        let half = (n - 1) as f64 / 2.0 * (ACTOR_HEIGHT + ACTOR_GAP);
        (half + ARMS, half + ACTOR_HEIGHT - ARMS)
    }

    /// The left and right reach of the use case's own row, from its middle.
    fn header(&self) -> (f64, f64) {
        let side = |actors: &[(&GraphNode, &GraphEdge)]| {
            if actors.is_empty() { 0.0 } else { SIDE_GAP + Self::column_width(actors) }
        };
        let half = self.use_case.width / 2.0;
        (half + side(&self.left), half + side(&self.right))
    }

    /// The reach above and below the use case's middle of its own row.
    fn header_height(&self) -> (f64, f64) {
        let half = self.use_case.height / 2.0;
        let (l_up, l_down) = Self::stack_reach(self.left.len());
        let (r_up, r_down) = Self::stack_reach(self.right.len());
        (half.max(l_up).max(r_up), half.max(l_down).max(r_down))
    }

    fn width(&self) -> f64 {
        let (left, right) = self.header();
        (left + right).max(self.includes_width())
    }

    fn includes_width(&self) -> f64 {
        let n = self.includes.len();
        self.includes.iter().map(|(t, _)| t.width()).sum::<f64>() + BOX_GAP * n.saturating_sub(1) as f64
    }

    /// Places the tree with its top left corner at (x, y), its use case centred on
    /// the tree's width.
    fn place(&self, x: f64, y: f64, out: &mut Group) {
        let width = self.width();
        let (left, right) = self.header();
        let (up, down) = self.header_height();
        let middle_x = x + (width - left - right) / 2.0 + left;
        let middle_y = y + up;
        let u = self.use_case;
        out.nodes.push(laid_out(u, u.id.clone(), middle_x - u.width / 2.0, middle_y - u.height / 2.0));
        let use_case = out.nodes.len() - 1;

        for (actors, on_left) in [(&self.left, true), (&self.right, false)] {
            let column = Self::column_width(actors);
            let (reach, _) = Self::stack_reach(actors.len());
            for (i, (actor, line)) in actors.iter().enumerate() {
                let column_x = if on_left {
                    middle_x - u.width / 2.0 - SIDE_GAP - column
                } else {
                    middle_x + u.width / 2.0 + SIDE_GAP
                };
                let top = middle_y - reach + i as f64 * (ACTOR_HEIGHT + ACTOR_GAP);
                let id = format!("{}@{}", actor.id, u.id);
                out.nodes.push(laid_out(actor, id.clone(), column_x + (column - actor.width) / 2.0, top));
                let figure = out.nodes.last().unwrap();
                let use_case = &out.nodes[use_case];
                let hand_x = centre_x(figure) + if on_left { ARM_REACH + HAND_GAP } else { -ARM_REACH - HAND_GAP };
                let hand = Point { x: hand_x, y: top + ARMS };
                let side = Point {
                    x: if on_left { use_case.x } else { use_case.x + use_case.width },
                    y: clamp(hand.y, use_case.y + USAGE_RADIUS, use_case.y + use_case.height - USAGE_RADIUS),
                };
                // In the line's own direction, from its source to its target.
                let points = if line.source_id == actor.id { vec![hand, side] } else { vec![side, hand] };
                let (source, target) = if line.source_id == actor.id { (id, u.id.clone()) } else { (u.id.clone(), id) };
                out.edge(line, source, target, points);
            }
        }

        let below = middle_y + down + INCLUDE_GAP;
        let mut child_x = x + (width - self.includes_width()) / 2.0;
        for (tree, include) in &self.includes {
            let child_middle = child_x + tree.width() / 2.0;
            tree.place(child_x, below, out);
            let parent = &out.nodes[use_case];
            let child = out.nodes.iter().find(|n| n.id == tree.use_case.id).unwrap();
            let child_top_x = clamp(centre_x(parent), child.x + USAGE_RADIUS, child.x + child.width - USAGE_RADIUS);
            let start_x = clamp(child_middle, parent.x + USAGE_RADIUS, parent.x + parent.width - USAGE_RADIUS);
            let points = vec![Point { x: start_x, y: parent.y + parent.height }, Point { x: child_top_x, y: child.y }];
            out.edge(include, u.id.clone(), tree.use_case.id.clone(), points);
            child_x += tree.width() + BOX_GAP;
        }
    }
}

/// One use case drawn as the spec draws it, in coordinates of its own.
#[derive(Default)]
struct Group {
    nodes: Vec<LaidOutNode>,
    edges: Vec<LaidOutEdge>,
}

impl Group {
    fn edge(&mut self, e: &GraphEdge, source_id: String, target_id: String, points: Vec<Point>) {
        self.edges.push(LaidOutEdge {
            id: format!("{}@{}", e.id, self.edges.len()),
            kind: e.kind,
            source_id,
            target_id,
            label: e.label.clone(),
            points,
            label_box: None,
        });
    }

    fn bounds(&self) -> (f64, f64, f64, f64) {
        let left = self.nodes.iter().map(|n| n.x).fold(f64::INFINITY, f64::min);
        let top = self.nodes.iter().map(|n| n.y).fold(f64::INFINITY, f64::min);
        let right = self.nodes.iter().map(|n| n.x + n.width).fold(f64::NEG_INFINITY, f64::max);
        let bottom = self.nodes.iter().map(|n| n.y + n.height).fold(f64::NEG_INFINITY, f64::max);
        (left, top, right, bottom)
    }

    fn shift(&mut self, dx: f64, dy: f64) {
        for n in &mut self.nodes {
            n.x += dx;
            n.y += dy;
        }
        for e in &mut self.edges {
            for p in &mut e.points {
                p.x += dx;
                p.y += dy;
            }
        }
    }
}

/// Lays out the use case view as the spec draws a use case, one group per use case
/// no other includes, in rows.
pub fn layout_use_cases(graph: &DiagramGraph) -> LaidOutDiagram {
    let by_id: HashMap<&str, &GraphNode> = graph.nodes.iter().map(|n| (n.id.as_str(), n)).collect();
    let use_cases: Vec<&GraphNode> = graph.nodes.iter().filter(|n| is_use_case_node(&n.keyword)).collect();
    let includes: Vec<&GraphEdge> = graph.edges.iter().filter(|e| e.kind == EdgeKind::Include).collect();
    let lines: Vec<&GraphEdge> = graph.edges.iter().filter(|e| e.kind == EdgeKind::Association).collect();
    let included: HashSet<&str> = includes.iter().map(|e| e.target_id.as_str()).collect();

    // A use case with the actors it is the first in its group to name, and below it
    // the use cases it includes that are not drawn yet.
    fn grow<'g>(
        use_case: &'g GraphNode,
        by_id: &HashMap<&str, &'g GraphNode>,
        includes: &[&'g GraphEdge],
        lines: &[&'g GraphEdge],
        placed: &mut HashSet<&'g str>,
        named: &mut HashSet<&'g str>,
    ) -> Tree<'g> {
        placed.insert(&use_case.id);
        let mut tree = Tree { use_case, left: Vec::new(), right: Vec::new(), includes: Vec::new() };
        for line in lines {
            // A primary actor's line starts at the actor, a secondary one's at the use case.
            let (actor, primary) = if line.target_id == use_case.id {
                (line.source_id.as_str(), true)
            } else if line.source_id == use_case.id {
                (line.target_id.as_str(), false)
            } else {
                continue;
            };
            let actor = by_id[actor];
            if actor.shape != NodeShape::Actor || !named.insert(&actor.id) {
                continue;
            }
            if primary { tree.left.push((actor, line)) } else { tree.right.push((actor, line)) }
        }
        let targets: Vec<&&'g GraphEdge> =
            includes.iter().filter(|e| e.source_id == use_case.id && !placed.contains(e.target_id.as_str())).collect();
        for include in targets {
            let target = by_id[include.target_id.as_str()];
            if !placed.contains(target.id.as_str()) {
                let child = grow(target, by_id, includes, lines, placed, named);
                tree.includes.push((child, include));
            }
        }
        tree
    }

    // Groups in the order of their first use case, those no other includes first.
    let mut placed: HashSet<&str> = HashSet::new();
    let mut groups: Vec<Group> = Vec::new();
    let roots = use_cases.iter().filter(|u| !included.contains(u.id.as_str())).chain(&use_cases);
    for root in roots {
        if placed.contains(root.id.as_str()) {
            continue;
        }
        let mut named = HashSet::new();
        let tree = grow(root, &by_id, &includes, &lines, &mut placed, &mut named);
        let mut group = Group::default();
        let subject = lines
            .iter()
            .find(|e| e.source_id == root.id && by_id[e.target_id.as_str()].keyword == "subject")
            .map(|e| (by_id[e.target_id.as_str()], *e));
        let top = subject.map_or(0.0, |(s, _)| s.height + SUBJECT_GAP);
        tree.place(0.0, top, &mut group);
        if let Some((s, line)) = subject {
            let (middle, use_case_top) = (centre_x(&group.nodes[0]), group.nodes[0].y);
            let x = middle - s.width / 2.0;
            let id = format!("{}@{}", s.id, root.id);
            group.nodes.push(laid_out(s, id.clone(), x, 0.0));
            let at = clamp(middle, x + USAGE_RADIUS, x + s.width - USAGE_RADIUS);
            let points = vec![Point { x: at, y: use_case_top }, Point { x: at, y: s.height }];
            group.edge(line, root.id.clone(), id, points);
        }
        groups.push(group);
    }
    // Groups take the declared order of their use case, whether or not another includes it.
    let order: HashMap<&str, usize> = use_cases.iter().enumerate().map(|(i, u)| (u.id.as_str(), i)).collect();
    let first = |g: &Group| g.nodes.iter().filter_map(|n| order.get(n.id.as_str())).min().copied();
    groups.sort_by_key(|g| first(g));

    if groups.is_empty() {
        return LaidOutDiagram { width: 2.0 * MARGIN, height: 2.0 * MARGIN, nodes: Vec::new(), edges: Vec::new() };
    }

    // Rows, as many groups to a row as the page takes, each group at the top of its row.
    let (mut x, mut y, mut row_bottom) = (MARGIN, MARGIN, MARGIN);
    let mut nodes = Vec::new();
    let mut edges = Vec::new();
    for mut group in groups {
        let (left, top, right, bottom) = group.bounds();
        if x > MARGIN && x + right - left > ROW_WIDTH {
            x = MARGIN;
            y = row_bottom + GROUP_GAP;
        }
        group.shift(x - left, y - top);
        x += right - left + GROUP_GAP;
        row_bottom = row_bottom.max(y + bottom - top);
        nodes.extend(group.nodes);
        edges.extend(group.edges);
    }

    LaidOutDiagram {
        width: nodes.iter().map(|n| n.x + n.width).fold(f64::NEG_INFINITY, f64::max) + MARGIN,
        height: nodes.iter().map(|n| n.y + n.height).fold(f64::NEG_INFINITY, f64::max) + MARGIN,
        nodes,
        edges,
    }
}
