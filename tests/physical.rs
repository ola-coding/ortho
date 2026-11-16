//! The physical view: the product as nested parts, wired port to port.
use ortho::diagrams::physical::extract_physical_graph;
use ortho::layout::elk::{LayoutOptions, layout_graph};
use ortho::layout::{LaidOutDiagram, LaidOutNode, PORT_SIZE};
use ortho::model::graph::{BOX_HEADER_HEIGHT, DiagramGraph, EdgeKind, NodeShape, compartments_height};
use ortho::parser::workspace::Workspace;
use ortho::render::svg::render_svg;
use ortho::render::theme::LIGHT;

mod common;
use common::{all_nodes, centre, close_to, compartment, exposed_by_view, fixture, names, parse_ok, title};

fn graph_of(workspace: &Workspace) -> DiagramGraph {
    extract_physical_graph(&workspace.ast, &exposed_by_view(workspace))
}

fn laid_out(graph: &DiagramGraph) -> LaidOutDiagram {
    layout_graph(graph, LayoutOptions::default()).unwrap()
}

fn node<'a>(laid_out: &'a LaidOutDiagram, prefix: &str) -> &'a LaidOutNode {
    laid_out.nodes.iter().find(|n| n.name.starts_with(prefix)).unwrap_or_else(|| panic!("no {prefix}"))
}

#[test]
fn draws_the_product_not_its_types_one_box_per_part_nested_in_its_owner() {
    let graph = graph_of(&fixture("physical.sysml"));
    // The exposed quadcopter is the one top-level box, carrying what its type
    // inherits from Drone.
    assert_eq!(names(&graph.nodes), ["quadcopter : Quadcopter"]);
    let children = graph.nodes[0].children.as_deref().unwrap();
    assert_eq!(names(children), ["fc : FlightController", "battery : Battery", "motors : Motor [4]"]);
    // Port defs are not boxes; ports show as markers on the parts.
    assert!(!names(all_nodes(&graph.nodes)).contains(&"PowerPort"));
    let mut ports: Vec<&str> = children[0].ports.iter().map(|p| p.label.as_str()).collect();
    ports.sort();
    assert_eq!(ports, ["motorCtrl", "powerIn"]);
}

#[test]
fn draws_every_box_as_a_part_with_no_attributes_or_type_level_edges() {
    let graph = graph_of(&fixture("physical.sysml"));
    for node in all_nodes(&graph.nodes) {
        assert!(node.compartments.is_empty(), "{}", node.name);
        assert_eq!((node.shape, node.keyword.as_str()), (NodeShape::Usage, "part"), "{}", node.name);
    }
    assert!(graph.edges.iter().all(|e| e.kind == EdgeKind::Connection));
}

#[test]
fn carries_no_requirements_the_view_is_the_product_not_its_specification() {
    let graph = graph_of(&fixture("physical.sysml"));
    // The fixture declares PowerBudget, powerReq and a satisfy; none of them belong
    // to any of the six views, so none of them reach this graph.
    assert!(!names(all_nodes(&graph.nodes)).iter().any(|n| n.contains("PowerBudget") || n.contains("powerReq")));
}

#[test]
fn wires_each_connection_between_the_ports_of_the_parts_it_names() {
    let graph = graph_of(&fixture("physical.sysml"));
    let wires: Vec<(Option<&str>, Option<&str>)> =
        graph.edges.iter().map(|e| (e.source_port_id.as_deref(), e.target_port_id.as_deref())).collect();
    assert_eq!(
        wires,
        [
            (Some("DronePhysical::quadcopter.battery#powerOut"), Some("DronePhysical::quadcopter.fc#powerIn")),
            (Some("DronePhysical::quadcopter.fc#motorCtrl"), Some("DronePhysical::quadcopter.motors#ctrlIn")),
        ]
    );
}

#[test]
fn lays_each_part_out_inside_its_owner_and_renders_it() {
    let graph = graph_of(&fixture("physical.sysml"));
    let laid_out = laid_out(&graph);
    let by_id = |id: &str| laid_out.nodes.iter().find(|n| n.id == id).unwrap();
    let owner = by_id("DronePhysical::quadcopter");
    for child in graph.nodes[0].children.as_deref().unwrap() {
        let b = by_id(&child.id);
        assert!(b.x >= owner.x && b.y >= owner.y);
        assert!(b.x + b.width <= owner.x + owner.width + 0.5);
        assert!(b.y + b.height <= owner.y + owner.height + 0.5);
    }
    // ELK placed every port label, so none is left to the fallback.
    for node in laid_out.nodes.iter().filter(|n| !n.ports.is_empty()) {
        assert!(node.ports.iter().all(|p| p.label_at.is_some()), "{}", node.name);
    }

    let svg = render_svg(&laid_out, &title("Physical view — DronePhysical", "fixtures/physical.sysml"), &LIGHT);
    assert!(svg.contains("<svg"));
    assert!(svg.contains(">powerIn</text>"));
    assert!(!svg.contains("part def"));
}

