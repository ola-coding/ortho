//! The table of requirements: each requirement a GridView exposes, with its ID, its
//! text and rationale, and what satisfies it.
use crate::model::graph::effective_name;
use crate::model::packages::exposed_members;
use crate::model::views::is_requirement;
use crate::parser::ast::{Ast, NodeId, Prop, Type};
use std::collections::{HashMap, HashSet};

/// A requirement as its table lists it.
pub struct Row {
    pub element: NodeId,
    /// Its short name, `<'REQ-001'>`, which is its `reqId`.
    pub id: Option<String>,
    pub name: Option<String>,
    /// How many requirements of the table it is nested in.
    pub depth: usize,
    /// Its `text`: the bodies of its own `doc` comments, a paragraph each.
    pub text: Vec<String>,
    /// The text of each `Rationale` metadata it holds.
    pub rationale: Vec<String>,
    /// Each `satisfy` that names it, with what satisfies it, as written.
    pub satisfied_by: Vec<(NodeId, String)>,
    /// The requirement it is nested in that is satisfied, when nothing satisfies it itself:
    /// satisfying a requirement satisfies the requirements nested in it.
    pub covered_by: Option<NodeId>,
}

impl Row {
    pub fn is_satisfied(&self) -> bool {
        !self.satisfied_by.is_empty() || self.covered_by.is_some()
    }
}

/// The requirements a GridView exposing `exposed` lists, in the order they are
/// declared, each followed by those nested in it. An exposed package stands for the
/// requirements in it.
pub fn tabled_requirements(ast: &Ast, exposed: &[NodeId]) -> Vec<(NodeId, usize)> {
    fn visit(
        ast: &Ast,
        requirement: NodeId,
        depth: usize,
        seen: &mut HashSet<NodeId>,
        rows: &mut Vec<(NodeId, usize)>,
    ) {
        if !seen.insert(requirement) {
            return;
        }
        rows.push((requirement, depth));
        for nested in ast.children(requirement, Prop::Members).filter(|member| is_requirement(ast, *member)) {
            visit(ast, nested, depth + 1, seen, rows);
        }
    }
    let mut rows = Vec::new();
    let mut seen = HashSet::new();
    for member in exposed_members(ast, exposed).into_iter().filter(|member| is_requirement(ast, *member)) {
        visit(ast, member, 0, &mut seen, &mut rows);
    }
    rows
}

/// The rows of a GridView exposing `exposed`, in a model whose files' roots are `roots`.
pub fn extract_requirements(ast: &Ast, roots: &[NodeId], exposed: &[NodeId]) -> Vec<Row> {
    let satisfied = satisfactions(ast, roots);
    tabled_requirements(ast, exposed)
        .into_iter()
        .map(|(element, depth)| {
            let satisfied_by = satisfied.get(&element).cloned().unwrap_or_default();
            let covered_by = if satisfied_by.is_empty() { covering(ast, element, &satisfied) } else { None };
            Row {
                element,
                id: ast.text(element, Prop::ShortName).map(String::from),
                name: effective_name(ast, element).map(String::from),
                depth,
                text: documentation(ast, element),
                rationale: rationale(ast, element),
                satisfied_by,
                covered_by,
            }
        })
        .collect()
}

/// What each requirement of the model is satisfied by: every `satisfy` that names it,
/// with the satisfying feature as its `by` names it, or else the element the
/// `satisfy` is in.
pub fn satisfactions(ast: &Ast, roots: &[NodeId]) -> HashMap<NodeId, Vec<(NodeId, String)>> {
    let mut satisfied: HashMap<NodeId, Vec<(NodeId, String)>> = HashMap::new();
    for satisfy in roots.iter().flat_map(|root| ast.with_all_contents(*root)) {
        if ast[satisfy].ty != Type::SatisfyUsage || ast.flag(satisfy, Prop::Negated) {
            continue;
        }
        let Some(requirement) = ast.child(satisfy, Prop::Requirement).and_then(|end| chain_target(ast, end)) else {
            continue;
        };
        let by = match ast.child(satisfy, Prop::Satisfier) {
            Some(end) => written_chain(ast, end),
            None => ast[satisfy].parent.and_then(|owner| effective_name(ast, owner)).unwrap_or("").to_string(),
        };
        satisfied.entry(requirement).or_default().push((satisfy, by));
    }
    satisfied
}

/// The requirement nearest above `requirement`, it included, that `satisfied` has.
pub fn covering<T>(ast: &Ast, requirement: NodeId, satisfied: &HashMap<NodeId, T>) -> Option<NodeId> {
    std::iter::successors(Some(requirement), |node| ast[*node].parent)
        .take_while(|node| is_requirement(ast, *node))
        .find(|node| satisfied.contains_key(node))
}

/// What a chain, `machine.heater`, names: what its last segment resolved to.
pub fn chain_target(ast: &Ast, end: NodeId) -> Option<NodeId> {
    ast.references_of(end, Prop::Segments).last().and_then(|segment| ast.reference_target(*segment))
}

/// A chain as written: `Physical::machine.heater`.
pub fn written_chain(ast: &Ast, end: NodeId) -> String {
    ast.texts(end, Prop::Segments).collect::<Vec<_>>().join(".")
}

/// The bodies of an element's own `doc` comments, each as its paragraphs.
pub fn documentation(ast: &Ast, element: NodeId) -> Vec<String> {
    ast.children(element, Prop::Annotations)
        .filter(|annotation| ast[*annotation].ty == Type::Documentation)
        .filter_map(|doc| ast.text(doc, Prop::Body))
        .flat_map(comment_paragraphs)
        .collect()
}

/// A comment's body as text: without its `/*` and `*/`, each line without the
/// spaces and the `*` it starts with, and its lines joined into paragraphs at each
/// empty line.
pub fn comment_paragraphs(body: &str) -> Vec<String> {
    let inner = body.strip_prefix("/*").unwrap_or(body);
    let inner = inner.strip_suffix("*/").unwrap_or(inner);
    let mut paragraphs: Vec<String> = Vec::new();
    let mut current: Vec<&str> = Vec::new();
    for line in inner.lines() {
        let line = line.trim();
        let line = line.strip_prefix('*').map_or(line, str::trim_start);
        if line.is_empty() {
            if !current.is_empty() {
                paragraphs.push(current.join(" "));
                current.clear();
            }
        } else {
            current.push(line);
        }
    }
    if !current.is_empty() {
        paragraphs.push(current.join(" "));
    }
    paragraphs
}

/// The text of each `Rationale` an element holds, `@Rationale { text = "..."; }`,
/// as the standard library's ModelingMetadata defines it. Matched by its last name,
/// since the library is not loaded.
fn rationale(ast: &Ast, element: NodeId) -> Vec<String> {
    let held = ast.children(element, Prop::Members).chain(ast.children(element, Prop::Annotations));
    let rationales = held.filter(|node| {
        ast[*node].ty == Type::MetadataUsage
            && ast.text(*node, Prop::Type).is_some_and(|ty| ty.rsplit("::").next() == Some("Rationale"))
    });
    let texts = rationales.filter_map(|metadata| {
        let text = ast.children(metadata, Prop::Members).find(|feature| {
            ast.text(*feature, Prop::Name) == Some("text") || ast.texts(*feature, Prop::Redefines).any(|r| r == "text")
        })?;
        let value = ast.child(text, Prop::Value).filter(|value| ast[*value].ty == Type::LiteralExpression)?;
        ast.text(value, Prop::Value).map(String::from)
    });
    texts.collect()
}
