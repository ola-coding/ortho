//! The survey drone example: each of its six views draws what the model says.
use ortho::diagrams::deployment::extract_deployment_graph;
use ortho::diagrams::implementation::extract_implementation_graph;
use ortho::diagrams::logical::extract_logical_graph;
use ortho::diagrams::physical::extract_physical_graph;
use ortho::diagrams::sequence::extract_sequence_model;
use ortho::diagrams::use_case::extract_use_case_graph;
use ortho::model::graph::{DiagramGraph, EdgeKind, NodeShape, is_use_case_node};
use ortho::model::views::{DiagramKind, diagram_kind, view_heading};
use ortho::parser::ast::Prop;
use ortho::parser::workspace::Workspace;
use ortho::pipeline::render_views;
use ortho::render::theme::LIGHT;
use std::collections::HashSet;

mod common;
use common::{all_nodes, compartment, exposed_by_view, lines_of, names, sorted_names};

/// One file per view.
const ALL: &[&str] =
    &["use-case.sysml", "logical.sysml", "implementation.sysml", "physical.sysml", "deployment.sysml", "process.sysml"];

/// The view whose file declares everything it draws, and names no other.
const SELF_CONTAINED: &[&str] = &["use-case.sysml"];

fn parse_set(files: &[&str]) -> Workspace {
    common::example("survey-drone", files)
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
        [
            "SystemUseCases",
            "Functions",
            "Apis",
            "Ground",
            "Onboard",
            "Middleware",
            "Flight",
            "FlightPlatform",
            "Hardware",
            "Deployment",
            "SurveyMissionScenario"
        ]
    );
}

#[test]
fn each_self_contained_view_parses_on_its_own() {
    for file in SELF_CONTAINED {
        parse_set(&[file]);
    }
}

#[test]
fn use_case_view_one_operator_three_use_cases_surveying_includes_both() {
    let workspace = parse_set(&["use-case.sysml"]);
    let graph = extract_use_case_graph(&workspace.ast, &exposed_by_view(&workspace));
    assert_eq!(
        sorted_names(graph.nodes.iter().filter(|n| n.keyword == "use case")),
        ["followProgress : SurveyTask", "planSurvey : SurveyTask", "surveyArea : SurveyTask"]
    );
    let subjects: Vec<_> = graph.nodes.iter().filter(|n| n.keyword == "subject").collect();
    assert_eq!(names(subjects.iter().copied()), ["system : SurveyDroneSystem"]);
    assert_eq!(graph.edges.iter().filter(|e| e.target_id == subjects[0].id).count(), 3);

    assert_eq!(names(graph.nodes.iter().filter(|n| n.shape == NodeShape::Actor)), ["operator : Operator"]);

    let mut includes: Vec<&str> =
        graph.edges.iter().filter(|e| e.kind == EdgeKind::Include).map(|e| e.target_id.as_str()).collect();
    includes.sort();
    assert_eq!(includes, ["SystemUseCases::followProgress", "SystemUseCases::planSurvey"]);

    // The operator comes through the shared use case def, so they are a primary
    // actor of all three: planning and watching stand on their own as well as inside
    // a survey.
    let associations = graph
        .edges
        .iter()
        .filter(|e| e.kind == EdgeKind::Association && e.source_id == "actor:operator : SystemUseCases::Operator");
    assert_eq!(associations.count(), 3);
}

#[test]
fn logical_view_a_strict_capability_tree_naming_nothing_that_realizes_a_function() {
    let (_, graph) = logical();
    let functions = graph.nodes.iter().filter(|n| !is_use_case_node(&n.keyword)).count();
    assert_eq!(graph.edges.iter().filter(|e| e.kind == EdgeKind::Decomposition).count(), functions - 1);
    assert_eq!(functions, 20);

    let all = names(&graph.nodes);
    // Every onboard module answers to a function here, supervising the system
    // included.
    for function in ["avoidObstacles", "maintainWorldModel", "superviseSystem"] {
        assert!(all.contains(&function), "{function}");
    }
    // Nothing that performs a function appears on this view. (`supplyPower` is a
    // function; `LiPoBattery` is the thing that does it.)
    for realization in
        ["LiPoBattery", "GpuComputer", "ControlComputer", "RoutePlanner", "ZenohRouter", "MissionExecutor"]
    {
        assert!(!all.contains(&realization), "{realization}");
    }

    let children: HashSet<&str> = graph.edges.iter().map(|e| e.target_id.as_str()).collect();
    assert_eq!(names(graph.nodes.iter().filter(|n| !children.contains(n.id.as_str()))), ["PerformAerialSurvey"]);

    // `action survey : PerformAerialSurvey` is what the realizing parts point into.
    // It is one use of the tree, not a second tree.
    assert!(!all.contains(&"survey"));
}

