//! The implementation view, software as packages of modules, and the deployment view,
//! the hosts each module runs on.
use ortho::diagrams::deployment::extract_deployment_graph;
use ortho::diagrams::implementation::extract_implementation_graph;
use ortho::layout::elk::{Algorithm, LayoutOptions, layout_graph};
use ortho::layout::{LaidOutDiagram, LaidOutNode};
use ortho::model::graph::{DiagramGraph, EdgeKind, NodeShape};
use ortho::model::views::{DiagramKind, diagram_kind};
use ortho::parser::ast::{Prop, Type};
use ortho::parser::workspace::Workspace;
use ortho::pipeline::render_views;
use ortho::render::svg::render_svg;
use ortho::render::text_metrics::{Weight, measure_text};
use ortho::render::theme::LIGHT;

mod common;
use common::{compartment, everything, exposed_by_view, fixture, names, parse_ok, sorted_names, title};

fn implementation(workspace: &Workspace) -> DiagramGraph {
    extract_implementation_graph(&workspace.ast, &everything(workspace))
}

fn deployment(workspace: &Workspace) -> DiagramGraph {
    extract_deployment_graph(&workspace.ast, &everything(workspace), &workspace.packages())
}

fn gridded(graph: &DiagramGraph) -> LaidOutDiagram {
    layout_graph(graph, LayoutOptions { grid_leaves: true, ..LayoutOptions::default() }).unwrap()
}

fn node<'a>(laid_out: &'a LaidOutDiagram, name: &str) -> &'a LaidOutNode {
    laid_out.nodes.iter().find(|n| n.name == name).unwrap_or_else(|| panic!("no {name}"))
}

fn bold_width(text: &str) -> f64 {
    measure_text(text, 12.0, Weight::Bold)
}

#[test]
fn parses_allocate_with_relatively_qualified_connector_ends() {
    let workspace = fixture("deployment.sysml");
    let ast = &workspace.ast;
    let software = ast
        .children(workspace.packages()[0], Prop::Members)
        .find(|m| ast.text(*m, Prop::Name) == Some("Software"))
        .unwrap();
    let allocations: Vec<_> =
        ast.children(software, Prop::Members).filter(|m| ast[*m].ty == Type::AllocationUsage).collect();
    assert_eq!(allocations.len(), 4);
    let segments = |end: Prop| -> Vec<&str> {
        let end = ast.child(allocations[0], end).unwrap();
        let references = ast.references_of(end, Prop::Segments);
        references.into_iter().map(|r| ast.text(ast.reference_target(r).unwrap(), Prop::Name).unwrap()).collect()
    };
    assert_eq!(segments(Prop::Source), ["flightStack", "attitudeControl"]);
    assert_eq!(segments(Prop::Target)[0], "fc");
}

#[test]
fn nests_packages_and_resolves_import_edges() {
    let graph = implementation(&fixture("deployment.sysml"));
    assert_eq!(graph.nodes.len(), 1);
    let root = &graph.nodes[0];
    assert_eq!(root.shape, NodeShape::Package);
    // Requirements holds no module, so it is not software and not drawn; the import
    // into it goes with it.
    let children = root.children.as_deref().unwrap();
    assert_eq!(
        sorted_names(children.iter().filter(|c| c.shape == NodeShape::Package)),
        ["Physical", "Software", "Structure"]
    );

    let mut imports: Vec<String> = graph.edges.iter().map(|e| format!("{} -> {}", e.source_id, e.target_id)).collect();
    imports.sort();
    assert_eq!(
        imports,
        [
            "DroneSystem::Physical -> DroneSystem::Structure",
            "DroneSystem::Software -> DroneSystem::Physical",
            "DroneSystem::Software -> DroneSystem::Structure",
        ]
    );
    assert!(graph.edges.iter().all(|e| e.kind == EdgeKind::Import));
    // Each imports a package's members, `Pkg::*` (§8.2.3.5).
    assert!(graph.edges.iter().all(|e| e.label.as_deref() == Some("«import»*")));
}

#[test]
fn draws_the_modules_a_package_declares_inside_it_and_no_instances() {
    let graph = implementation(&fixture("deployment.sysml"));
    let physical = graph.nodes[0].children.as_deref().unwrap().iter().find(|c| c.name == "Physical").unwrap();
    let modules = physical.children.as_deref().unwrap();
    // `fc` and `cc` are package-level part usages: instances, not modules.
    assert_eq!(sorted_names(modules), ["CompanionComputer", "FlightController"]);
    // A module is a definition: a square box headed «part def».
    assert!(modules.iter().all(|c| c.shape == NodeShape::Definition && c.keyword == "part def"));
}

#[test]
fn lists_module_apis_in_a_ports_compartment_conjugating_the_ones_a_module_needs() {
    let laid_out = gridded(&implementation(&parse_ok(
        "package S {
            port def Api;
            part def Server { port api : Api; }
            part def Client { port api : ~Api; }
        }",
    )));
    let server = node(&laid_out, "Server");
    let client = node(&laid_out, "Client");
    assert_eq!(server.compartments, [compartment("ports", &["api : Api"])]);
    assert_eq!(client.compartments, [compartment("ports", &["api : ~Api"])]);

    // A square on the border is for a port a wire lands on; this view draws no wires
    // between modules, and the modules stay gridded.
    assert!(server.ports.is_empty());
    assert_eq!(server.y, client.y);
}

#[test]
fn keeps_a_package_name_wider_than_its_grid_inside_the_package() {
    let laid_out = gridded(&implementation(&parse_ok(
        "package S {
            package FirmwareModulesForTheFlightControllerBoard {
                part def A;
            }
        }",
    )));
    let package = node(&laid_out, "FirmwareModulesForTheFlightControllerBoard");
    let module = node(&laid_out, "A");
    assert!(package.width > bold_width(&package.name));
    // The grid centres under the name.
    assert!(((module.x + module.width / 2.0) - (package.x + package.width / 2.0)).abs() < 0.005);
}

