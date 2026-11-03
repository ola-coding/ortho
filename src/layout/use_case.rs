//! The use case view's layout, without ELK.
use super::{LaidOutDiagram, LaidOutEdge, LaidOutNode, Point};
use crate::model::graph::{DiagramGraph, EdgeKind, GraphEdge, GraphNode, NodeShape, USAGE_RADIUS, is_use_case_node};
use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};

/// Clearance around the drawing; matches ELK's default root padding.
const MARGIN: f64 = 12.0;
/// Between the subject and the use cases, and between the use cases and the actors.
const ROW_GAP: f64 = 56.0;
/// Between two use cases side by side.
const BOX_GAP: f64 = 36.0;
/// Between a use case and one it includes: room for «include» and its arrow.
const INCLUDE_GAP: f64 = 84.0;
/// Between subjects, and between actors, side by side.
const SIDE_GAP: f64 = 40.0;
/// A line to an actor stops this far above its head.
const HEAD_GAP: f64 = 3.0;
/// How far below the row an «include» between use cases that are not neighbours runs.
const DETOUR: f64 = 16.0;

fn centre_x(n: &LaidOutNode) -> f64 {
    n.x + n.width / 2.0
}

fn clamp(v: f64, lo: f64, hi: f64) -> f64 {
    lo.max(hi.min(v))
}

/// Lexical order of two keys, as JavaScript sorts by a difference.
fn compare_keys(a: &[f64], b: &[f64]) -> Ordering {
    match a.iter().zip(b).find(|(x, y)| x != y) {
        Some((x, y)) => (x - y).partial_cmp(&0.0).unwrap_or(Ordering::Equal),
        None => Ordering::Equal,
    }
}

struct Row<'a> {
    by_id: HashMap<&'a str, &'a GraphNode>,
    includes: Vec<&'a GraphEdge>,
    placed: HashSet<&'a str>,
}

