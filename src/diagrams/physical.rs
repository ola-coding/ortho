//! The physical view: the product's parts, nested, with their ports and what connects them.
use super::connector_ends::{box_size, perform_compartments, usage_label};
use crate::model::graph::qualified_name;
use crate::model::graph::{
    Compartment, DiagramGraph, EdgeKind, GraphEdge, GraphNode, GraphPort, box_of, effective_name,
};
use crate::parser::ast::{Ast, NodeId, Prop, Type};
use std::collections::{HashMap, HashSet};

const HEADER_HEIGHT: f64 = 40.0;
const MIN_WIDTH: f64 = 110.0;

/// One part of the product: a box on the diagram, and the parts nested in it.
struct Instance {
    node: GraphNode,
    /// The usage this box stands for.
    usage: NodeId,
    /// Each nested part by name, as an index into the instances.
    children: HashMap<String, usize>,
    /// The nested parts in order, as indices into the instances.
    nested: Vec<usize>,
}

/// A connection, together with the part whose body declares it.
struct Pending {
    connection: NodeId,
    context: usize,
}

fn multiplicity_text(ast: &Ast, multiplicity: Option<NodeId>) -> String {
    let Some(multiplicity) = multiplicity else {
        return String::new();
    };
    let lower = ast.text(multiplicity, Prop::Lower).unwrap_or("");
    match ast.text(multiplicity, Prop::Upper) {
        Some(upper) => format!(" [{lower}..{upper}]"),
        None => format!(" [{lower}]"),
    }
}

/// First of each name wins, so a part's own members shadow what its type declares;
/// a redefinition, `part :>> engine`, takes the name it redefines.
fn unique_by_name(ast: &Ast, items: Vec<NodeId>) -> Vec<NodeId> {
    let mut seen = HashSet::new();
    items
        .into_iter()
        .filter(|item| match effective_name(ast, *item) {
            None | Some("") => true,
            Some(name) => seen.insert(name.to_string()),
        })
        .collect()
}

/// The members of type `ty` a definition declares or inherits through `:>`. A local
/// member shadows an inherited one of the same name, as it does in name resolution.
fn effective_members(ast: &Ast, def: NodeId, ty: Type, seen: &mut HashSet<NodeId>) -> Vec<NodeId> {
    if !seen.insert(def) {
        return Vec::new();
    }
    let mut members: Vec<NodeId> = ast.children(def, Prop::Members).filter(|member| ast[*member].ty == ty).collect();
    // Only a part def's supertypes are resolved; the rest are kept as written.
    if ast[def].ty == Type::PartDef {
        for supertype in ast.references_of(def, Prop::Supertypes) {
            if let Some(supertype) = ast.reference_target(supertype) {
                members.extend(effective_members(ast, supertype, ty, seen));
            }
        }
    }
    unique_by_name(ast, members)
}

fn members_of(ast: &Ast, def: Option<NodeId>, ty: Type) -> Vec<NodeId> {
    def.map(|def| effective_members(ast, def, ty, &mut HashSet::new())).unwrap_or_default()
}

fn of_type(ast: &Ast, body: &[NodeId], ty: Type) -> Vec<NodeId> {
    body.iter().copied().filter(|member| ast[*member].ty == ty).collect()
}

fn ports_of(ast: &Ast, ty: Option<NodeId>, body: &[NodeId]) -> Vec<NodeId> {
    let mut ports = of_type(ast, body, Type::PortUsage);
    ports.extend(members_of(ast, ty, Type::PortUsage));
    unique_by_name(ast, ports)
}

/// What a part performs: what its body says, plus what its type contributes.
fn performs_of(ast: &Ast, ty: Option<NodeId>, body: &[NodeId]) -> Vec<Compartment> {
    let mut members = body.to_vec();
    members.extend(members_of(ast, ty, Type::PerformUsage));
    perform_compartments(ast, &members)
}

/// A «part» box: `name : Type [n]`, its ports, and what it performs.
fn part_box(ast: &Ast, id: String, part: NodeId, ty: Option<NodeId>) -> GraphNode {
    let multiplicity = ast.children(part, Prop::Multiplicities).next();
    let name = format!("{}{}", usage_label(ast, part), multiplicity_text(ast, multiplicity));
    let body: Vec<NodeId> = ast.children(part, Prop::Members).collect();
    let performs = performs_of(ast, ty, &body);
    let (shape, keyword) = box_of(ast, part);
    let ports = ports_of(ast, ty, &body)
        .into_iter()
        .map(|port| {
            let name = effective_name(ast, port).unwrap_or("");
            GraphPort { id: format!("{id}#{name}"), label: name.to_string() }
        })
        .collect();
    let (width, height) = box_size(&name, &performs, MIN_WIDTH, HEADER_HEIGHT, keyword);
    GraphNode {
        element: Some(part),
        id,
        shape,
        keyword: keyword.to_string(),
        name,
        compartments: performs,
        ports,
        children: None,
        width,
        height,
    }
}

struct Extraction<'a> {
    ast: &'a Ast,
    instances: Vec<Instance>,
    pending: Vec<Pending>,
}

