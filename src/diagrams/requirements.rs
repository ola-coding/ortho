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
    /// Its short name, `<'REQ-1.2'>`, which is its `reqId`.
    pub id: Option<String>,
    pub name: Option<String>,
    /// How many requirements of the table it is nested in.
    pub depth: usize,
    /// Its `text`: the bodies of its own `doc` comments, a paragraph each.
    pub text: Vec<String>,
    /// The text of each `Rationale` metadata it holds.
    pub rationale: Vec<String>,
    /// What satisfies it, as each `satisfy` that names it writes it, and the element that is.
    pub satisfied_by: Vec<(NodeId, String)>,
    /// The requirement it is nested in that is satisfied, when nothing satisfies it itself:
    /// satisfying a requirement satisfies the requirements nested in it.
    pub covered_by: Option<NodeId>,
    /// Its `StatusInfo`'s status as written, `tbr`, when it has one.
    pub status: Option<String>,
    /// The verification cases whose objective verifies it, each with its name and
    /// the `VerificationMethod` kinds it holds, `test`. Verifying a requirement does not
    /// verify the requirements nested in it: each has its own figure to show.
    pub verified_by: Vec<Verification>,
    /// The requirements it is derived from: the original of each derivation that
    /// derives it, as the standard library's DerivationConnections has it, each
    /// with its ID, or else its name, for a table that does not list it.
    pub derived_from: Vec<(NodeId, String)>,
    /// The package it is declared in, which is its section once the table has more than one.
    pub section: Section,
}

/// A package a table's requirements are declared in, with its name and its own `doc`.
#[derive(Clone, PartialEq)]
pub struct Section {
    pub package: Option<NodeId>,
    pub name: String,
    pub introduction: Vec<String>,
}

/// A verification case that verifies a requirement.
#[derive(Clone)]
pub struct Verification {
    pub case: NodeId,
    pub name: String,
    pub methods: Vec<String>,
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
    let verified = verifications(ast, roots);
    let derived = derivations(ast, roots);
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
                status: status(ast, element),
                verified_by: verified.get(&element).cloned().unwrap_or_default(),
                derived_from: derived
                    .get(&element)
                    .into_iter()
                    .flatten()
                    .map(|original| {
                        let label = ast.text(*original, Prop::ShortName).or(effective_name(ast, *original));
                        (*original, label.unwrap_or("").to_string())
                    })
                    .collect(),
                section: section(ast, element),
            }
        })
        .collect()
}

/// What each requirement of the model is satisfied by, for every `satisfy` that
/// names it: the feature its `by` names, or else the element the `satisfy` is in,
/// with its name as written there.
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
            Some(end) => (chain_target(ast, end), written_chain(ast, end)),
            None => {
                let owner = ast[satisfy].parent;
                (owner, owner.and_then(|owner| effective_name(ast, owner)).unwrap_or("").to_string())
            }
        };
        satisfied.entry(requirement).or_default().push((by.0.unwrap_or(satisfy), by.1));
    }
    satisfied
}

/// Whether `connection` is a derivation, as the standard library's
/// RequirementDerivation and DerivationConnections define it: a connection or a
/// connection def marked `#derivation`, or typed by `Derivation`.
pub fn is_derivation(ast: &Ast, connection: NodeId) -> bool {
    let connects = ast[connection].ty == Type::ConnectionUsage
        || (ast[connection].ty == Type::OtherDefinition && ast.text(connection, Prop::Kind) == Some("connection"));
    connects
        && (ast.texts(connection, Prop::PrefixMetadata).any(|tag| last_name(tag) == "derivation")
            || ast.text(connection, Prop::Type).is_some_and(|ty| last_name(ty) == "Derivation"))
}