#[test]
fn widens_a_box_to_a_name_wider_than_its_contents_and_centres_them_under_it() {
    let graph = graph_of(&parse_ok(
        "package Hardware {
            private import StandardViewDefinitions::*;
            port def RfPort;
            part def Radio {
                port rf : RfPort;
            }
            part def GroundStation {
                port rfLink : RfPort;
                part radio : Radio;
                connect radio.rf to rfLink;
            }
            part def Site {
                part primaryGroundStation : GroundStation;
            }
            part site : Site;
            view 'Site' : InterconnectionView {
                expose site;
            }
        }",
    ));
    let name = "primaryGroundStation : GroundStation";
    let name_width = all_nodes(&graph.nodes).into_iter().find(|n| n.name == name).unwrap().width;
    let laid_out = laid_out(&graph);

    let station = node(&laid_out, name);
    let radio = node(&laid_out, "radio : Radio");
    assert_eq!(station.width, name_width);
    assert!(station.height < name_width);
    assert!(close_to(centre(radio.x, radio.width), centre(station.x, station.width), 2));
    // The wire still reaches the port on the box's border.
    let port = &station.ports[0];
    let end = laid_out.edges[0].points.last().unwrap();
    assert!(close_to(end.x, station.x + port.x + PORT_SIZE / 2.0, 2));
}

#[test]
fn keeps_the_parts_of_an_assembly_below_the_compartment_of_what_it_performs() {
    // No example has an assembly that performs a function (the coffee machine's
    // serving moved onto the parts that pour), so the layout that reserves room for
    // a container's own compartment is covered here.
    let graph = graph_of(&parse_ok(
        "package M {
            action def Job { action serve; }
            action job : Job;
            part def Inner;
            part def Housing {
                perform M::job.serve;
                part inner : Inner;
            }
            part housing : Housing;
            view v : InterconnectionView { expose housing; }
        }",
    ));
    let laid_out = laid_out(&graph);
    let housing = node(&laid_out, "housing");
    let inner = node(&laid_out, "inner");
    assert_eq!(housing.compartments, [compartment("perform actions", &["serve"])]);
    assert!(inner.y > housing.y + BOX_HEADER_HEIGHT + compartments_height(&housing.compartments));
}

#[test]
fn keeps_parts_wired_to_one_hub_in_declared_order() {
    let parts: String = (0..8).map(|i| format!("part p{i} : Unit; ")).collect();
    let wires: String = (0..8).map(|i| format!("connect hub.feed to p{i}.supply; ")).collect();
    let workspace = parse_ok(&format!(
        "package H {{
            port def P;
            part def Unit {{ port supply : P; }}
            part def Hub {{ port feed : P; }}
            part def Machine {{ part hub : Hub; {parts} {wires} }}
            part machine : Machine;
            view 'Star' : InterconnectionView {{ expose machine; }}
        }}"
    ));
    let laid_out = laid_out(&graph_of(&workspace));
    let mut fed: Vec<&LaidOutNode> = laid_out.nodes.iter().filter(|n| n.name.starts_with('p')).collect();
    fed.sort_by(|a, b| a.y.total_cmp(&b.y).then(a.x.total_cmp(&b.x)));
    let order: Vec<&str> = fed.iter().map(|n| n.name.split(' ').next().unwrap()).collect();
    assert_eq!(order, ["p0", "p1", "p2", "p3", "p4", "p5", "p6", "p7"]);
}

/// A tank feeding a pump, a pump feeding a heater, and a board and the pump
/// exchanging commands and status over one cable; `flows` declared in the machine.
fn flowing(flows: &str) -> DiagramGraph {
    graph_of(&parse_ok(&format!(
        "package F {{
            item def Water;
            item def Command;
            item def Status;
            port def WaterPort {{ out item water : Water; }}
            port def ControlPort {{ out item command : Command; in item status : Status; }}
            part def Tank {{ port waterOut : WaterPort; }}
            part def Pump {{ port waterIn : ~WaterPort; port waterOut : WaterPort; port ctrl : ~ControlPort; }}
            part def Heater {{ port waterIn : ~WaterPort; }}
            part def Board {{ port ctrl : ControlPort; }}
            part def Machine {{
                part tank : Tank;
                part pump : Pump;
                part heater : Heater;
                part board : Board;
                connect tank.waterOut to pump.waterIn;
                connect board.ctrl to pump.ctrl;
                {flows}
            }}
            part machine : Machine;
            view 'Physical' : InterconnectionView {{ expose machine; }}
        }}"
    )))
}

fn flows_on(graph: &DiagramGraph, source_port: &str) -> Vec<(String, bool)> {
    let edge = graph.edges.iter().find(|e| e.source_port_id.as_deref().is_some_and(|p| p.ends_with(source_port)));
    edge.unwrap().flows.iter().map(|flow| (flow.label.clone(), flow.forward)).collect()
}