impl Extraction<'_> {
    fn add(&mut self, node: GraphNode, usage: NodeId) -> usize {
        self.instances.push(Instance { node, usage, children: HashMap::new(), nested: Vec::new() });
        self.instances.len() - 1
    }

    /// Adds a nested box for every part the owner's type declares or inherits, and
    /// every part in the usage's own body, recursively. `path` holds the types
    /// already being expanded, so a definition that contains itself is drawn once.
    fn expand(&mut self, owner: usize, ty: Option<NodeId>, body: &[NodeId], path: &[NodeId]) {
        let ast = self.ast;
        let mut parts = of_type(ast, body, Type::PartUsage);
        parts.extend(members_of(ast, ty, Type::PartUsage));
        for part in unique_by_name(ast, parts) {
            let part_type = ast.target(part, Prop::Type);
            let name = effective_name(ast, part).unwrap_or("");
            let id = format!("{}.{name}", self.instances[owner].node.id);
            let child = self.add(part_box(ast, id, part, part_type), part);
            self.instances[owner].children.insert(name.to_string(), child);
            self.instances[owner].nested.push(child);
            if part_type.is_some_and(|part_type| path.contains(&part_type)) {
                continue;
            }
            let members: Vec<NodeId> = ast.children(part, Prop::Members).collect();
            let mut path = path.to_vec();
            path.extend(part_type);
            self.expand(child, part_type, &members, &path);
        }
        let mut connections = of_type(ast, body, Type::ConnectionUsage);
        connections.extend(members_of(ast, ty, Type::ConnectionUsage));
        for connection in connections {
            self.pending.push(Pending { connection, context: owner });
        }
    }

    /// Walks a connector end (`drone.rfLink`, `radio.rf`) from the part whose body
    /// declares the connection down to the box it names, and the port on it.
    /// Resolving per instance is the point: the drone's radio and the ground
    /// station's radio share a type but are two boxes, each wired on its own.
    fn resolve_end(&self, end: Option<NodeId>, context: usize) -> Option<(String, Option<String>)> {
        let ast = self.ast;
        let mut current = context;
        let segments = end.map(|end| ast.references_of(end, Prop::Segments)).unwrap_or_default();
        for segment in segments {
            let Some(feature) = ast.reference_target(segment) else {
                break;
            };
            let name = effective_name(ast, feature).unwrap_or("");
            match ast[feature].ty {
                Type::PartUsage => {
                    let child = *self.instances[current].children.get(name)?;
                    if self.instances[child].usage != feature {
                        return None;
                    }
                    current = child;
                }
                Type::PortUsage => {
                    let node = &self.instances[current].node;
                    let port_id = format!("{}#{name}", node.id);
                    let on_box = node.ports.iter().any(|port| port.id == port_id);
                    return Some((node.id.clone(), on_box.then_some(port_id)));
                }
                _ => break,
            }
        }
        Some((self.instances[current].node.id.clone(), None))
    }

    /// The box of an instance with the boxes nested in it.
    fn tree(&mut self, instance: usize) -> GraphNode {
        let nested = std::mem::take(&mut self.instances[instance].nested);
        let children: Vec<GraphNode> = nested.iter().map(|child| self.tree(*child)).collect();
        let mut node = self.instances[instance].node.clone();
        if !children.is_empty() {
            node.children = Some(children);
        }
        node
    }
}

/// The Physical view is the product's topology, drawn as an internal block diagram:
/// one box per part *in the product*, nested inside the part that contains it, with
/// ports on the box edges and a line only where something is connected, cabled or
/// piped. Containment is shown by nesting, so there are no composition edges, and no
/// attribute compartments: the view is about what connects to what, not about
/// specification values.
///
/// Drawing instances rather than definitions matters: a type used twice (the same IP
/// radio in the drone and in the ground station) becomes two boxes, and each
/// connection lands on the part it actually names.
///
/// The top-level boxes are the exposed parts.
pub fn extract_physical_graph(ast: &Ast, exposed: &[NodeId]) -> DiagramGraph {
    let mut extraction = Extraction { ast, instances: Vec::new(), pending: Vec::new() };
    let mut roots = Vec::new();
    for usage in exposed.iter().copied().filter(|element| ast[*element].ty == Type::PartUsage) {
        let ty = ast.target(usage, Prop::Type);
        let root = extraction.add(part_box(ast, qualified_name(ast, usage), usage, ty), usage);
        let members: Vec<NodeId> = ast.children(usage, Prop::Members).collect();
        extraction.expand(root, ty, &members, &Vec::from_iter(ty));
        roots.push(root);
    }

    let mut edges: Vec<GraphEdge> = Vec::new();
    for pending in &extraction.pending {
        let source = extraction.resolve_end(ast.child(pending.connection, Prop::Source), pending.context);
        let target = extraction.resolve_end(ast.child(pending.connection, Prop::Target), pending.context);
        let (Some(source), Some(target)) = (source, target) else {
            continue;
        };
        if source == target {
            continue;
        }
        // A connection is labelled as declared (§8.2.3.13): `: Interface` for an
        // unnamed typed one.
        let label = usage_label(ast, pending.connection);
        let mut edge = GraphEdge::new(format!("e{}", edges.len()), EdgeKind::Connection, source.0, target.0);
        edge.source_port_id = source.1;
        edge.target_port_id = target.1;
        edge.label = (!label.is_empty()).then_some(label);
        edges.push(edge);
    }

    let nodes = roots.into_iter().map(|root| extraction.tree(root)).collect();
    DiagramGraph { nodes, edges }
}
