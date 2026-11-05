//! The logical view: a function tree, laid out as a work-breakdown chart.
use ortho::diagrams::logical::extract_logical_graph;
use ortho::layout::tree::layout_tree;
use ortho::layout::{LaidOutDiagram, LaidOutNode, Point};
use ortho::model::graph::{DiagramGraph, EdgeKind, GraphEdge, GraphNode, NodeShape, USAGE_RADIUS};
use ortho::model::views::exposed_elements;
use ortho::parser::workspace::{Severity, Workspace};
use ortho::render::svg::render_svg;
use ortho::render::theme::LIGHT;
use std::collections::{HashMap, HashSet};

mod common;
use common::{centre, close_to, count, everything, fixture, names, title, views};

fn example_graph() -> DiagramGraph {
    let workspace = fixture("logical.sysml");
    extract_logical_graph(&workspace.ast, &everything(&workspace), &workspace.packages())
}

fn node<'a>(laid_out: &'a LaidOutDiagram, name: &str) -> &'a LaidOutNode {
    laid_out.nodes.iter().find(|n| n.name == name || n.id == name).unwrap_or_else(|| panic!("no {name}"))
}

fn middle(n: &LaidOutNode) -> f64 {
    centre(n.x, n.width)
}

#[test]
fn produces_one_box_per_function_an_action_def_for_the_root_an_action_below_it() {
    let graph = example_graph();
    let root = &graph.nodes[0];
    assert_eq!(
        (root.name.as_str(), root.shape, root.keyword.as_str()),
        ("OperateDrone", NodeShape::Definition, "action def")
    );
    assert!(graph.nodes[1..].iter().all(|n| n.shape == NodeShape::Usage && n.keyword == "action"));
    let all = names(&graph.nodes);
    for name in ["OperateDrone", "generateThrust", "holdAltitude"] {
        assert!(all.contains(&name), "{name}");
    }
}

#[test]
fn is_a_strict_tree_one_root_one_parent_per_function() {
    let graph = example_graph();
    // A tree of n nodes has exactly n-1 branches.
    assert_eq!(graph.edges.len(), graph.nodes.len() - 1);
    let mut parents: HashMap<&str, usize> = HashMap::new();
    for edge in &graph.edges {
        *parents.entry(edge.target_id.as_str()).or_default() += 1;
    }
    assert!(parents.values().all(|count| *count == 1));
    assert_eq!(names(graph.nodes.iter().filter(|n| !parents.contains_key(n.id.as_str()))), ["OperateDrone"]);
}

#[test]
fn draws_branches_as_plain_decomposition_lines_never_arrows() {
    let graph = example_graph();
    assert!(graph.edges.iter().all(|e| e.kind == EdgeKind::Decomposition && e.label.is_none()));
}

