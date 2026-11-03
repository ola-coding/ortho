//! What a model reports once linked. Most of it is names that resolve to nothing,
//! and ortho does not load the standard library, so some references name nothing it
//! has and are not errors: see `may_name_nothing`.
//!
//! The rest keeps a model ortho accepts a valid SysML v2 model, where the grammar
//! alone cannot. A reserved word as a name needs no check here: the grammar's
//! keywords include them all, so it does not parse.
use super::ast::{Ast, NodeId, Prop, Type};
use super::scope::Logged;
use super::workspace::{Diagnostic, Severity};
use crate::model::graph::effective_name;
use crate::model::library::{LIBRARY, LibraryPackage};
use std::collections::HashSet;

/// What one file reports: its unresolved references, then each node's checks.
pub fn validate(ast: &Ast, root: NodeId, _text: &str, log: &[Logged]) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    for logged in log {
        if let Some(message) = &logged.error
            && !may_name_nothing(ast, logged)
        {
            let offset = ast[logged.reference.node].fields[logged.reference.field as usize].offset as usize;
            diagnostics.push(Diagnostic { severity: Severity::Error, offset, message: message.clone() });
        }
    }
    for node in ast.with_all_contents(root) {
        match ast[node].ty {
            Type::ConnectorEnd => check_end_starts_at_accessible_feature(ast, node, &mut diagnostics),
            Type::PackageDecl => check_package_not_named_after_library(ast, node, &mut diagnostics),
            Type::PerformUsage => check_perform_target(ast, node, &mut diagnostics),
            Type::UseCaseDef | Type::UseCaseUsage => check_subject_before_actors(ast, node, &mut diagnostics),
            _ => {}
        }
    }
    diagnostics
}

/// Where a property's first value starts, or else the node.
fn offset_of(ast: &Ast, node: NodeId, prop: Prop) -> usize {
    let field = ast[node].fields.iter().find(|field| field.prop == prop);
    field.map_or(ast[node].offset, |field| field.offset) as usize
}

/// A top-level package named after a standard library package hides it, and which
/// of the two a reference finds depends on the tool. ortho finds the model's; OMG's
/// pilot implementation finds the library's when the reference is in the same file.
/// So the model renders here and may fail elsewhere, which is worth a warning but no
/// reason to stop.
fn check_package_not_named_after_library(ast: &Ast, package: NodeId, diagnostics: &mut Vec<Diagnostic>) {
    let top_level = ast[package].parent.is_some_and(|parent| ast[parent].ty == Type::Model);
    let Some(name) = ast.text(package, Prop::Name) else {
        return;
    };
    if top_level && !ast.flag(package, Prop::IsLibrary) && !name.is_empty() && library_package(name).is_some() {
        diagnostics.push(Diagnostic {
            severity: Severity::Warning,
            offset: offset_of(ast, package, Prop::Name),
            message: format!(
                "Package '{name}' has the name of a standard library package and hides it, and other tools may find the library's instead. Rename it."
            ),
        });
    }
}

/// A qualified name may walk through packages only. `Hardware::CoffeeMachine::controller`
/// names a feature through the definition that owns it, which SysML v2 rejects: the
/// chain has to start at a usage and continue with dots, as in `Hardware::machine.controller`.
fn check_end_starts_at_accessible_feature(ast: &Ast, end: NodeId, diagnostics: &mut Vec<Diagnostic>) {
    let Some(first) = ast.references_of(end, Prop::Segments).first().copied() else {
        return;
    };
    let text = ast.reference_text(first);
    if let Some(target) = ast.reference_target(first)
        && text.contains("::")
        && ast[target].parent.is_none_or(|owner| ast[owner].ty != Type::PackageDecl)
    {
        diagnostics.push(Diagnostic {
            severity: Severity::Error,
            offset: offset_of(ast, end, Prop::Segments),
            message: format!("'{text}' names a feature through its owner. Start at a usage and continue with dots."),
        });
    }
}

/// The subject is a use case's first parameter and its actors follow, so a use case
/// that declares an actor has to declare its subject before it, even when its
/// definition already has one. A secondary actor, added on a single use case,
/// therefore comes with that use case's own `subject` line.
fn check_subject_before_actors(ast: &Ast, use_case: NodeId, diagnostics: &mut Vec<Diagnostic>) {
    let first =
        ast.children(use_case, Prop::Members).find(|m| matches!(ast[*m].ty, Type::SubjectUsage | Type::ActorUsage));
    if let Some(first) = first
        && ast[first].ty == Type::ActorUsage
    {
        let named = match effective_name(ast, use_case) {
            Some(name) => format!("the subject of '{name}'"),
            None => "the use case's subject".to_string(),
        };
        diagnostics.push(Diagnostic {
            severity: Severity::Error,
            offset: ast[first].offset as usize,
            message: format!("Declare {named} before its actors: the subject is a use case's first parameter."),
        });
    }
}

