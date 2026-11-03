//! The use case view: use cases, their subject and actors, and what they include.
use super::connector_ends::{box_size, declaration, type_name, usage_label};
use crate::model::graph::{
    DiagramGraph, EdgeKind, GraphEdge, GraphNode, NodeShape, box_of, effective_name, qualified_name,
};
use crate::model::packages::exposed_members;
use crate::parser::ast::{Ast, NodeId, Prop, Type};
use crate::render::text_metrics::{Weight, measure_text};
use std::collections::{HashMap, HashSet};

/// Own members plus, for a typed usage, the members inherited from its def. A use
/// case is a `use case def`, a `use case`, or an `include`: `include use case x`
/// declares one (§7.25), `include x` only names one.
fn effective_members(ast: &Ast, use_case: NodeId) -> Vec<NodeId> {
    let mut members: Vec<NodeId> = ast.children(use_case, Prop::Members).collect();
    if ast[use_case].ty != Type::UseCaseDef
        && let Some(definition) = ast.target(use_case, Prop::Type)
    {
        members.extend(ast.children(definition, Prop::Members));
    }
    members
}

fn plain_node(id: String, shape: NodeShape, keyword: &str, name: String, width: f64, height: f64) -> GraphNode {
    GraphNode {
        id,
        shape,
        keyword: keyword.to_string(),
        name,
        compartments: Vec::new(),
        ports: Vec::new(),
        children: None,
        width,
        height,
    }
}

/// A «use case» box, named as declared, or a «use case def» one.
pub fn use_case_node(ast: &Ast, use_case: NodeId) -> GraphNode {
    let name = if ast[use_case].ty == Type::UseCaseDef {
        ast.text(use_case, Prop::Name).unwrap_or("").to_string()
    } else {
        usage_label(ast, use_case)
    };
    let (shape, keyword) = box_of(ast, use_case);
    let (width, height) = box_size(&name, &[], 110.0, 40.0, keyword);
    plain_node(qualified_name(ast, use_case), shape, keyword, name, width, height)
}

/// The «subject» box, drawn once however many use cases share it.
fn subject_node(ast: &Ast, subject: NodeId, label: &str) -> GraphNode {
    let (shape, keyword) = box_of(ast, subject);
    let (width, height) = box_size(label, &[], 110.0, 40.0, keyword);
    plain_node(format!("subject:{label}"), shape, keyword, label.to_string(), width, height)
}

fn actor_node(id: String, label: String) -> GraphNode {
    let width = 48f64.max(measure_text(&label, 11.0, Weight::Regular) + 10.0);
    plain_node(id, NodeShape::Actor, "", label, width, 76.0)
}

fn is_use_case(ast: &Ast, node: NodeId) -> bool {
    matches!(ast[node].ty, Type::UseCaseDef | Type::UseCaseUsage)
}

