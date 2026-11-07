//! The implementation view: packages of software modules, and what they import.
use super::connector_ends::{box_size, perform_compartments, type_name};
use crate::model::graph::{
    Compartment, DiagramGraph, EdgeKind, GraphEdge, GraphNode, NodeShape, box_of, qualified_name,
};
use crate::model::packages::collect_packages;
use crate::parser::ast::{Ast, NodeId, Prop, Type};
use crate::render::text_metrics::{Weight, measure_text};
use std::collections::HashSet;

/// A module's API, as the spec's *ports* compartment (Table 10): one line per port,
/// `name : PortDef`, and `~PortDef` where the port is conjugated, the API the module
/// needs rather than the one it offers (§7.12.3). No squares on the border: that is
/// how a port is drawn where a wire lands on it, and this view draws no wires
/// between modules.
fn ports_compartment(ast: &Ast, def: NodeId) -> Vec<Compartment> {
    let ports = ast.children(def, Prop::Members).filter(|member| ast[*member].ty == Type::PortUsage);
    let lines: Vec<String> = ports
        .map(|port| {
            let name = ast.text(port, Prop::Name).unwrap_or("");
            match type_name(ast, port) {
                Some(ty) => {
                    let conjugated = if ast.flag(port, Prop::Conjugated) { "~" } else { "" };
                    format!("{name} : {conjugated}{ty}")
                }
                None => name.to_string(),
            }
        })
        .collect();
    if lines.is_empty() { Vec::new() } else { vec![Compartment { title: Some("ports".to_string()), lines }] }
}

/// A module's heading as declared (§8.2.3.6): its name, and what it specializes,
/// such as the vendor's service it is built on: `OrderTable :> DynamoDB`.
fn module_heading(ast: &Ast, def: NodeId) -> String {
    let name = ast.text(def, Prop::Name).unwrap_or("");
    let supertypes: Vec<&str> = ast
        .references_of(def, Prop::Supertypes)
        .into_iter()
        .map(|reference| {
            let named = ast.reference_target(reference).and_then(|target| ast.text(target, Prop::Name));
            named.unwrap_or_else(|| ast.reference_text(reference))
        })
        .collect();
    if supertypes.is_empty() { name.to_string() } else { format!("{name} :> {}", supertypes.join(", ")) }
}

fn module_node(ast: &Ast, def: NodeId) -> GraphNode {
    let members: Vec<NodeId> = ast.children(def, Prop::Members).collect();
    let mut compartments = perform_compartments(ast, &members);
    compartments.extend(ports_compartment(ast, def));
    let (shape, keyword) = box_of(ast, def);
    let name = module_heading(ast, def);
    let (width, height) = box_size(&name, &compartments, 120.0, 40.0, keyword);
    GraphNode {
        element: Some(def),
        id: qualified_name(ast, def),
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

/// A package this view draws: one holding something part-like, a module or a
/// deployable part, at any depth. The view is rendered with `logical.sysml`
/// alongside, because the modules name the functions they perform, and a package of
/// functions, or of requirements, is not software.
fn holds_parts(ast: &Ast, package: NodeId) -> bool {
    ast.children(package, Prop::Members).any(|member| match ast[member].ty {
        Type::PartDef | Type::PartUsage => true,
        Type::PackageDecl => holds_parts(ast, member),
        _ => false,
    })
}

/// A package box, with its sub-packages and the modules it declares drawn inside it.
/// A module is a part def. A package-level part usage is an instance, the tree of
/// deployable parts the deployment view allocates from, not a module, so it is left
/// off this view.
fn package_node(ast: &Ast, package: NodeId) -> GraphNode {
    let mut children = Vec::new();
    for member in ast.children(package, Prop::Members) {
        match ast[member].ty {
            Type::PackageDecl if holds_parts(ast, member) => children.push(package_node(ast, member)),
            Type::PartDef => children.push(module_node(ast, member)),
            _ => {}
        }
    }
    let name = ast.text(package, Prop::Name).unwrap_or("");
    GraphNode {
        element: Some(package),
        id: qualified_name(ast, package),
        shape: NodeShape::Package,
        keyword: String::new(),
        name: name.to_string(),
        compartments: Vec::new(),
        ports: Vec::new(),
        children: if children.is_empty() { None } else { Some(children) },
        width: 130f64.max(measure_text(name, 12.0, Weight::Bold) + 50.0),
        height: 60.0,
    }
}

/// An import's keyword (§8.2.3.5): «import» for one element, «import»* for a
/// namespace's members, «import»** for its members' members too.
fn import_label(ast: &Ast, import: NodeId) -> &'static str {
    if ast.flag(import, Prop::Recursive) {
        "«import»**"
    } else if ast.flag(import, Prop::Wildcard) {
        "«import»*"
    } else {
        "«import»"
    }
}

/// The package an import points at: `Pkg::Element` imports from `Pkg`.
fn imported_package(ast: &Ast, import: NodeId) -> Option<NodeId> {
    let mut at = ast.target(import, Prop::Target);
    while let Some(node) = at {
        if ast[node].ty == Type::PackageDecl {
            return Some(node);
        }
        at = ast[node].parent;
    }
    None
}

/// The Implementation view: the exposed packages that hold software, each a box with
/// its sub-packages and modules inside, and an «import» arrow wherever one of them
/// imports from another on the view.
pub fn extract_implementation_graph(ast: &Ast, exposed: &[NodeId]) -> DiagramGraph {
    let drawn: Vec<NodeId> = exposed
        .iter()
        .copied()
        .filter(|element| ast[*element].ty == Type::PackageDecl && holds_parts(ast, *element))
        .collect();
    let nodes = drawn.iter().map(|package| package_node(ast, *package)).collect();

    // Imports are drawn between the packages on this view; one reaching outside it,
    // such as a module's import of the function tree, or a package of requirements,
    // is not.
    let all_packages: Vec<NodeId> =
        collect_packages(ast, &drawn).into_iter().filter(|package| holds_parts(ast, *package)).collect();
    let package_ids: HashSet<String> = all_packages.iter().map(|package| qualified_name(ast, *package)).collect();

    let mut edges: Vec<GraphEdge> = Vec::new();
    let mut seen = HashSet::new();
    for package in &all_packages {
        let source_id = qualified_name(ast, *package);
        for import in ast.children(*package, Prop::Imports) {
            let target_id = imported_package(ast, import)
                .map(|target| qualified_name(ast, target))
                .filter(|target_id| package_ids.contains(target_id));
            let Some(target_id) = target_id else {
                continue;
            };
            if target_id != source_id && seen.insert(format!("{source_id}->{target_id}")) {
                let mut edge =
                    GraphEdge::new(format!("e{}", edges.len()), EdgeKind::Import, source_id.clone(), target_id);
                edge.label = Some(import_label(ast, import).to_string());
                edges.push(edge);
            }
        }
    }
    DiagramGraph { nodes, edges }
}