impl<'a> Row<'a> {
    /// A use case with half of what it includes on either side, so a use case
    /// including two stands between them.
    fn sequence(&mut self, use_case: &'a GraphNode) -> Vec<&'a GraphNode> {
        self.placed.insert(&use_case.id);
        let targets: Vec<&'a GraphNode> = self
            .includes
            .iter()
            .filter(|e| e.source_id == use_case.id)
            .map(|e| self.by_id[e.target_id.as_str()])
            .filter(|target| !self.placed.contains(target.id.as_str()))
            .collect();
        for target in &targets {
            self.placed.insert(&target.id);
        }
        let half = targets.len() / 2;
        let mut row = Vec::new();
        for target in &targets[..half] {
            row.extend(self.sequence(target));
        }
        row.push(use_case);
        for target in &targets[half..] {
            row.extend(self.sequence(target));
        }
        row
    }
}

/// Side by side, in the order of their keys, each centred on what it is joined to as
/// far as the one before it allows.
fn spread<'g>(
    at: &mut HashMap<&'g str, Point>,
    nodes: &[&'g GraphNode],
    top: impl Fn(&GraphNode) -> f64,
    key: impl Fn(&HashMap<&'g str, Point>, &GraphNode) -> Vec<f64>,
    middle: &impl Fn(&HashMap<&'g str, Point>, &str) -> f64,
) {
    let mut ordered: Vec<(&'g GraphNode, Vec<f64>)> = nodes.iter().map(|n| (*n, key(at, n))).collect();
    ordered.sort_by(|a, b| compare_keys(&a.1, &b.1));
    let mut right = f64::NEG_INFINITY;
    for (n, _) in ordered {
        let left = (middle(at, &n.id) - n.width / 2.0).max(right + SIDE_GAP);
        at.insert(&n.id, Point { x: left, y: top(n) });
        right = left + n.width;
    }
}

/// Lays out the use case view as OMG's examples are drawn (§7.25), and without ELK:
/// the subject above its use cases, each joined to it by a short line, and the actors
/// below them. OMG draws one use case, its actors beside it; a view of several,
/// sharing the subject and the actors, keeps every line clear of every box only with
/// the two facing the row of use cases from opposite sides. Each use case stands
/// between the use cases it includes, so an «include» joins neighbours.
pub fn layout_use_cases(graph: &DiagramGraph) -> LaidOutDiagram {
    let by_id: HashMap<&str, &GraphNode> = graph.nodes.iter().map(|n| (n.id.as_str(), n)).collect();
    let use_cases: Vec<&GraphNode> = graph.nodes.iter().filter(|n| is_use_case_node(&n.keyword)).collect();
    let subjects: Vec<&GraphNode> = graph.nodes.iter().filter(|n| n.keyword == "subject").collect();
    let actors: Vec<&GraphNode> = graph.nodes.iter().filter(|n| n.shape == NodeShape::Actor).collect();
    let includes: Vec<&GraphEdge> = graph.edges.iter().filter(|e| e.kind == EdgeKind::Include).collect();
    let lines: Vec<&GraphEdge> = graph.edges.iter().filter(|e| e.kind == EdgeKind::Association).collect();

    // The row: each use case with half of what it includes on either side.
    let included: HashSet<&str> = includes.iter().map(|e| e.target_id.as_str()).collect();
    let mut sequencer = Row { by_id: by_id.clone(), includes: includes.clone(), placed: HashSet::new() };
    let mut row: Vec<&GraphNode> = Vec::new();
    let roots = use_cases.iter().filter(|u| !included.contains(u.id.as_str())).chain(&use_cases);
    for root in roots {
        if !sequencer.placed.contains(root.id.as_str()) {
            row.extend(sequencer.sequence(root));
        }
    }

    let joins = |a: &str, b: &str| {
        includes.iter().any(|e| (e.source_id == a && e.target_id == b) || (e.source_id == b && e.target_id == a))
    };
    let subject_height = subjects.iter().map(|s| s.height).fold(0.0, f64::max);
    let row_top = MARGIN + if subjects.is_empty() { 0.0 } else { subject_height + ROW_GAP };
    let row_height = row.iter().map(|u| u.height).fold(0.0, f64::max);
    let mut at: HashMap<&str, Point> = HashMap::new();
    let mut x = 0.0;
    for (i, use_case) in row.iter().enumerate() {
        at.insert(&use_case.id, Point { x, y: row_top + (row_height - use_case.height) / 2.0 });
        let joined = row.get(i + 1).is_some_and(|next| joins(&use_case.id, &next.id));
        x += use_case.width + if joined { INCLUDE_GAP } else { BOX_GAP };
    }

    // The middle of the use cases a subject or an actor is joined to.
    let middle = |at: &HashMap<&str, Point>, id: &str| -> f64 {
        let joined: Vec<&GraphNode> = lines
            .iter()
            .filter(|e| e.source_id == id || e.target_id == id)
            .map(|e| by_id[if e.source_id == id { e.target_id.as_str() } else { e.source_id.as_str() }])
            .filter(|n| at.contains_key(n.id.as_str()))
            .collect();
        if joined.is_empty() {
            0.0
        } else {
            joined.iter().fold(0.0, |sum, n| sum + at[n.id.as_str()].x + n.width / 2.0) / joined.len() as f64
        }
    };
    let subject_top = |s: &GraphNode| MARGIN + subject_height - s.height;
    spread(&mut at, &subjects, subject_top, |at, s| vec![middle(at, &s.id)], &middle);
    // A primary actor before a secondary one: the extractor points a secondary
    // actor's lines at the actor.
    let secondary = |id: &str| if lines.iter().any(|e| e.target_id == id) { 1.0 } else { 0.0 };
    let actors_top = row_top + row_height + ROW_GAP;
    spread(&mut at, &actors, |_| actors_top, |at, a| vec![secondary(&a.id), middle(at, &a.id)], &middle);

    let drawn: Vec<&GraphNode> = graph.nodes.iter().filter(|n| at.contains_key(n.id.as_str())).collect();
    if drawn.is_empty() {
        return LaidOutDiagram { width: 2.0 * MARGIN, height: 2.0 * MARGIN, nodes: Vec::new(), edges: Vec::new() };
    }
    let shift = MARGIN - drawn.iter().map(|n| at[n.id.as_str()].x).fold(f64::INFINITY, f64::min);
    let nodes: Vec<LaidOutNode> = drawn
        .iter()
        .map(|n| LaidOutNode {
            id: n.id.clone(),
            shape: n.shape,
            keyword: n.keyword.clone(),
            name: n.name.clone(),
            compartments: n.compartments.clone(),
            ports: Vec::new(),
            has_children: false,
            x: at[n.id.as_str()].x + shift,
            y: at[n.id.as_str()].y,
            width: n.width,
            height: n.height,
        })
        .collect();
    let laid_out: HashMap<&str, &LaidOutNode> = nodes.iter().map(|n| (n.id.as_str(), n)).collect();
    let row_index = |id: &str| row.iter().position(|u| u.id == id).map_or(-1, |i| i as i64);

    // An «include» runs straight to a neighbour, or round below the row to any other.
    // A subject's or actor's line meets a use case's top or bottom edge, straight up
    // or down where the two overlap, leaving the subject's bottom edge or the actor's
    // head.
    let route = |e: &GraphEdge, source: &LaidOutNode, target: &LaidOutNode| -> Vec<Point> {
        if e.kind == EdgeKind::Include {
            if (row_index(&source.id) - row_index(&target.id)).abs() == 1 {
                let y = source.y + source.height / 2.0;
                let rightwards = centre_x(target) > centre_x(source);
                return vec![
                    Point { x: if rightwards { source.x + source.width } else { source.x }, y },
                    Point { x: if rightwards { target.x } else { target.x + target.width }, y },
                ];
            }
            let below = row_top + row_height + DETOUR;
            return vec![
                Point { x: centre_x(source), y: source.y + source.height },
                Point { x: centre_x(source), y: below },
                Point { x: centre_x(target), y: below },
                Point { x: centre_x(target), y: target.y + target.height },
            ];
        }
        let from_use_case = source.shape != NodeShape::Actor && source.keyword != "subject";
        let (use_case, other) = if from_use_case { (source, target) } else { (target, source) };
        let on_use_case = |x: f64| clamp(x, use_case.x + USAGE_RADIUS, use_case.x + use_case.width - USAGE_RADIUS);
        let mut ends = if other.shape == NodeShape::Actor {
            let head = Point { x: centre_x(other), y: other.y - HEAD_GAP };
            vec![head, Point { x: on_use_case(head.x), y: use_case.y + use_case.height }]
        } else {
            let start_x = clamp(centre_x(use_case), other.x + USAGE_RADIUS, other.x + other.width - USAGE_RADIUS);
            let start = Point { x: start_x, y: other.y + other.height };
            vec![start, Point { x: on_use_case(start.x), y: use_case.y }]
        };
        // In the edge's own direction, from its source to its target.
        if from_use_case {
            ends.reverse();
        }
        ends
    };

    let edges: Vec<LaidOutEdge> = graph
        .edges
        .iter()
        .filter_map(|e| {
            let (source, target) = (laid_out.get(e.source_id.as_str())?, laid_out.get(e.target_id.as_str())?);
            Some(LaidOutEdge {
                id: e.id.clone(),
                kind: e.kind,
                source_id: e.source_id.clone(),
                target_id: e.target_id.clone(),
                label: e.label.clone(),
                points: route(e, source, target),
                label_box: None,
            })
        })
        .collect();

    LaidOutDiagram {
        width: nodes.iter().map(|n| n.x + n.width).fold(f64::NEG_INFINITY, f64::max) + MARGIN,
        height: nodes.iter().map(|n| n.y + n.height).fold(f64::NEG_INFINITY, f64::max) + MARGIN,
        nodes,
        edges,
    }
}