/// The use case view (§8.2.3.25): «use case» boxes, the «subject» and the actors
/// each joined to their use cases by a plain line (§8.2.3.21), and an «include»
/// arrow to each included use case.
pub fn extract_use_case_graph(ast: &Ast, exposed: &[NodeId]) -> DiagramGraph {
    let mut use_cases: Vec<NodeId> = Vec::new();
    for member in exposed_members(ast, exposed) {
        if is_use_case(ast, member) && !use_cases.contains(&member) {
            use_cases.push(member);
        }
    }
    // What those declare in turn is reached too.
    let mut i = 0;
    while i < use_cases.len() {
        for member in effective_members(ast, use_cases[i]) {
            if ast[member].ty == Type::IncludeUsage && ast.flag(member, Prop::UseCase) && !use_cases.contains(&member) {
                use_cases.push(member);
            }
        }
        i += 1;
    }

    // A def represented by usages is not shown itself.
    let typed_defs: HashSet<NodeId> = use_cases
        .iter()
        .filter(|u| ast[**u].ty != Type::UseCaseDef)
        .filter_map(|u| ast.target(*u, Prop::Type))
        .collect();
    let rendered: Vec<NodeId> =
        use_cases.iter().copied().filter(|u| !(ast[*u].ty == Type::UseCaseDef && typed_defs.contains(u))).collect();
    let mut nodes: Vec<GraphNode> = rendered.iter().map(|u| use_case_node(ast, *u)).collect();
    // Each rendered use case's box is the node at the same place.
    let box_of_use_case: HashMap<NodeId, usize> = rendered.iter().enumerate().map(|(i, u)| (*u, i)).collect();
    let mut subjects: HashMap<String, usize> = HashMap::new();
    let mut actor_nodes: HashMap<String, usize> = HashMap::new();
    let mut edges: Vec<GraphEdge> = Vec::new();
    let mut joined: HashSet<(String, String)> = HashSet::new();

    // A line has no direction and is drawn without an arrowhead; the edge's
    // direction only tells the layout a secondary actor from a primary one.
    let mut join = |edges: &mut Vec<GraphEdge>, source_id: &str, target_id: &str| {
        if joined.insert((source_id.to_string(), target_id.to_string())) {
            let id = format!("e{}", edges.len());
            edges.push(GraphEdge::new(id, EdgeKind::Association, source_id.to_string(), target_id.to_string()));
        }
    };

    // An actor declared on a use case def, and so shared by every use case of that
    // kind, is a primary actor. One added on a single use case only, someone the
    // system acts on, not a user of it, is a secondary actor, drawn after the
    // primary ones. The distinction needs at least one actor to come through a def;
    // otherwise every actor is primary.
    let actor_key = |actor: NodeId| -> String {
        match ast.target(actor, Prop::Type) {
            Some(definition) => qualified_name(ast, definition),
            None => ast.text(actor, Prop::Name).unwrap_or("").to_string(),
        }
    };
    let is_actor = |member: &NodeId| ast[*member].ty == Type::ActorUsage;
    let mut on_def: HashSet<String> = HashSet::new();
    for use_case in use_cases.iter().filter(|u| ast[**u].ty == Type::UseCaseDef) {
        on_def.extend(ast.children(*use_case, Prop::Members).filter(is_actor).map(actor_key));
    }
    let is_secondary = |key: &str| !on_def.is_empty() && !on_def.contains(key);

    for (i, use_case) in rendered.iter().enumerate() {
        let members = effective_members(ast, *use_case);
        let node_id = nodes[i].id.clone();
        // A use case restating its def's subject, `subject machine;`, redefines it
        // and keeps its type: both are `machine : CoffeeMachine`, one box.
        let declared: Vec<NodeId> = members.iter().copied().filter(|m| ast[*m].ty == Type::SubjectUsage).collect();
        if let Some(first) = declared.first() {
            let name = declared.iter().filter_map(|s| effective_name(ast, *s)).find(|name| !name.is_empty());
            let ty = declared.iter().filter_map(|s| type_name(ast, *s)).find(|ty| !ty.is_empty());
            let label = declaration(name.unwrap_or(""), ty);
            let at = match subjects.get(&label) {
                Some(at) => *at,
                None => {
                    nodes.push(subject_node(ast, *first, &label));
                    subjects.insert(label, nodes.len() - 1);
                    nodes.len() - 1
                }
            };
            let subject_id = nodes[at].id.clone();
            join(&mut edges, &node_id, &subject_id);
        }

        for actor in members.iter().copied().filter(is_actor) {
            let key = actor_key(actor);
            let at = match actor_nodes.get(&key) {
                Some(at) => *at,
                None => {
                    nodes.push(actor_node(format!("actor:{key}"), usage_label(ast, actor)));
                    actor_nodes.insert(key.clone(), nodes.len() - 1);
                    nodes.len() - 1
                }
            };
            let actor_id = nodes[at].id.clone();
            if is_secondary(&key) {
                join(&mut edges, &node_id, &actor_id);
            } else {
                join(&mut edges, &actor_id, &node_id);
            }
        }
    }

    // Include edges. The target is always a usage: SysML v2 includes a use case,
    // not a kind of use case.
    for (i, use_case) in rendered.iter().enumerate() {
        for include in effective_members(ast, *use_case).into_iter().filter(|m| ast[*m].ty == Type::IncludeUsage) {
            // Its own box, or that of the use case it names.
            let included = if ast.flag(include, Prop::UseCase) {
                Some(include)
            } else {
                let end = ast.child(include, Prop::Target);
                end.and_then(|end| ast.references_of(end, Prop::Segments).pop()).and_then(|r| ast.reference_target(r))
            };
            let Some(&target) = included.and_then(|included| box_of_use_case.get(&included)) else {
                continue;
            };
            if target != i {
                let id = format!("e{}", edges.len());
                let mut edge = GraphEdge::new(id, EdgeKind::Include, nodes[i].id.clone(), nodes[target].id.clone());
                edge.label = Some("«include»".to_string());
                edges.push(edge);
            }
        }
    }

    DiagramGraph { nodes, edges }
}
