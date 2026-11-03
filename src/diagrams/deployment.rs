//! The deployment view: the hosts, what is allocated to each, and the devices they are fitted in.
use super::connector_ends::{box_size, end_part, usage_label};
use crate::model::graph::{Compartment, DiagramGraph, GraphNode, box_of, qualified_name};
use crate::parser::ast::{Ast, NodeId, Prop, Type};
use crate::render::text_metrics::{Weight, measure_text};
use std::collections::HashMap;

/// A connector end as the *relationships* compartment names it, by qualified name
/// (§8.2.3.5): the model's chain `sw.recipeEngine` reads `sw::recipeEngine`.
fn end_name(ast: &Ast, end: NodeId) -> String {
    ast.texts(end, Prop::Segments).collect::<Vec<_>>().join("::")
}

/// A host: a «part» box listing what is allocated to it in its *relationships*
/// compartment (§8.2.3.5), `allocated sw::recipeEngine`.
fn host_node(ast: &Ast, host: NodeId, allocated: &[String]) -> GraphNode {
    let name = usage_label(ast, host);
    let lines = allocated.iter().map(|software| format!("allocated {software}")).collect();
    let compartments = vec![Compartment { title: Some("relationships".to_string()), lines }];
    let (shape, keyword) = box_of(ast, host);
    let (width, height) = box_size(&name, &compartments, 150.0, 40.0, keyword);
    GraphNode {
        id: qualified_name(ast, host),
        shape,
        keyword: keyword.to_string(),
        name,
        compartments,
        ports: Vec::new(),
        children: None,
        width,
        height,
    }
}

/// A physical device the hosts are fitted in, drawn as an assembly frame: the
/// device's one usage, or its definition when it has none or several.
fn device_frame(ast: &Ast, id: String, device: NodeId) -> GraphNode {
    let name = if ast[device].ty == Type::PartUsage {
        usage_label(ast, device)
    } else {
        ast.text(device, Prop::Name).unwrap_or("").to_string()
    };
    let (shape, keyword) = box_of(ast, device);
    GraphNode {
        id,
        shape,
        keyword: keyword.to_string(),
        width: 150f64.max(measure_text(&name, 12.0, Weight::Bold) + 40.0),
        name,
        compartments: Vec::new(),
        ports: Vec::new(),
        children: Some(Vec::new()),
        height: 60.0,
    }
}

/// Where a box goes: at the top, or into a device's frame.
#[derive(Clone, Copy)]
enum Place {
    Root,
    Frame(usize),
}

struct Extraction<'a> {
    ast: &'a Ast,
    usages_of: HashMap<NodeId, Vec<NodeId>>,
    /// Every frame and host, each with what is put in it, in the order made.
    boxes: Vec<(GraphNode, Vec<usize>)>,
    roots: Vec<usize>,
    frames: HashMap<NodeId, usize>,
}

impl Extraction<'_> {
    fn put(&mut self, node: GraphNode, place: Place) -> usize {
        self.boxes.push((node, Vec::new()));
        let at = self.boxes.len() - 1;
        match place {
            Place::Root => self.roots.push(at),
            Place::Frame(frame) => self.boxes[frame].1.push(at),
        }
        at
    }

    /// The frame of a device, nested in the frame of the device its one usage is in.
    fn frame_for(&mut self, def: NodeId, path: &[NodeId]) -> usize {
        if let Some(existing) = self.frames.get(&def) {
            return *existing;
        }
        let ast = self.ast;
        let uses = self.usages_of.get(&def).cloned().unwrap_or_default();
        let only = (uses.len() == 1).then(|| uses[0]);
        let frame = device_frame(ast, format!("device:{}", qualified_name(ast, def)), only.unwrap_or(def));
        // Made before it is placed, so a device that contains itself finds it.
        self.boxes.push((frame, Vec::new()));
        let at = self.boxes.len() - 1;
        self.frames.insert(def, at);
        let owner = only.and_then(|usage| ast[usage].parent);
        match owner {
            Some(owner) if ast[owner].ty == Type::PartDef && !path.contains(&owner) => {
                let mut path = path.to_vec();
                path.push(def);
                let outer = self.frame_for(owner, &path);
                self.boxes[outer].1.push(at);
            }
            _ => self.roots.push(at),
        }
        at
    }

    fn tree(&self, at: usize) -> GraphNode {
        let (node, contents) = &self.boxes[at];
        let mut node = node.clone();
        if node.children.is_some() {
            node.children = Some(contents.iter().map(|inner| self.tree(*inner)).collect());
        }
        node
    }
}

/// The Deployment view lists on each host what is allocated to it (§8.2.3.15).
/// Nothing is drawn inside a host, since a box drawn inside another is its member,
/// which software is not; and an «allocate» arrow per program would bury the hosts
/// under arrows. A host appears once something is allocated to it, and a program
/// allocated to two hosts is listed on both.
///
/// Hosts sit inside a frame for the device they are fitted in, borrowed from the
/// physical containment: both computers inside the drone, the tablet in the ground
/// station. Which programs run in the air and which on the ground then reads at a
/// glance. A device fitted exactly once is labelled by that usage
/// (`drone : SurveyDrone`) and nested in the frame of the device that contains it; a
/// top-level definition, or one fitted in several places, is a frame of its own. A
/// host declared directly in a package has no frame. `packages` are the model's.
pub fn extract_deployment_graph(ast: &Ast, exposed: &[NodeId], packages: &[NodeId]) -> DiagramGraph {
    let allocations: Vec<NodeId> = exposed
        .iter()
        .flat_map(|element| ast.with_all_contents(*element).into_iter().skip(1))
        .filter(|node| ast[*node].ty == Type::AllocationUsage)
        .collect();

    let mut usages_of: HashMap<NodeId, Vec<NodeId>> = HashMap::new();
    for node in packages.iter().flat_map(|package| ast.with_all_contents(*package)) {
        if ast[node].ty == Type::PartUsage
            && let Some(def) = ast.target(node, Prop::Type)
        {
            usages_of.entry(def).or_default().push(node);
        }
    }

    // What each host runs, in the order it was allocated.
    let mut hosts: Vec<(NodeId, Vec<String>)> = Vec::new();
    for allocation in allocations {
        let source = ast.child(allocation, Prop::Source);
        let Some(host) = end_part(ast, ast.child(allocation, Prop::Target)) else {
            continue;
        };
        let Some(source) = source else {
            continue;
        };
        if end_part(ast, Some(source)) == Some(host) {
            continue;
        }
        let software = end_name(ast, source);
        let at = match hosts.iter().position(|(other, _)| *other == host) {
            Some(at) => at,
            None => {
                hosts.push((host, Vec::new()));
                hosts.len() - 1
            }
        };
        if !hosts[at].1.contains(&software) {
            hosts[at].1.push(software);
        }
    }

    let mut extraction = Extraction { ast, usages_of, boxes: Vec::new(), roots: Vec::new(), frames: HashMap::new() };
    for (host, software) in &hosts {
        let node = host_node(ast, *host, software);
        let place = match ast[*host].parent {
            Some(device) if ast[device].ty == Type::PartDef => Place::Frame(extraction.frame_for(device, &[])),
            _ => Place::Root,
        };
        extraction.put(node, place);
    }

    let nodes = extraction.roots.iter().map(|root| extraction.tree(*root)).collect();
    DiagramGraph { nodes, edges: Vec::new() }
}
