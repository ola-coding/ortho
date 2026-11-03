//! The coffee machine example: each of its six views draws what the model says.
use ortho::diagrams::deployment::extract_deployment_graph;
use ortho::diagrams::implementation::extract_implementation_graph;
use ortho::diagrams::logical::extract_logical_graph;
use ortho::diagrams::physical::extract_physical_graph;
use ortho::diagrams::sequence::extract_sequence_model;
use ortho::diagrams::use_case::extract_use_case_graph;
use ortho::model::graph::{DiagramGraph, EdgeKind, NodeShape, is_use_case_node, qualified_name};
use ortho::model::views::{DiagramKind, diagram_kind, view_heading};
use ortho::parser::ast::{Prop, Type};
use ortho::parser::workspace::Workspace;
use ortho::pipeline::render_views;
use ortho::render::theme::LIGHT;
use std::collections::HashSet;

mod common;
use common::{all_nodes, exposed_by_view, lines_of, names, sorted_names};

/// One file per view.
const ALL: &[&str] =
    &["use-case.sysml", "logical.sysml", "implementation.sysml", "physical.sysml", "deployment.sysml", "process.sysml"];

/// The view whose file declares everything it draws, and names no other.
const SELF_CONTAINED: &[&str] = &["use-case.sysml"];

fn parse_set(files: &[&str]) -> Workspace {
    common::example("coffee-machine", files)
}

fn logical() -> (Workspace, DiagramGraph) {
    let workspace = parse_set(&["logical.sysml", "use-case.sysml"]);
    let graph = extract_logical_graph(&workspace.ast, &exposed_by_view(&workspace), &workspace.packages());
    (workspace, graph)
}

fn implementation() -> DiagramGraph {
    let workspace = parse_set(&["implementation.sysml", "logical.sysml", "use-case.sysml"]);
    extract_implementation_graph(&workspace.ast, &exposed_by_view(&workspace))
}

fn physical() -> DiagramGraph {
    let workspace = parse_set(&["physical.sysml", "logical.sysml", "use-case.sysml"]);
    extract_physical_graph(&workspace.ast, &exposed_by_view(&workspace))
}

#[test]
fn parses_and_links_all_six_files_as_one_workspace() {
    let workspace = parse_set(ALL);
    let packages: Vec<_> =
        workspace.packages().into_iter().map(|p| workspace.ast.text(p, Prop::Name).unwrap()).collect();
    assert_eq!(
        packages,
        ["SystemUseCases", "Functions", "Software", "Hardware", "Deployment", "MakeCappuccinoScenario"]
    );
}

#[test]
fn each_self_contained_view_parses_on_its_own() {
    // Where the chain starts: the use cases name no other view.
    for file in SELF_CONTAINED {
        parse_set(&[file]);
    }
}

#[test]
fn use_case_view_two_drinks_and_a_service_task_all_of_the_one_machine() {
    let workspace = parse_set(&["use-case.sysml"]);
    let graph = extract_use_case_graph(&workspace.ast, &exposed_by_view(&workspace));
    assert_eq!(
        sorted_names(graph.nodes.iter().filter(|n| n.keyword == "use case")),
        ["descale : ServiceMachine", "makeCappuccino : OperateMachine", "makeEspresso : OperateMachine"]
    );
    // Both use case defs name the machine as their subject: one box, a line to each.
    let subjects: Vec<_> = graph.nodes.iter().filter(|n| n.keyword == "subject").collect();
    assert_eq!(names(subjects.iter().copied()), ["machine : CoffeeMachine"]);
    assert_eq!(graph.edges.iter().filter(|e| e.target_id == subjects[0].id).count(), 3);

    assert_eq!(sorted_names(graph.nodes.iter().filter(|n| n.shape == NodeShape::Actor)), ["user : User"]);

    let includes: Vec<(&str, &str)> = graph
        .edges
        .iter()
        .filter(|e| e.kind == EdgeKind::Include)
        .map(|e| (e.source_id.as_str(), e.target_id.as_str()))
        .collect();
    assert_eq!(includes, [("SystemUseCases::makeCappuccino", "SystemUseCases::makeEspresso")]);
}

