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