/// A cloud solution from a vendor's services: no use cases, no functions.
const CLOUD: &str = "
    package Aws {
        part def Lambda;
        part def DynamoDB;
    }
    package OrderSolution {
        private import Aws::*;
        part def OrderHandler :> Lambda;
        part def OrderTable :> DynamoDB, Lambda;
        view 'Order solution' : GeneralView {
            expose OrderSolution;
            expose Aws;
        }
    }
";

#[test]
fn renders_an_implementation_view_with_no_use_case_or_logical_view_every_link_between_views_being_optional() {
    let workspace = parse_ok(CLOUD);
    let rendering = render_views(&workspace, &LIGHT).unwrap();
    assert!(rendering.skipped.is_empty());
    let rendered: Vec<(DiagramKind, &str)> =
        rendering.rendered.iter().map(|r| (diagram_kind(&workspace.ast, r.view), r.file.as_str())).collect();
    assert_eq!(rendered, [(DiagramKind::Implementation, "order-solution.svg")]);
}

#[test]
fn heads_a_module_with_what_it_specializes_as_declared() {
    let workspace = parse_ok(CLOUD);
    let graph = extract_implementation_graph(&workspace.ast, &exposed_by_view(&workspace));
    let modules = graph.nodes[0].children.as_deref().unwrap();
    assert_eq!(names(modules), ["OrderHandler :> Lambda", "OrderTable :> DynamoDB, Lambda"]);
    // Its box fits the longer heading.
    let [handler, table] = modules else { panic!() };
    assert!(table.width > handler.width);
    assert!(table.width > bold_width("OrderTable :> DynamoDB, Lambda"));
}

#[test]
fn lists_what_each_host_runs_in_its_relationships_compartment_and_draws_no_edges() {
    let graph = deployment(&fixture("deployment.sysml"));
    assert!(graph.edges.is_empty());

    assert_eq!(sorted_names(&graph.nodes), ["cc : CompanionComputer", "fc : FlightController"]);
    assert!(graph.nodes.iter().all(|n| n.shape == NodeShape::Usage && n.keyword == "part"));
    // Nothing is drawn inside a host: software is not its member.
    assert!(graph.nodes.iter().all(|n| n.children.is_none()));

    let runs = |name: &str| &graph.nodes.iter().find(|n| n.name == name).unwrap().compartments;
    assert_eq!(
        *runs("fc : FlightController"),
        [compartment("relationships", &["allocated flightStack::attitudeControl", "allocated flightStack::telemetry"])]
    );
    assert_eq!(
        *runs("cc : CompanionComputer"),
        [compartment("relationships", &["allocated flightStack::navigation", "allocated flightStack::perception"])]
    );
}

#[test]
fn lists_a_program_allocated_to_two_hosts_on_both_once_each() {
    let graph = deployment(&parse_ok(
        "package S {
            part def Computer;
            part a : Computer;
            part b : Computer;
            part sw { part logger; }
            allocate sw.logger to a;
            allocate sw.logger to b;
            allocate sw.logger to a;
        }",
    ));
    let hosts: Vec<(&str, &[String])> =
        graph.nodes.iter().map(|n| (n.name.as_str(), n.compartments[0].lines.as_slice())).collect();
    let logger = ["allocated sw::logger".to_string()];
    assert_eq!(hosts, [("a : Computer", &logger[..]), ("b : Computer", &logger[..])]);
}

#[test]
fn keeps_a_name_wider_than_the_contents_inside_its_box() {
    let graph = deployment(&parse_ok(
        "package S {
            part def ElectronicsBayWithAVeryLongName {
                part flightControllerComputerWithALongName : Computer;
            }
            part def Computer;
            part bay : ElectronicsBayWithAVeryLongName;
            part sw { part a; }
            allocate sw.a to bay.flightControllerComputerWithALongName;
        }",
    ));
    let options = LayoutOptions { algorithm: Algorithm::RectPacking, ..LayoutOptions::default() };
    let laid_out = layout_graph(&graph, options).unwrap();

    let host = node(&laid_out, "flightControllerComputerWithALongName : Computer");
    let frame = node(&laid_out, "bay : ElectronicsBayWithAVeryLongName");
    for b in [host, frame] {
        assert!(b.width > bold_width(&b.name), "{}", b.name);
    }
    // The host sits inside its device, as the part it is.
    assert!(host.x > frame.x);
    assert!(host.x + host.width < frame.x + frame.width);
    assert_eq!(host.compartments[0].lines, ["allocated sw::a"]);
}

#[test]
fn renders_both_diagrams_end_to_end() {
    let workspace = fixture("deployment.sysml");
    let options = LayoutOptions::default();

    let heading = title("Implementation view — DroneSystem", "fixtures/deployment.sysml");
    let implementation_svg = render_svg(&layout_graph(&implementation(&workspace), options).unwrap(), &heading, &LIGHT);
    assert!(implementation_svg.contains("&#171;import&#187;*"));

    let heading = title("Deployment view — DroneSystem", "fixtures/deployment.sysml");
    let deployment_svg = render_svg(&layout_graph(&deployment(&workspace), options).unwrap(), &heading, &LIGHT);
    // Hosts list what they run, and nothing on this view is an arrow.
    assert!(deployment_svg.contains("fc : FlightController"));
    assert!(deployment_svg.contains(">relationships</text>"));
    assert!(deployment_svg.contains(">allocated flightStack::attitudeControl</text>"));
    assert!(!deployment_svg.contains("marker-end"));
}