#[test]
fn logical_view_a_strict_capability_tree_naming_no_realization() {
    let (_, graph) = logical();
    // Two roots, one per use case definition: making a drink, and looking after the
    // machine. A forest of n trees has n fewer edges than nodes.
    let functions = graph.nodes.iter().filter(|n| !is_use_case_node(&n.keyword)).count();
    assert_eq!(graph.edges.iter().filter(|e| e.kind == EdgeKind::Decomposition).count(), functions - 2);

    let all = names(&graph.nodes);
    assert!(all.contains(&"heatWater"));
    assert!(all.contains(&"regulateTemperature"));
    // This view says what, not what by: no realization name reaches it. (`pumpWater`
    // is a function; `WaterPump` is the thing that does it.)
    for realization in ["Thermoblock", "WaterPump", "ControlBoard", "RecipeEngine"] {
        assert!(!all.contains(&realization), "{realization}");
    }

    let children: HashSet<&str> = graph.edges.iter().map(|e| e.target_id.as_str()).collect();
    assert_eq!(
        names(graph.nodes.iter().filter(|n| !children.contains(n.id.as_str()))),
        ["MakeBeverage", "MaintainMachine"]
    );
}

#[test]
fn logical_view_each_tree_refines_its_use_case_and_no_use_case_goes_unrefined() {
    let (workspace, graph) = logical();
    let refinements: Vec<(&str, &str)> = graph
        .edges
        .iter()
        .filter(|e| e.kind == EdgeKind::Dependency)
        .map(|e| (e.source_id.as_str(), e.target_id.as_str()))
        .collect();
    assert_eq!(
        refinements,
        [
            ("Functions::MakeBeverage", "SystemUseCases::OperateMachine"),
            ("Functions::MaintainMachine", "SystemUseCases::ServiceMachine"),
        ]
    );
    // A use case no function tree details would be a hole in the design.
    let ast = &workspace.ast;
    let use_cases =
        workspace.packages().into_iter().find(|p| ast.text(*p, Prop::Name) == Some("SystemUseCases")).unwrap();
    let refined: HashSet<&str> = refinements.iter().map(|(_, target)| *target).collect();
    let unrefined: Vec<String> = ast
        .children(use_cases, Prop::Members)
        .filter(|m| ast[*m].ty == Type::UseCaseDef)
        .map(|m| qualified_name(ast, m))
        .filter(|name| !refined.contains(name.as_str()))
        .collect();
    assert_eq!(unrefined, Vec::<String>::new());
}

#[test]
fn implementation_view_the_application_layer_depends_on_control_never_the_reverse() {
    let graph = implementation();
    let imports: Vec<String> = graph.edges.iter().map(|e| format!("{} -> {}", e.source_id, e.target_id)).collect();
    assert_eq!(imports, ["Software::Application -> Software::Control"]);

    // `sw`, the tree of deployable parts, is an instance, not a module.
    let all = names(all_nodes(&graph.nodes));
    assert!(all.contains(&"RecipeEngine"));
    assert!(!all.contains(&"sw"));
}

#[test]
fn implementation_view_the_recipe_engine_offers_one_api_and_needs_three() {
    let graph = implementation();
    let all = all_nodes(&graph.nodes);
    let ports = |name: &str| lines_of(all.iter().find(|n| n.name == name).unwrap(), "ports");

    // One offered, three needed: the whole of the machine's control is this module
    // calling those.
    assert_eq!(
        ports("RecipeEngine"),
        ["recipe : RecipeApi", "brew : ~BrewApi", "temperature : ~TemperatureApi", "milk : ~MilkApi"]
    );
    // The panel asks the engine; the engine drives the controllers.
    assert_eq!(ports("UiApp"), ["recipe : ~RecipeApi"]);
    assert_eq!(ports("BrewController"), ["brew : BrewApi"]);
}

#[test]
fn physical_view_the_machine_as_nested_parts_looms_named_by_their_interface() {
    let graph = physical();
    assert_eq!(names(&graph.nodes), ["machine : CoffeeMachine"]);
    let parts = names(graph.nodes[0].children.as_deref().unwrap());
    assert_eq!(parts.len(), 11);
    for part in ["heater : Thermoblock", "controller : ControlBoard", "milkTube : MilkTube"] {
        assert!(parts.contains(&part), "{part}");
    }
    // Interface defs label their connection rather than becoming a box.
    assert!(!parts.iter().any(|n| n.contains("ControlLink")));
    assert_eq!(graph.edges.len(), 14);
    let looms =
        graph.edges.iter().filter(|e| e.kind == EdgeKind::Connection && e.label.as_deref() == Some(": ControlLink"));
    assert_eq!(looms.count(), 6);
}