/// The package a requirement is declared in, its nested requirements' too.
fn section(ast: &Ast, requirement: NodeId) -> Section {
    let package = std::iter::successors(ast[requirement].parent, |node| ast[*node].parent)
        .find(|node| ast[*node].ty == Type::PackageDecl);
    Section {
        package,
        name: package.and_then(|package| effective_name(ast, package)).unwrap_or("").to_string(),
        introduction: package.map(|package| documentation(ast, package)).unwrap_or_default(),
    }
}

/// Whether the model has a derivation.
pub fn has_derivation(ast: &Ast, roots: &[NodeId]) -> bool {
    roots.iter().flat_map(|root| ast.with_all_contents(*root)).any(|node| is_derivation(ast, node))
}

/// The originals each requirement of the model is derived from, each once, in the
/// order the derivations are declared. A derivation's end tagged `#original` names
/// the original, those tagged `#derive` what is derived from it. An end of a
/// derivation typed by a derivation def takes its tag from the def's end it
/// redefines, by name, as in OMG's RequirementDerivationExample.
pub fn derivations(ast: &Ast, roots: &[NodeId]) -> HashMap<NodeId, Vec<NodeId>> {
    let mut derived: HashMap<NodeId, Vec<NodeId>> = HashMap::new();
    for connection in roots.iter().flat_map(|root| ast.with_all_contents(*root)) {
        if !is_derivation(ast, connection) || ast[connection].ty != Type::ConnectionUsage {
            continue;
        }
        let definition = ast.target(connection, Prop::Type);
        let role = |end: NodeId, role: &str| {
            derivation_role(ast, end, role)
                || definition.is_some_and(|definition| {
                    let name = effective_name(ast, end);
                    ast.children(definition, Prop::Members).any(|own| {
                        ast.flag(own, Prop::IsEnd)
                            && name.is_some()
                            && effective_name(ast, own) == name
                            && derivation_role(ast, own, role)
                    })
                })
        };
        let named = |end: NodeId| ast.child(end, Prop::Referenced).and_then(|reference| chain_target(ast, reference));
        let ends: Vec<NodeId> =
            ast.children(connection, Prop::Members).filter(|member| ast.flag(*member, Prop::IsEnd)).collect();
        let Some(original) = ends.iter().find(|end| role(**end, "original")).and_then(|end| named(*end)) else {
            continue;
        };
        for end in ends.iter().filter(|end| role(**end, "derive")) {
            if let Some(requirement) = named(*end) {
                let originals = derived.entry(requirement).or_default();
                if !originals.contains(&original) {
                    originals.push(original);
                }
            }
        }
    }
    derived
}

/// Whether `end` is an end of a derivation.
pub fn is_derivation_end(ast: &Ast, end: NodeId) -> bool {
    ast.flag(end, Prop::IsEnd) && ast[end].parent.is_some_and(|connection| is_derivation(ast, connection))
}

/// Whether a derivation's end is tagged `#original` or `#derive`, which the
/// library's RequirementDerivation defines.
pub fn derivation_role(ast: &Ast, end: NodeId, role: &str) -> bool {
    ast.texts(end, Prop::PrefixMetadata).any(|tag| last_name(tag) == role)
}

/// A name without what qualifies it, and without a `#`: `#RequirementDerivation::derive` is `derive`.
fn last_name(name: &str) -> &str {
    let name = name.strip_prefix('#').unwrap_or(name);
    name.rsplit("::").next().unwrap_or(name)
}

/// Whether `node` is a `verify` of a requirement: `verify r;`, or `verify requirement r : R;`.
pub fn is_verify(ast: &Ast, node: NodeId) -> bool {
    ast[node].ty == Type::OtherUsage && ast.text(node, Prop::Kind) == Some("verify")
}

/// Whether `node` is a verification case: `verification def`, or `verification`.
pub fn is_verification_case(ast: &Ast, node: NodeId) -> bool {
    matches!(ast[node].ty, Type::OtherDefinition | Type::OtherUsage)
        && ast.text(node, Prop::Kind) == Some("verification")
}