#[test]
fn logical_view_the_tree_refines_the_survey_and_the_view_draws_the_tree_alone() {
    let (workspace, graph) = logical();
    // Surveying an area includes planning it and following it, so the tree that
    // details the survey details those too.
    let refinements = common::refinements(&workspace);
    assert_eq!(refinements, [("Functions::PerformAerialSurvey".to_string(), "SystemUseCases::surveyArea".to_string())]);
    // The view draws the tree alone; use-case.sysml draws the use cases.
    assert!(!graph.nodes.iter().any(|n| is_use_case_node(&n.keyword)));
    assert!(!graph.edges.iter().any(|e| e.kind == EdgeKind::Dependency));
}

#[test]
fn implementation_view_each_module_lists_the_functions_it_realizes() {
    let graph = implementation();
    let all = all_nodes(&graph.nodes);
    let compartment_of = |name: &str, title: &str| lines_of(all.iter().find(|n| n.name == name).unwrap(), title);

    assert_eq!(compartment_of("VideoStreamer", "perform actions"), ["streamVideo"]);
    assert_eq!(compartment_of("SurveyPlanner", "perform actions"), ["planMission", "presentProgress"]);

    // A platform module realizes nothing of the function tree: it is what the modules
    // above it stand on. It still has an API of its own.
    for platform in ["RealTimeKernel", "ZenohRouter", "HardwareAbstraction"] {
        assert_eq!(compartment_of(platform, "perform actions"), Vec::<&str>::new(), "{platform}");
    }
    assert_eq!(compartment_of("RealTimeKernel", "ports"), ["scheduler : SchedulerApi"]);

    // The containers publish and subscribe: one topic, two ends.
    assert_eq!(compartment_of("VideoStreamer", "ports"), ["video : VideoTopic"]);
    assert_eq!(compartment_of("Diagnostics", "ports"), ["health : HealthTopic", "video : ~VideoTopic"]);

    // The function tree is a companion file, not part of this view.
    assert!(!names(&graph.nodes).contains(&"Functions"));
}

#[test]
fn implementation_view_the_firmware_stack_and_the_linux_stack_never_import_each_other() {
    let graph = implementation();
    let mut imports: Vec<String> = graph.edges.iter().map(|e| format!("{} -> {}", e.source_id, e.target_id)).collect();
    imports.sort();
    assert_eq!(imports, ["Flight -> FlightPlatform", "Ground -> Middleware", "Onboard -> Middleware",]);

    // The two stacks meet over MAVLink at runtime, never through a compile-time
    // dependency: the firmware knows nothing of Zenoh or Linux, and nothing in the
    // Linux stack builds against the firmware.
    let firmware = ["Flight", "FlightPlatform"];
    for edge in &graph.edges {
        assert_eq!(firmware.contains(&edge.source_id.as_str()), firmware.contains(&edge.target_id.as_str()));
    }
}

#[test]
fn physical_view_the_product_as_nested_parts_each_radio_wired_on_its_own() {
    let graph = physical();
    assert_eq!(names(&graph.nodes), ["system : SurveyDroneSystem"]);
    let devices = graph.nodes[0].children.as_deref().unwrap();
    assert_eq!(names(devices), ["drone : SurveyDrone", "ground : GroundStation"]);
    assert_eq!(devices[0].children.as_deref().unwrap().len(), 8);
    assert_eq!(graph.edges.len(), 11);
    assert!(graph.edges.iter().all(|e| e.kind == EdgeKind::Connection));

    let wires: Vec<(&str, &str)> = graph
        .edges
        .iter()
        .map(|e| (e.source_port_id.as_deref().unwrap_or(""), e.target_port_id.as_deref().unwrap_or("")))
        .collect();
    // The same radio type is fitted at both ends: two boxes, and the camera's video
    // reaches the companion computer, not the tablet.
    assert!(wires.contains(&("Hardware::system.drone.camera#videoOut", "Hardware::system.drone.companion#cameraIn")));
    assert!(wires.contains(&("Hardware::system.ground.tablet#network", "Hardware::system.ground.radio#ethernet")));

    // The air-to-ground link is the one connection typed by an interface.
    let typed: Vec<(Option<&str>, Option<&str>)> = graph
        .edges
        .iter()
        .filter(|e| e.label.as_deref() == Some(": RadioLink"))
        .map(|e| (e.source_port_id.as_deref(), e.target_port_id.as_deref()))
        .collect();
    assert_eq!(typed, [(Some("Hardware::system.drone.radio#rf"), Some("Hardware::system.ground.radio#rf"))]);
}

#[test]
fn physical_view_a_part_inherits_what_its_definition_performs() {
    let graph = physical();
    let all = all_nodes(&graph.nodes);
    let compartments = |name: &str| &all.iter().find(|n| n.name == name).unwrap().compartments;

    // The perform sits on PropulsionUnit; the box is the usage.
    assert_eq!(*compartments("motors : PropulsionUnit [4]"), [compartment("perform actions", &["produceThrust"])]);
    // The motors and the board that drives them produce thrust together:
    // realization is many to many.
    assert_eq!(compartments("esc : FourInOneEsc")[0].lines, ["produceThrust"]);
    // A radio performs a whole branch of the tree.
    assert_eq!(compartments("radio : IpRadio")[0].lines, ["communicate"]);
    // The computers perform theirs through the software deployed on them.
    assert!(compartments("fc : ControlComputer").is_empty());
    assert!(compartments("companion : GpuComputer").is_empty());
}