#[test]
fn realization_every_function_is_performed_by_something_and_some_parts_by_nothing() {
    let (_, logical) = logical();
    let parents: HashSet<&str> = logical.edges.iter().map(|e| e.source_id.as_str()).collect();
    let leaves =
        names(logical.nodes.iter().filter(|n| !parents.contains(n.id.as_str()) && !is_use_case_node(&n.keyword)));

    let (software, hardware) = (implementation(), physical());
    let boxes: Vec<_> = all_nodes(&software.nodes).into_iter().chain(all_nodes(&hardware.nodes)).collect();
    let performed: HashSet<&str> =
        boxes.iter().flat_map(|n| n.compartments.iter().flat_map(|c| c.lines.iter().map(String::as_str))).collect();

    // Every function has something that does it; a function nothing performs would
    // show here, as a hole in the design.
    assert_eq!(leaves.iter().filter(|leaf| !performed.contains(*leaf)).count(), 0);
    // Frothing is realized twice over, by the wand and by the program that drives
    // the wand: realization is many to many.
    let frothers = sorted_names(
        boxes.iter().copied().filter(|n| n.compartments.iter().any(|c| c.lines.iter().any(|l| l == "frothMilk"))),
    );
    assert_eq!(frothers, ["MilkController", "wand : SteamWand"]);
    // The other direction is a part that realizes nothing: the panel and the supply
    // are there for the parts that do.
    let idle = names(all_nodes(&hardware.nodes).into_iter().filter(|n| n.compartments.is_empty()));
    assert!(idle.contains(&"display : TouchDisplay"));
    assert!(idle.contains(&"psu : MainsSupply"));
}

#[test]
fn deployment_view_five_programs_on_two_hosts_inside_the_machine() {
    let workspace =
        parse_set(&["deployment.sysml", "implementation.sysml", "physical.sysml", "logical.sysml", "use-case.sysml"]);
    let graph = extract_deployment_graph(&workspace.ast, &exposed_by_view(&workspace), &workspace.packages());
    assert!(graph.edges.is_empty());
    // Both hosts are fitted in the machine, so they share its frame.
    assert_eq!(names(&graph.nodes), ["machine : CoffeeMachine"]);
    let hosts = graph.nodes[0].children.as_deref().unwrap();
    let mut ids: Vec<&str> = hosts.iter().map(|n| n.id.as_str()).collect();
    ids.sort();
    assert_eq!(ids, ["Hardware::CoffeeMachine::controller", "Hardware::CoffeeMachine::display"]);
    assert!(hosts.iter().all(|h| h.keyword == "part"));
    let placed: Vec<&String> = hosts.iter().flat_map(|n| n.compartments.iter().flat_map(|c| &c.lines)).collect();
    assert_eq!(placed.len(), 5);
    assert_eq!(placed.iter().collect::<HashSet<_>>().len(), 5);
}

#[test]
fn process_view_one_press_produces_a_cappuccino() {
    let workspace = parse_set(&["process.sysml", "implementation.sysml", "logical.sysml", "use-case.sysml"]);
    let sequence = extract_sequence_model(&workspace.ast, &exposed_by_view(&workspace));
    let lifelines: Vec<&str> = sequence.lifelines.iter().map(|l| l.label.as_str()).collect();
    assert_eq!(
        lifelines,
        [
            "user",
            "uiApp : UiApp",
            "recipeEngine : RecipeEngine",
            "temperatureController : TemperatureController",
            "brewController : BrewController",
            "milkController : MilkController",
        ]
    );
    assert_eq!(sequence.messages[0].label, "selectCappuccino");
    assert_eq!(sequence.messages.last().unwrap().label, "beverageReady");
}

#[test]
fn renders_all_six_views_each_to_a_file_named_after_it() {
    let workspace = parse_set(ALL);
    let ast = &workspace.ast;
    let rendering = render_views(&workspace, &LIGHT).unwrap();
    assert!(rendering.skipped.is_empty());
    let files: Vec<(DiagramKind, &str)> =
        rendering.rendered.iter().map(|r| (diagram_kind(ast, r.view), r.file.as_str())).collect();
    assert_eq!(
        files,
        [
            (DiagramKind::UseCase, "use-case-view-coffee-machine.svg"),
            (DiagramKind::Logical, "logical-view-coffee-machine.svg"),
            (DiagramKind::Implementation, "implementation-view-coffee-machine.svg"),
            (DiagramKind::Physical, "physical-view-coffee-machine.svg"),
            (DiagramKind::Deployment, "deployment-view-coffee-machine.svg"),
            (DiagramKind::Process, "process-view-coffee-machine.svg"),
        ]
    );
    for rendered in &rendering.rendered {
        let heading = view_heading(ast, rendered.view);
        assert!(rendered.result.svg.starts_with("<svg"), "{heading}");
        assert!(rendered.result.svg.contains(heading), "{heading}");
    }
    // Each diagram names the file that declares its view.
    let physical = rendering.rendered.iter().find(|r| diagram_kind(ast, r.view) == DiagramKind::Physical).unwrap();
    assert!(physical.result.svg.contains("examples/coffee-machine/physical.sysml"));
}