/// `perform` names something its owner can carry out: an action of the function
/// tree, a use case, or other behaviour such as a calculation or a verification
/// case. A part, a port or an attribute is structure, not behaviour. The chain is
/// otherwise the same as any connector end, so it is checked there.
fn check_perform_target(ast: &Ast, perform: NodeId, diagnostics: &mut Vec<Diagnostic>) {
    let Some(target) = ast.child(perform, Prop::Target) else {
        return;
    };
    let segments = ast.references_of(target, Prop::Segments);
    if let Some(found) = segments.last().and_then(|segment| ast.reference_target(*segment))
        && is_structure(ast, found)
    {
        let written: Vec<&str> = segments.iter().map(|segment| ast.reference_text(*segment)).collect();
        diagnostics.push(Diagnostic {
            severity: Severity::Error,
            offset: offset_of(ast, perform, Prop::Target),
            message: format!("'{}' is not an action or a use case, so it cannot be performed.", written.join(".")),
        });
    }
}

fn is_structure(ast: &Ast, node: NodeId) -> bool {
    use Type::*;
    matches!(ast[node].ty, PartUsage | PortUsage | AttributeUsage | ActorUsage | SubjectUsage | ConnectionUsage)
        || (ast[node].ty == OtherUsage && matches!(ast.text(node, Prop::Kind), Some("item" | "occurrence" | "enum")))
}

/// ortho does not load the standard library, so some references name nothing it
/// has, and are not errors:
///
/// - an import of a library package, such as `ScalarValues`, which imports nothing:
///   a name it would have supplied fails where it is used instead;
/// - a name of the library itself, `SpatialItems::SpatialItem`, or one an import of
///   the library brings in, which a view shows as written.
///
/// A typo is still an error: the library has to have the name. And a chain's later
/// segments, after one that failed, are not reported again.
fn may_name_nothing(ast: &Ast, logged: &Logged) -> bool {
    let container = logged.reference.node;
    if ast[container].ty == Type::ImportDecl {
        return true;
    }
    let index = logged.index.unwrap_or(0);
    if ast[container].ty == Type::ConnectorEnd && index > 0 {
        let previous = ast.references_of(container, Prop::Segments).get(index - 1).copied();
        if previous.is_none_or(|segment| ast.reference_target(segment).is_none()) {
            return true;
        }
    }
    names_library_element(ast, container, ast.reference_text(logged.reference))
}

fn names_library_element(ast: &Ast, container: NodeId, text: &str) -> bool {
    if let Some((package, name)) = text.rsplit_once("::") {
        return library_names(package, false).contains(name);
    }
    let name = text;
    let mut namespace = Some(container);
    while let Some(current) = namespace {
        for import in ast.children(current, Prop::Imports) {
            if ast.target(import, Prop::Target).is_some() {
                continue;
            }
            let path = ast.text(import, Prop::Target).unwrap_or("");
            let recursive = ast.flag(import, Prop::Recursive);
            let brought = if ast.flag(import, Prop::Wildcard) || recursive {
                library_names(path, recursive).contains(name)
            } else {
                match path.rsplit_once("::") {
                    Some((package, last)) => last == name && library_names(package, false).contains(name),
                    None => path == name && library_names("", false).contains(name),
                }
            };
            if brought {
                return true;
            }
        }
        namespace = ast[current].parent;
    }
    false
}

fn library_package(name: &str) -> Option<&'static LibraryPackage> {
    LIBRARY.iter().find(|package| package.name == name)
}

/// The names a library package shows: its own and its public imports'; with `nested`, its packages' too.
fn library_names(package: &str, nested: bool) -> HashSet<&'static str> {
    fn visit(name: &str, seen: &mut HashSet<&'static str>, found: &mut HashSet<&'static str>) {
        let Some(entry) = library_package(name) else {
            return;
        };
        if !seen.insert(entry.name) {
            return;
        }
        found.extend(entry.names.iter().copied());
        for import in entry.imports {
            visit(import, seen, found);
        }
    }
    let mut found = HashSet::new();
    let mut seen = HashSet::new();
    visit(package, &mut seen, &mut found);
    if nested {
        let prefix = format!("{package}::");
        for entry in LIBRARY.iter().filter(|entry| entry.name.starts_with(&prefix)) {
            visit(entry.name, &mut seen, &mut found);
        }
    }
    found
}