#[test]
fn deployment_view_every_module_hosted_on_three_computers_in_two_devices() {
    let workspace =
        parse_set(&["deployment.sysml", "implementation.sysml", "physical.sysml", "logical.sysml", "use-case.sysml"]);
    let graph = extract_deployment_graph(&workspace.ast, &exposed_by_view(&workspace), &workspace.packages());
    assert!(graph.edges.is_empty());

    assert_eq!(names(&graph.nodes), ["system : SurveyDroneSystem"]);
    let devices = graph.nodes[0].children.as_deref().unwrap();
    let drone = devices.iter().find(|c| c.name == "drone : SurveyDrone").unwrap();
    let mut computers: Vec<&str> = drone.children.as_deref().unwrap().iter().map(|c| c.id.as_str()).collect();
    computers.sort();
    assert_eq!(computers, ["Hardware::SurveyDrone::companion", "Hardware::SurveyDrone::fc"]);
    assert!(devices.iter().any(|c| c.id == "device:Hardware::GroundStation"));

    // A host is a «part» listing what is allocated to it.
    let hosts: Vec<_> = all_nodes(&graph.nodes)
        .into_iter()
        .filter(|n| n.compartments.iter().any(|c| c.title.as_deref() == Some("relationships")))
        .collect();
    assert_eq!(hosts.len(), 3);
    assert!(hosts.iter().all(|h| h.keyword == "part"));

    // The flight computer runs the firmware and nothing else: no container, no
    // router, not a line of the Linux stack.
    let fc = hosts.iter().find(|n| n.id == "Hardware::SurveyDrone::fc").unwrap();
    assert_eq!(
        fc.compartments[0].lines,
        [
            "allocated sw::missionExecutor",
            "allocated sw::stateEstimator",
            "allocated sw::flightControl",
            "allocated sw::mavlinkInterface",
            "allocated sw::hardwareAbstraction",
            "allocated sw::realTimeKernel",
        ]
    );

    // A module missing from this view would be a program with nowhere to run, so the
    // count is the whole of `sw`, one usage of every module.
    let placed: Vec<&String> = hosts.iter().flat_map(|n| n.compartments.iter().flat_map(|c| &c.lines)).collect();
    assert_eq!(placed.len(), 16);
    assert_eq!(placed.iter().collect::<HashSet<_>>().len(), 16);
}

#[test]
fn process_view_a_survey_mission_uploaded_over_the_mavlink_mission_protocol() {
    let workspace = parse_set(&["process.sysml", "implementation.sysml", "logical.sysml", "use-case.sysml"]);
    let sequence = extract_sequence_model(&workspace.ast, &exposed_by_view(&workspace));
    assert_eq!(sequence.lifelines.len(), 6);
    // The operator starts it: the actor of the use cases.
    assert_eq!(sequence.lifelines[0].label, "operator : Operator");
    assert_eq!(sequence.messages[0].label, "markArea");

    // The mission travels down both stacks, and the vehicle acknowledges it.
    let labels: Vec<&str> = sequence.messages.iter().map(|m| m.label.as_str()).collect();
    let handshake = [
        "uploadMission of MissionItems",
        "missionUpload of MissionItems",
        "storeMission of MissionItems",
        "missionAck of MissionAck",
    ];
    let positions: Vec<usize> = handshake.iter().map(|l| labels.iter().position(|m| m == l).unwrap()).collect();
    assert!(positions.windows(2).all(|pair| pair[0] < pair[1]), "{positions:?}");

    // Self-messages: the work nobody has to be asked for.
    let to_itself: Vec<&str> =
        sequence.messages.iter().filter(|m| m.source_id == m.target_id).map(|m| m.label.as_str()).collect();
    assert_eq!(to_itself, ["computeSurveyLegs", "flySurveyLegs", "returnToLaunch"]);
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
            (DiagramKind::UseCase, "use-case-view-survey-drone.svg"),
            (DiagramKind::Logical, "logical-view-survey-drone.svg"),
            (DiagramKind::Implementation, "implementation-view-survey-drone.svg"),
            (DiagramKind::Physical, "physical-view-survey-drone.svg"),
            (DiagramKind::Deployment, "deployment-view-survey-drone.svg"),
            (DiagramKind::Process, "process-view-survey-drone.svg"),
        ]
    );
    for rendered in &rendering.rendered {
        let heading = view_heading(ast, rendered.view);
        assert!(rendered.result.svg.starts_with("<svg"), "{heading}");
        assert!(rendered.result.svg.contains(heading), "{heading}");
    }
    // Each diagram names the file that declares its view.
    let physical = rendering.rendered.iter().find(|r| diagram_kind(ast, r.view) == DiagramKind::Physical).unwrap();
    assert!(physical.result.svg.contains("examples/survey-drone/physical.sysml"));
}