#[test]
fn draws_a_flow_between_connected_ports_on_their_line_named_by_what_flows() {
    // §8.2.3.16 flow-on-connection: an arrowhead on the connection, labelled with the
    // payload, pointing the way the flow goes, which may be against the line's own way.
    let graph = flowing(
        "flow of Water from tank.waterOut.water to pump.waterIn.water;
         flow of Command from board.ctrl.command to pump.ctrl.command;
         flow statusReport of Status from pump.ctrl.status to board.ctrl.status;",
    );
    assert_eq!(graph.edges.len(), 2, "a flow on a connection adds no line");
    assert!(graph.edges.iter().all(|e| e.kind == EdgeKind::Connection));
    assert_eq!(flows_on(&graph, "tank#waterOut"), [("Water".to_string(), true)]);
    assert_eq!(flows_on(&graph, "board#ctrl"), [("Command".to_string(), true), ("Status".to_string(), false)]);
}

#[test]
fn draws_a_flow_from_a_port_or_a_feature_in_it_alike_and_one_with_no_payload_by_its_name() {
    let graph = flowing("flow supply from tank.waterOut to pump.waterIn;");
    assert_eq!(flows_on(&graph, "tank#waterOut"), [("supply".to_string(), true)]);
}

#[test]
fn draws_a_flow_with_no_connection_as_a_line_of_its_own() {
    // A flow connects its ends by itself (§7.16.2): a line with a filled arrowhead,
    // labelled `name of Payload` (§8.2.3.16).
    let graph = flowing(
        "flow feed of Water from pump.waterOut.water to heater.waterIn.water;
         flow of Water from pump.waterOut to heater.waterIn;",
    );
    let flows: Vec<_> = graph.edges.iter().filter(|e| e.kind == EdgeKind::Flow).collect();
    assert_eq!(flows.len(), 2);
    assert_eq!(flows[0].label.as_deref(), Some("feed of Water"));
    assert_eq!(flows[1].label.as_deref(), Some("of Water"));
    assert_eq!(flows[0].source_port_id.as_deref(), Some("F::machine.pump#waterOut"));
    assert_eq!(flows[0].target_port_id.as_deref(), Some("F::machine.heater#waterIn"));
    let svg = render_svg(&laid_out(&graph), &title("Physical", "model.sysml"), &LIGHT);
    assert!(svg.contains("feed of Water"));
}

#[test]
fn places_each_flow_name_beside_its_arrowhead_clear_of_everything_else_in_the_same_layout() {
    // The coffee machine's flows run along its pipes and its mains cable.
    let workspace = common::example("coffee-machine", &["physical.sysml", "logical.sysml", "use-case.sysml"]);
    let graph = graph_of(&workspace);
    let laid_out = laid_out(&graph);
    let mut without = graph.clone();
    without.edges.iter_mut().for_each(|e| e.flows.clear());
    let plain = layout_graph(&without, LayoutOptions::default()).unwrap();
    // The names are placed in free space after the layout, so it is as without them.
    assert_eq!((laid_out.width, laid_out.height), (plain.width, plain.height));

    type Area = (f64, f64, f64, f64);
    let overlap = |a: Area, b: Area| a.0 < b.2 && b.0 < a.2 && a.1 < b.3 && b.1 < a.3;
    let mut drawn: Vec<(String, Area)> = Vec::new();
    for n in laid_out.nodes.iter().filter(|n| !n.has_children) {
        drawn.push((n.name.clone(), (n.x, n.y, n.x + n.width, n.y + n.height)));
        for p in &n.ports {
            let (px, py) = (n.x + p.x, n.y + p.y);
            drawn.push((p.label.clone(), (px, py, px + PORT_SIZE, py + PORT_SIZE)));
            if let Some((lx, ly)) = p.label_at {
                drawn.push((format!("{} label", p.label), (px + lx, py + ly, px + lx + 20.0, py + ly + 11.0)));
            }
        }
    }
    let mut flows = 0;
    for edge in &laid_out.edges {
        for flow in &edge.flows {
            let b = flow.label_box.as_ref().unwrap_or_else(|| panic!("{} has no place", flow.label));
            let label = (b.x, b.y, b.x + b.width, b.y + b.height);
            let (at, _) = flow.arrow.unwrap();
            assert!(edge.points.windows(2).any(|pair| {
                let (a, z) = (pair[0], pair[1]);
                (at.x - a.x) * (z.y - a.y) == (at.y - a.y) * (z.x - a.x)
            }));
            for (what, area) in &drawn {
                assert!(!overlap(label, *area), "{} touches {what}", flow.label);
            }
            drawn.push((flow.label.clone(), label));
            flows += 1;
        }
    }
    assert_eq!(flows, 8);
}
