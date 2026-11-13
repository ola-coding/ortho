//! The web shop example: software only, so four views, each drawing what the model says.
use ortho::diagrams::implementation::extract_implementation_graph;
use ortho::diagrams::logical::extract_logical_graph;
use ortho::diagrams::sequence::extract_sequence_model;
use ortho::diagrams::use_case::extract_use_case_graph;
use ortho::model::graph::{DiagramGraph, EdgeKind, NodeShape, is_use_case_node};
use ortho::model::views::{DiagramKind, diagram_kind, view_heading};
use ortho::parser::ast::Prop;
use ortho::parser::workspace::{Severity, Workspace};
use ortho::pipeline::render_views;
use ortho::render::theme::LIGHT;
use std::collections::HashSet;

mod common;
use common::{all_nodes, exposed_by_view, lines_of, names, sorted_names};

/// One file per view. No hardware, so no physical or deployment file.
const ALL: &[&str] = &["use-case.sysml", "logical.sysml", "implementation.sysml", "process.sysml"];

fn parse_set(files: &[&str]) -> Workspace {
    common::example("web-shop", files)
}

fn logical() -> DiagramGraph {
    let workspace = parse_set(&["logical.sysml", "use-case.sysml"]);
    extract_logical_graph(&workspace.ast, &exposed_by_view(&workspace), &workspace.packages())
}

fn implementation() -> DiagramGraph {
    let workspace = parse_set(&["implementation.sysml", "logical.sysml", "use-case.sysml"]);
    extract_implementation_graph(&workspace.ast, &exposed_by_view(&workspace))
}

#[test]
fn parses_and_links_all_four_files_as_one_workspace_with_no_warning() {
    let workspace = parse_set(ALL);
    let packages: Vec<_> =
        workspace.packages().into_iter().map(|p| workspace.ast.text(p, Prop::Name).unwrap()).collect();
    assert_eq!(
        packages,
        [
            "SystemUseCases",
            "Functions",
            "Aws",
            "Apis",
            "Storefront",
            "Catalogue",
            "Ordering",
            "Fulfilment",
            "PlaceOrderScenario"
        ]
    );
    // With no deployment view, nothing has to run on a host; every function is
    // performed by software, and every use case is refined.
    assert_eq!(workspace.reported(Severity::Warning), Vec::<String>::new());
}

#[test]
fn the_use_cases_parse_on_their_own() {
    parse_set(&["use-case.sysml"]);
}

#[test]
fn use_case_view_customers_and_staff_with_two_outside_parties() {
    let workspace = parse_set(&["use-case.sysml"]);
    let graph = extract_use_case_graph(&workspace.ast, &exposed_by_view(&workspace));
    assert_eq!(
        sorted_names(graph.nodes.iter().filter(|n| n.keyword == "use case")),
        ["findProduct : Shop", "fulfilOrder : RunShop", "payOrder : Shop", "placeOrder : Shop", "trackOrder : Shop"]
    );
    // Primary actors from the two use case defs, and the payment provider and the
    // carrier as secondary actors, each on the one use case it takes part in.
    let actors: HashSet<&str> =
        graph.nodes.iter().filter(|n| n.shape == NodeShape::Actor).map(|n| n.name.as_str()).collect();
    let mut actors: Vec<&str> = actors.into_iter().collect();
    actors.sort();
    assert_eq!(
        actors,
        ["carrier : Carrier", "clerk : Clerk", "customer : Customer", "paymentProvider : PaymentProvider"]
    );
    let includes: Vec<(&str, &str)> = graph
        .edges
        .iter()
        .filter(|e| e.kind == EdgeKind::Include)
        .map(|e| (e.source_id.as_str(), e.target_id.as_str()))
        .collect();
    assert_eq!(includes, [("SystemUseCases::placeOrder", "SystemUseCases::payOrder")]);
}

#[test]
fn logical_view_one_tree_refining_every_use_case_and_naming_no_service() {
    let graph = logical();
    let functions = graph.nodes.iter().filter(|n| !is_use_case_node(&n.keyword)).count();
    assert_eq!(graph.edges.iter().filter(|e| e.kind == EdgeKind::Decomposition).count(), functions - 1);
    assert_eq!(graph.edges.iter().filter(|e| e.kind == EdgeKind::Dependency).count(), 5);
    let all = names(&graph.nodes);
    for realization in ["Lambda", "DynamoDB", "OrderFunction", "StorefrontApp"] {
        assert!(!all.contains(&realization), "{realization}");
    }
}

#[test]
fn implementation_view_four_parts_of_the_shop_all_built_on_aws() {
    let graph = implementation();
    let imports: Vec<String> = graph.edges.iter().map(|e| format!("{} -> {}", e.source_id, e.target_id)).collect();
    assert_eq!(imports, ["Storefront -> Aws", "Catalogue -> Aws", "Ordering -> Aws", "Fulfilment -> Aws"]);

    let all = all_nodes(&graph.nodes);
    let headings = names(all.iter().copied());
    for module in ["StorefrontApp :> S3", "OrderFunction :> Lambda", "OrderTable :> DynamoDB"] {
        assert!(headings.contains(&module), "{module}");
    }
    let ports = |name: &str| lines_of(all.iter().find(|n| n.name == name).unwrap(), "ports");
    // The order function publishes what happened; fulfilment and the notifier subscribe.
    assert_eq!(
        ports("OrderFunction :> Lambda"),
        ["orders : OrderApi", "payment : ~PaymentApi", "events : OrderEvents"]
    );
    assert_eq!(ports("NotifyFunction :> Lambda"), ["orders : ~OrderEvents", "shipments : ~ShipmentEvents"]);
}

#[test]
fn process_view_an_order_paid_for_with_the_provider() {
    let workspace = parse_set(&["process.sysml", "implementation.sysml", "logical.sysml", "use-case.sysml"]);
    let sequence = extract_sequence_model(&workspace.ast, &exposed_by_view(&workspace));
    let lifelines: Vec<&str> = sequence.lifelines.iter().map(|l| l.label.as_str()).collect();
    assert_eq!(
        lifelines,
        [
            "customer : Customer",
            "storefront : StorefrontApp",
            "orders : OrderFunction",
            "payments : PaymentFunction",
            "provider : PaymentProvider",
            "fulfilment : FulfilmentFunction",
        ]
    );
    assert_eq!(sequence.messages[0].label, "checkout");
    assert_eq!(sequence.messages.last().unwrap().label, "reserveStock");
}

#[test]
fn renders_all_four_views_each_to_a_file_named_after_it() {
    let workspace = parse_set(ALL);
    let ast = &workspace.ast;
    let rendering = render_views(&workspace, &LIGHT).unwrap();
    assert!(rendering.skipped.is_empty());
    let files: Vec<(DiagramKind, &str)> =
        rendering.rendered.iter().map(|r| (diagram_kind(ast, r.view), r.file.as_str())).collect();
    assert_eq!(
        files,
        [
            (DiagramKind::UseCase, "use-case-view-web-shop.svg"),
            (DiagramKind::Logical, "logical-view-web-shop.svg"),
            (DiagramKind::Implementation, "implementation-view-web-shop.svg"),
            (DiagramKind::Process, "process-view-web-shop.svg"),
        ]
    );
    for rendered in &rendering.rendered {
        let heading = view_heading(ast, rendered.view);
        assert!(rendered.result.svg.contains(heading), "{heading}");
        assert!(rendered.warning.is_none(), "{heading}");
    }
}
