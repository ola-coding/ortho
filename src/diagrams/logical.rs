//! The logical view: function trees, and the use cases they refine.
use super::connector_ends::box_size;
use super::use_case::use_case_node;
use crate::model::graph::{DiagramGraph, EdgeKind, GraphEdge, GraphNode, box_of, qualified_name};
use crate::model::packages::exposed_members;
use crate::parser::ast::{Ast, NodeId, Prop, Type};
use std::collections::HashSet;

/// «action def» for the root of a tree, «action» for the functions in it.
fn function_node(ast: &Ast, function: NodeId) -> GraphNode {
    let (shape, keyword) = box_of(ast, function);
    let name = ast.text(function, Prop::Name).unwrap_or("");
    let (width, height) = box_size(name, &[], 110.0, 40.0, keyword);
    GraphNode {
        element: Some(function),
        id: qualified_name(ast, function),
        shape,
        keyword: keyword.to_string(),
        name: name.to_string(),
        compartments: Vec::new(),
        ports: Vec::new(),
        children: None,
        width,
        height,
    }
}

struct Extraction<'a> {
    ast: &'a Ast,
    nodes: Vec<GraphNode>,
    edges: Vec<GraphEdge>,
    seen: HashSet<String>,
}

impl Extraction<'_> {
    fn edge(&mut self, kind: EdgeKind, source_id: String, target_id: String) {
        let id = format!("e{}", self.edges.len());
        self.edges.push(GraphEdge::new(id, kind, source_id, target_id));
    }

    /// A use case the view exposes, drawn once.
    fn use_case(&mut self, use_case: NodeId) {
        if self.seen.insert(qualified_name(self.ast, use_case)) {
            self.nodes.push(use_case_node(self.ast, use_case));
        }
    }

    /// A function and, nested in it in declaration order, its children.
    fn visit(&mut self, function: NodeId) {
        let id = qualified_name(self.ast, function);
        if !self.seen.insert(id.clone()) {
            return;
        }
        self.nodes.push(function_node(self.ast, function));
        let ast = self.ast;
        for child in ast.children(function, Prop::Members).filter(|member| ast[*member].ty == Type::ActionUsage) {
            let child_id = qualified_name(ast, child);
            self.visit(child);
            self.edge(EdgeKind::Decomposition, id.clone(), child_id);
        }
    }
}

/// The Logical view is a capability tree: every function is one box with exactly one
/// parent, and a branch is a plain line rather than an arrow. Decomposition comes
/// from nesting in the source, an `action` declared inside another is its child, so a
/// function appears exactly once no matter how many things use it.
///
/// A typed action usage in a package is one use of a tree, not a tree of its own:
/// `action survey : PerformAerialSurvey` is what a realizing part's `perform` chain
/// starts from, and the tree it names is already drawn from its definition. Drawing
/// it too would double the diagram.
///
/// The functions detail what the use cases ask for. A use case the view exposes is
/// drawn too, and a dependency joining two things on the view, such as a tree's
/// `#refinement dependency` on its use case, is a dashed arrow (§8.2.3.3).
/// `packages` are the model's, where the dependencies are found.
pub fn extract_logical_graph(ast: &Ast, exposed: &[NodeId], packages: &[NodeId]) -> DiagramGraph {
    let mut extraction = Extraction { ast, nodes: Vec::new(), edges: Vec::new(), seen: HashSet::new() };
    for member in exposed_members(ast, exposed) {
        match ast[member].ty {
            Type::ActionDef => extraction.visit(member),
            Type::ActionUsage if ast.text(member, Prop::Type).is_none() => extraction.visit(member),
            Type::UseCaseDef | Type::UseCaseUsage => extraction.use_case(member),
            _ => {}
        }
    }
    for package in packages {
        for dependency in ast.with_all_contents(*package) {
            if ast[dependency].ty != Type::Dependency {
                continue;
            }
            let ends = |prop: Prop| -> Vec<String> {
                let targets =
                    ast.references_of(dependency, prop).into_iter().filter_map(|end| ast.reference_target(end));
                targets.map(|end| qualified_name(ast, end)).filter(|id| extraction.seen.contains(id)).collect()
            };
            let (clients, suppliers) = (ends(Prop::Clients), ends(Prop::Suppliers));
            for source_id in &clients {
                for target_id in &suppliers {
                    extraction.edge(EdgeKind::Dependency, source_id.clone(), target_id.clone());
                }
            }
        }
    }
    DiagramGraph { nodes: extraction.nodes, edges: extraction.edges }
}