#[test]
fn renders_each_branch_with_a_filled_diamond_at_its_parent_and_no_arrowhead() {
    let graph = example_graph();
    let laid_out = layout_tree(&graph);
    let svg = render_svg(&laid_out, &title("Logical view — DroneFunctions", "fixtures/logical.sysml"), &LIGHT);
    assert!(svg.contains(&format!(r#"rx="{USAGE_RADIUS}""#)));
    // A function is owned by its parent (§8.2.3.6): the diamond sits where every
    // branch leaves it, so siblings share one.
    assert_eq!(count(&svg, r#"marker-start="url(#diamond)""#), graph.edges.len());
    assert!(!svg.contains("marker-end"));
    let start = |target: &str| laid_out.edges.iter().find(|e| e.target_id == target).unwrap().points[0];
    let parents: HashSet<&str> = graph.edges.iter().map(|e| e.source_id.as_str()).collect();
    for parent in parents {
        let starts: Vec<Point> =
            graph.edges.iter().filter(|e| e.source_id == parent).map(|e| start(&e.target_id)).collect();
        assert!(starts.iter().all(|p| *p == starts[0]), "{parent}");
    }
}

#[test]
fn lays_the_tree_out_as_a_work_breakdown_chart() {
    let graph = example_graph();
    let laid_out = layout_tree(&graph);
    assert_eq!(laid_out.nodes.len(), graph.nodes.len());

    let mut family: Vec<(&str, Vec<&str>)> = Vec::new();
    for edge in &graph.edges {
        match family.iter_mut().find(|(parent, _)| *parent == edge.source_id) {
            Some((_, kids)) => kids.push(&edge.target_id),
            None => family.push((&edge.source_id, vec![&edge.target_id])),
        }
    }
    let has_kids = |id: &str| family.iter().any(|(parent, _)| *parent == id);
    for (parent, kids) in &family {
        let parent = node(&laid_out, parent);
        let boxes: Vec<&LaidOutNode> = kids.iter().map(|k| node(&laid_out, k)).collect();
        assert!(boxes.iter().all(|b| b.y > parent.y + parent.height));
        if kids.iter().all(|k| !has_kids(k)) {
            // The lowest level is listed: one column, in declaration order.
            assert!(boxes.iter().all(|b| b.x == boxes[0].x));
            assert!(boxes.windows(2).all(|pair| pair[1].y > pair[0].y));
        } else {
            // Any other family is a row. Its parent stands over the middle child when
            // there is one, and over the middle of the row when not.
            assert!(boxes.iter().all(|b| b.y == boxes[0].y));
            let row_middle = if boxes.len() % 2 == 1 {
                middle(boxes[(boxes.len() - 1) / 2])
            } else {
                (middle(boxes[0]) + middle(boxes[boxes.len() - 1])) / 2.0
            };
            assert!(close_to(middle(parent), row_middle, 5));
        }
    }

    // No two boxes overlap, and every branch is drawn with right angles.
    for a in &laid_out.nodes {
        for b in &laid_out.nodes {
            if a.id != b.id {
                let apart =
                    a.x + a.width <= b.x || b.x + b.width <= a.x || a.y + a.height <= b.y || b.y + b.height <= a.y;
                assert!(apart, "{} overlaps {}", a.name, b.name);
            }
        }
    }
    for edge in &laid_out.edges {
        assert!(edge.points.windows(2).all(|pair| pair[0].x == pair[1].x || pair[0].y == pair[1].y));
    }
}

/// A model whose view exposes the given elements, the errors it reports, and what
/// the logical view draws of it.
fn refining(exposes: &[&str], refinements: &[&str]) -> (Vec<String>, DiagramGraph) {
    let refinements: Vec<String> = refinements.iter().map(|r| format!("#refinement dependency {r};")).collect();
    let exposes: Vec<String> = exposes.iter().map(|e| format!("expose {e};")).collect();
    let text = format!(
        "package SystemUseCases {{
            use case def Order;
            use case def Return;
        }}
        package Functions {{
            private import ModelingMetadata::*;
            action def HandleOrder {{ action pack; action ship; }}
            action def HandleReturn {{ action refund; }}
            {}
            view v : GeneralView {{ {} }}
        }}",
        refinements.join("\n"),
        exposes.join(" ")
    );
    let workspace = Workspace::build(vec![("model.sysml".to_string(), text)]).unwrap();
    let errors = workspace.documents[0]
        .diagnostics
        .iter()
        .filter(|d| d.severity == Severity::Error)
        .map(|d| d.message.clone())
        .collect();
    let exposed = exposed_elements(&workspace.ast, views(&workspace)[0]);
    let graph = extract_logical_graph(&workspace.ast, &exposed, &workspace.packages());
    (errors, graph)
}

const ORDER: &[&str] = &["HandleOrder to SystemUseCases::Order"];

#[test]
fn stands_a_use_case_above_its_tree_with_a_dashed_arrow_up_to_it() {
    let (errors, graph) = refining(&["HandleOrder", "SystemUseCases::Order"], ORDER);
    assert_eq!(errors, Vec::<String>::new());
    let laid_out = layout_tree(&graph);
    let (root, use_case) = (node(&laid_out, "HandleOrder"), node(&laid_out, "Order"));
    assert_eq!((use_case.shape, use_case.keyword.as_str()), (NodeShape::Definition, "use case def"));
    assert!(use_case.y + use_case.height < root.y);
    assert!(close_to(middle(use_case), middle(root), 2));

    // From the client up to the supplier, straight.
    let arrow = laid_out.edges.iter().find(|e| e.kind == EdgeKind::Dependency).unwrap();
    assert_eq!(
        arrow.points,
        [Point { x: middle(root), y: root.y }, Point { x: middle(root), y: use_case.y + use_case.height }]
    );
    let svg = render_svg(&laid_out, &title("Functions", "inline"), &LIGHT);
    assert!(svg.contains(r#"stroke-dasharray="6 4" marker-end="url(#openArrow)""#));
}

#[test]
fn draws_neither_a_use_case_nor_an_arrow_the_view_does_not_expose() {
    let (_, graph) = refining(&["HandleOrder"], ORDER);
    assert_eq!(names(&graph.nodes), ["HandleOrder", "pack", "ship"]);
    assert!(graph.edges.iter().all(|e| e.kind == EdgeKind::Decomposition));
}

#[test]
fn shows_a_use_case_nothing_refines_last_and_with_no_arrow() {
    let (_, graph) = refining(&["HandleOrder", "SystemUseCases::Return", "SystemUseCases::Order"], ORDER);
    let laid_out = layout_tree(&graph);
    let arrows: Vec<&str> =
        laid_out.edges.iter().filter(|e| e.kind == EdgeKind::Dependency).map(|e| e.target_id.as_str()).collect();
    assert_eq!(arrows, ["SystemUseCases::Order"]);
    let order = node(&laid_out, "Order");
    assert!(node(&laid_out, "Return").x > order.x + order.width);
}

#[test]
fn centres_a_use_case_over_all_the_trees_that_refine_it_and_keeps_a_row_of_them_apart() {
    let (_, both) = refining(
        &["HandleOrder", "HandleReturn", "SystemUseCases::Order"],
        &["HandleOrder to SystemUseCases::Order", "HandleReturn to SystemUseCases::Order"],
    );
    let shared = layout_tree(&both);
    let trees = (middle(node(&shared, "HandleOrder")) + middle(node(&shared, "HandleReturn"))) / 2.0;
    assert!(close_to(middle(node(&shared, "Order")), trees, 2));

    // One tree refining both: its use cases side by side, not overlapping.
    let (_, one) = refining(
        &["HandleOrder", "SystemUseCases::Order", "SystemUseCases::Return"],
        &["HandleOrder to SystemUseCases::Order, SystemUseCases::Return"],
    );
    let laid_out = layout_tree(&one);
    assert_eq!(laid_out.edges.iter().filter(|e| e.kind == EdgeKind::Dependency).count(), 2);
    let (order, ret) = (node(&laid_out, "Order"), node(&laid_out, "Return"));
    assert!(ret.x >= order.x + order.width);
    assert!(laid_out.width >= ret.x + ret.width);
}

#[test]
fn fails_on_a_dependency_that_names_nothing_so_a_typo_cannot_pass() {
    let (errors, _) = refining(&["HandleOrder"], &["HandleOrder to SystemUseCases::Ordr"]);
    assert_eq!(errors, ["Could not resolve reference to Member named 'SystemUseCases::Ordr'."]);
}

fn function(id: &str, width: f64) -> GraphNode {
    GraphNode {
        id: id.to_string(),
        shape: NodeShape::Usage,
        keyword: "action".to_string(),
        name: id.to_string(),
        compartments: Vec::new(),
        ports: Vec::new(),
        children: None,
        width,
        height: 40.0,
    }
}

fn decompose(parent: &str, child: &str) -> GraphEdge {
    GraphEdge::new(format!("{parent}-{child}"), EdgeKind::Decomposition, parent.to_string(), child.to_string())
}

/// `a` carries a wide listed leaf, so the row is lopsided: the middle of the row is
/// 67 px from the middle child, which the first test confirms before relying on it.
fn lopsided(extra: &[GraphNode]) -> LaidOutDiagram {
    let mut nodes = vec![
        function("root", 100.0),
        function("a", 40.0),
        function("a1", 150.0),
        function("b", 200.0),
        function("c", 60.0),
    ];
    let mut edges = vec![decompose("root", "a"), decompose("a", "a1"), decompose("root", "b"), decompose("root", "c")];
    for n in extra {
        nodes.push(n.clone());
        edges.push(decompose("root", &n.id));
    }
    layout_tree(&DiagramGraph { nodes, edges })
}

#[test]
fn drops_the_stem_straight_onto_the_middle_child_of_an_odd_row() {
    let laid_out = lopsided(&[]);
    let at = |id: &str| middle(node(&laid_out, id));
    assert!(((at("a") + at("c")) / 2.0 - at("b")).abs() > 50.0);

    assert!(close_to(at("root"), at("b"), 9));
    let stem = laid_out.edges.iter().find(|e| e.target_id == "b").unwrap();
    assert!(stem.points.iter().all(|p| p.x == stem.points[0].x));
}

#[test]
fn keeps_the_stem_over_the_middle_of_an_even_row_which_has_no_middle_child() {
    let laid_out = lopsided(&[function("d", 80.0)]);
    let at = |id: &str| middle(node(&laid_out, id));
    assert!(close_to(at("root"), (at("a") + at("d")) / 2.0, 9));
}

#[test]
fn draws_a_branch_of_a_tree_alone_when_a_view_exposes_only_that_function() {
    // A tree too big for one page splits into a view for each branch.
    let workspace = common::parse_ok(
        "package P {
            private import StandardViewDefinitions::*;
            action def Top {
                action a { action a1 { action a1x; action a1y; } action a2; }
                action b { action b1; }
            }
            view 'Branch a' : GeneralView { expose Top::a; }
        }",
    );
    let view = views(&workspace)[0];
    assert_eq!(ortho::model::views::why_undrawable(&workspace.ast, view), None);
    assert_eq!(ortho::model::views::diagram_kind(&workspace.ast, view), ortho::model::views::DiagramKind::Logical);
    let graph = extract_logical_graph(&workspace.ast, &exposed_elements(&workspace.ast, view), &workspace.packages());
    let mut drawn = names(graph.nodes.iter());
    drawn.sort();
    assert_eq!(drawn, ["a", "a1", "a1x", "a1y", "a2"]);
    let laid_out = layout_tree(&graph);
    assert!(node(&laid_out, "a").y < node(&laid_out, "a1").y);
}