/// Whether the model has a verification case.
pub fn has_verification_case(ast: &Ast, roots: &[NodeId]) -> bool {
    roots.iter().flat_map(|root| ast.with_all_contents(*root)).any(|node| is_verification_case(ast, node))
}

/// The verification cases that verify each requirement of the model, each once:
/// a case's objective names a requirement with `verify`, as the standard library's
/// VerificationCases defines it.
pub fn verifications(ast: &Ast, roots: &[NodeId]) -> HashMap<NodeId, Vec<Verification>> {
    let mut verified: HashMap<NodeId, Vec<Verification>> = HashMap::new();
    for verify in roots.iter().flat_map(|root| ast.with_all_contents(*root)) {
        if !is_verify(ast, verify) {
            continue;
        }
        let Some(requirement) = ast.child(verify, Prop::Requirement).and_then(|end| chain_target(ast, end)) else {
            continue;
        };
        let Some(case) = std::iter::successors(ast[verify].parent, |node| ast[*node].parent)
            .find(|node| is_verification_case(ast, *node))
        else {
            continue;
        };
        let cases = verified.entry(requirement).or_default();
        if cases.iter().all(|known| known.case != case) {
            let name = effective_name(ast, case).unwrap_or("").to_string();
            cases.push(Verification { case, name, methods: verification_methods(ast, case) });
        }
    }
    verified
}

/// The kinds of each `VerificationMethod` a case holds, as written and without the
/// enumeration's name: `@VerificationMethod { kind = (VerificationMethodKind::test, analyze); }`
/// is `test` and `analyze`.
fn verification_methods(ast: &Ast, case: NodeId) -> Vec<String> {
    let mut kinds = Vec::new();
    for value in metadata_values(ast, case, "VerificationMethod", "kind") {
        let items: Vec<NodeId> = if ast[value].ty == Type::SequenceExpression {
            ast.children(value, Prop::Items).collect()
        } else {
            vec![value]
        };
        kinds.extend(items.into_iter().filter_map(|item| enumeration_value(ast, item)));
    }
    kinds
}

/// A requirement's status, `@StatusInfo { status = StatusKind::tbr; }`, as written
/// without the enumeration's name: `tbr`.
fn status(ast: &Ast, element: NodeId) -> Option<String> {
    metadata_values(ast, element, "StatusInfo", "status").into_iter().find_map(|value| enumeration_value(ast, value))
}

/// The value an expression names, without what qualifies it: `StatusKind::tbr` is `tbr`.
fn enumeration_value(ast: &Ast, value: NodeId) -> Option<String> {
    if ast[value].ty != Type::FeatureReferenceExpression {
        return None;
    }
    ast.text(value, Prop::Name).and_then(|name| name.rsplit("::").next()).map(String::from)
}

/// The values given to `feature` in each metadata of the definition `definition`
/// that an element holds, `@Rationale { text = "..."; }`. A definition is matched by
/// its last name, since the library is not loaded.
fn metadata_values(ast: &Ast, element: NodeId, definition: &str, feature: &str) -> Vec<NodeId> {
    let held = ast.children(element, Prop::Members).chain(ast.children(element, Prop::Annotations));
    let metadata = held.filter(|node| {
        ast[*node].ty == Type::MetadataUsage
            && ast.text(*node, Prop::Type).is_some_and(|ty| ty.rsplit("::").next() == Some(definition))
    });
    let values = metadata.filter_map(|metadata| {
        let given = ast.children(metadata, Prop::Members).find(|member| {
            ast.text(*member, Prop::Name) == Some(feature) || ast.texts(*member, Prop::Redefines).any(|r| r == feature)
        })?;
        ast.child(given, Prop::Value)
    });
    values.collect()
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
    let values = metadata_values(ast, element, "Rationale", "text").into_iter();
    let literals = values.filter(|value| ast[*value].ty == Type::LiteralExpression);
    literals.filter_map(|value| ast.text(value, Prop::Value).map(String::from)).collect()
}
