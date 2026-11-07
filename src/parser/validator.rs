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
use crate::model::packages::{collect_packages, exposed_members};
use crate::model::views::{
    DiagramKind, declared_views, diagram_kind, exposed_elements, is_sub_function, why_undrawable,
};
use std::collections::HashSet;

/// What a model leaves undone across its files, found once for the whole model and
/// reported by each file where it is. A function and what performs it, or a tree
/// and the use case it refines, may be in different files.
pub struct Gaps {
    unperformed: HashSet<NodeId>,
    unrefined: HashSet<NodeId>,
    /// `allocate` ends, the software's or the host's, that no other view draws.
    foreign_software: HashSet<NodeId>,
    foreign_hosts: HashSet<NodeId>,
}

impl Gaps {
    pub fn of(ast: &Ast, roots: &[NodeId]) -> Gaps {
        let (foreign_software, foreign_hosts) = foreign_allocation_ends(ast, roots);
        Gaps {
            unperformed: unperformed_functions(ast, roots),
            unrefined: unrefined_trees(ast, roots),
            foreign_software,
            foreign_hosts,
        }
    }
}

/// What one file reports: its unresolved references, then each node's checks,
/// with the model's `gaps` among them.
pub fn validate(ast: &Ast, root: NodeId, _text: &str, log: &[Logged], gaps: &Gaps) -> Vec<Diagnostic> {
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
        if gaps.foreign_software.contains(&node) {
            warn_foreign_end(ast, node, "is not a module the implementation view draws", &mut diagnostics);
        }
        if gaps.foreign_hosts.contains(&node) {
            warn_foreign_end(ast, node, "is not a part the physical view draws", &mut diagnostics);
        }
        if gaps.unperformed.contains(&node) {
            warn_unperformed(ast, node, &mut diagnostics);
        }
        if gaps.unrefined.contains(&node) {
            warn_unrefined(ast, node, &mut diagnostics);
        }
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

/// The functions in scope that no part or module of the solution performs.
///
/// A logical view and a solution view, an implementation or physical one, have to
/// agree: once a model declares both, every function at the bottom of a tree a
/// logical view draws needs a module the implementation view draws, or a part the
/// physical view draws, to perform it. A model without a solution view is not
/// warned of, as `perform` is optional; neither is one without a logical view, such
/// as a cloud solution drawn as an implementation view alone. In a model with a use
/// case view, only a tree that refines a use case is in scope, so a catalogue of
/// functions needs no solution until a use case asks for it.
///
/// Performing a function performs everything under it, and performing a typed
/// action, `perform beverage;`, performs its definition's tree. A typed action
/// inside a tree stands for its definition's tree, which is checked where a view
/// draws it.
fn unperformed_functions(ast: &Ast, roots: &[NodeId]) -> HashSet<NodeId> {
    if !has_view(ast, roots, &[DiagramKind::Implementation, DiagramKind::Physical]) {
        return HashSet::new();
    }
    let owners = solution_owners(ast, roots);
    let scoped_by_use_cases = has_view(ast, roots, &[DiagramKind::UseCase]);
    let refined = refined_trees(ast, roots);
    let mut seen = HashSet::new();
    let mut leaves = Vec::new();
    for view in views_of(ast, roots, DiagramKind::Logical) {
        for member in exposed_members(ast, &exposed_elements(ast, view)) {
            if is_function(ast, member) && (!scoped_by_use_cases || refined.contains(&top_of_tree(ast, member))) {
                collect_leaves(ast, member, &mut seen, &mut leaves);
            }
        }
    }
    let mut performed = HashSet::new();
    for perform in roots.iter().flat_map(|root| ast.with_all_contents(*root)) {
        if ast[perform].ty != Type::PerformUsage || !ast[perform].parent.is_some_and(|owner| owners.contains(&owner)) {
            continue;
        }
        let last = ast.child(perform, Prop::Target).and_then(|end| ast.references_of(end, Prop::Segments).pop());
        if let Some(function) = last.and_then(|segment| ast.reference_target(segment)) {
            performed.insert(function);
            performed.extend(ast.target(function, Prop::Type));
        }
    }
    let is_performed = |leaf: &NodeId| {
        std::iter::successors(Some(*leaf), |node| ast[*node].parent).any(|node| performed.contains(&node))
    };
    leaves.into_iter().filter(|leaf| !is_performed(leaf)).collect()
}

/// What may perform a function: the modules an implementation view draws, the part
/// definitions in the packages it exposes, and the parts a physical view draws, with
/// their definitions and what those specialize, whose performs the parts list too.
fn solution_owners(ast: &Ast, roots: &[NodeId]) -> HashSet<NodeId> {
    let mut owners = implementation_modules(ast, roots);
    owners.extend(physical_parts(ast, roots));
    owners
}

/// The modules the implementation views draw: the part definitions in the packages
/// they expose, nested packages' included.
fn implementation_modules(ast: &Ast, roots: &[NodeId]) -> HashSet<NodeId> {
    let mut modules = HashSet::new();
    for view in views_of(ast, roots, DiagramKind::Implementation) {
        let exposed = exposed_elements(ast, view);
        let packages: Vec<NodeId> =
            exposed.into_iter().filter(|element| ast[*element].ty == Type::PackageDecl).collect();
        for package in collect_packages(ast, &packages) {
            modules.extend(ast.children(package, Prop::Members).filter(|member| ast[*member].ty == Type::PartDef));
        }
    }
    modules
}

/// The parts the physical views draw, with their definitions and what those specialize.
fn physical_parts(ast: &Ast, roots: &[NodeId]) -> HashSet<NodeId> {
    let mut parts = HashSet::new();
    for view in views_of(ast, roots, DiagramKind::Physical) {
        for part in exposed_elements(ast, view) {
            drawn_parts(ast, part, &mut parts);
        }
    }
    parts
}

/// The `allocate` ends of the deployment views that no other view draws: software
/// that is not a usage of a module the implementation view draws, and hosts that
/// are not parts the physical view draws. A deployment view maps the other two onto
/// each other and adds nothing of its own; an end is checked only once the model
/// declares the view it comes from.
fn foreign_allocation_ends(ast: &Ast, roots: &[NodeId]) -> (HashSet<NodeId>, HashSet<NodeId>) {
    let (mut software, mut hosts) = (HashSet::new(), HashSet::new());
    let deployments = views_of(ast, roots, DiagramKind::Deployment);
    if deployments.is_empty() {
        return (software, hosts);
    }
    let modules = has_view(ast, roots, &[DiagramKind::Implementation]).then(|| implementation_modules(ast, roots));
    let parts = has_view(ast, roots, &[DiagramKind::Physical]).then(|| physical_parts(ast, roots));
    let named =
        |end: NodeId| ast.references_of(end, Prop::Segments).pop().and_then(|segment| ast.reference_target(segment));
    let allocations = deployments
        .into_iter()
        .flat_map(|view| exposed_elements(ast, view))
        .flat_map(|element| ast.with_all_contents(element))
        .filter(|node| ast[*node].ty == Type::AllocationUsage);
    for allocation in allocations {
        if let (Some(modules), Some(end)) = (&modules, ast.child(allocation, Prop::Source))
            && let Some(usage) = named(end)
            && !ast.target(usage, Prop::Type).is_some_and(|module| modules.contains(&module))
        {
            software.insert(end);
        }
        if let (Some(parts), Some(end)) = (&parts, ast.child(allocation, Prop::Target))
            && let Some(mut host) = named(end)
        {
            // A port stands for the part it is on.
            if ast[host].ty == Type::PortUsage {
                host = ast[host].parent.unwrap_or(host);
            }
            if !parts.contains(&host) {
                hosts.insert(end);
            }
        }
    }
    (software, hosts)
}

fn warn_foreign_end(ast: &Ast, end: NodeId, problem: &str, diagnostics: &mut Vec<Diagnostic>) {
    let written: Vec<&str> = ast.texts(end, Prop::Segments).collect();
    diagnostics.push(Diagnostic {
        severity: Severity::Warning,
        offset: ast[end].offset as usize,
        message: format!(
            "'{}' {problem}. A deployment view allocates what the other views draw, and adds nothing of its own.",
            written.join(".")
        ),
    });
}

/// A part, its definition and what that specializes, and the parts nested in them.
fn drawn_parts(ast: &Ast, part: NodeId, owners: &mut HashSet<NodeId>) {
    if !owners.insert(part) {
        return;
    }
    let mut definitions = Vec::new();
    let mut next: Vec<NodeId> = ast.target(part, Prop::Type).into_iter().collect();
    while let Some(definition) = next.pop() {
        if owners.insert(definition) {
            definitions.push(definition);
            next.extend(
                ast.references_of(definition, Prop::Supertypes).into_iter().filter_map(|s| ast.reference_target(s)),
            );
        }
    }
    let nested: Vec<NodeId> = std::iter::once(part)
        .chain(definitions)
        .flat_map(|owner| ast.children(owner, Prop::Members))
        .filter(|member| ast[*member].ty == Type::PartUsage)
        .collect();
    for child in nested {
        drawn_parts(ast, child, owners);
    }
}

/// The functions with none under them, from `function` down, each once.
fn collect_leaves(ast: &Ast, function: NodeId, seen: &mut HashSet<NodeId>, leaves: &mut Vec<NodeId>) {
    if !seen.insert(function) {
        return;
    }
    let children: Vec<NodeId> =
        ast.children(function, Prop::Members).filter(|member| ast[*member].ty == Type::ActionUsage).collect();
    if children.is_empty() {
        if is_function(ast, function) {
            leaves.push(function);
        }
        return;
    }
    for child in children {
        collect_leaves(ast, child, seen, leaves);
    }
}

fn warn_unperformed(ast: &Ast, function: NodeId, diagnostics: &mut Vec<Diagnostic>) {
    let named = effective_name(ast, function).map_or("this function".to_string(), |name| format!("'{name}'"));
    diagnostics.push(Diagnostic {
        severity: Severity::Warning,
        offset: offset_of(ast, function, Prop::Name),
        message: format!(
            "No part or module of the implementation or physical view performs {named}. Perform it, or a function above it, in the one that realizes it."
        ),
    });
}

/// The function trees that refine no use case, though a logical view draws them
/// beside use cases.
///
/// A refinement is optional, but a logical view that shows use cases is there to
/// tie its trees to them. Asked to fix an error, an agent may delete the
/// refinement it points at, and the view would then draw as if complete, the tree
/// without its arrow up to its use case. A tree is named by the function at its
/// top, also where a view draws one of its branches; a dependency from a typed
/// action, `beverage : MakeBeverage`, refines its definition's tree.
fn unrefined_trees(ast: &Ast, roots: &[NodeId]) -> HashSet<NodeId> {
    let mut drawn = Vec::new();
    for view in views_of(ast, roots, DiagramKind::Logical) {
        let members = exposed_members(ast, &exposed_elements(ast, view));
        if members.iter().any(|member| is_use_case(ast, *member)) {
            drawn.extend(members.into_iter().filter(|member| is_function(ast, *member)).map(|f| top_of_tree(ast, f)));
        }
    }
    let refined = refined_trees(ast, roots);
    drawn.into_iter().filter(|tree| !refined.contains(tree)).collect()
}

/// What a dependency on a use case names as its client, and for a typed action
/// its definition: the trees that refine a use case.
fn refined_trees(ast: &Ast, roots: &[NodeId]) -> HashSet<NodeId> {
    let is_use_case = |node: NodeId| is_use_case(ast, node);
    let mut refined = HashSet::new();
    for dependency in roots.iter().flat_map(|root| ast.with_all_contents(*root)) {
        if ast[dependency].ty != Type::Dependency {
            continue;
        }
        let ends =
            |prop: Prop| ast.references_of(dependency, prop).into_iter().filter_map(|end| ast.reference_target(end));
        if ends(Prop::Suppliers).any(is_use_case) {
            for client in ends(Prop::Clients) {
                refined.insert(client);
                refined.extend(ast.target(client, Prop::Type));
            }
        }
    }
    refined
}

fn is_use_case(ast: &Ast, node: NodeId) -> bool {
    matches!(ast[node].ty, Type::UseCaseDef | Type::UseCaseUsage)
}

/// The function at the top of the tree `function` is in.
fn top_of_tree(ast: &Ast, function: NodeId) -> NodeId {
    let mut top = function;
    while let Some(parent) =
        ast[top].parent.filter(|parent| matches!(ast[*parent].ty, Type::ActionDef | Type::ActionUsage))
    {
        top = parent;
    }
    top
}

fn warn_unrefined(ast: &Ast, tree: NodeId, diagnostics: &mut Vec<Diagnostic>) {
    let named = effective_name(ast, tree).map_or("This tree".to_string(), |name| format!("'{name}'"));
    diagnostics.push(Diagnostic {
        severity: Severity::Warning,
        offset: offset_of(ast, tree, Prop::Name),
        message: format!(
            "{named} refines no use case, though a logical view draws it beside use cases. Add a #refinement dependency from it to the use case it details."
        ),
    });
}

/// The views of `kind` that ortho draws.
fn views_of(ast: &Ast, roots: &[NodeId], kind: DiagramKind) -> Vec<NodeId> {
    let views = roots.iter().flat_map(|root| declared_views(ast, *root));
    views.filter(|view| diagram_kind(ast, *view) == kind && why_undrawable(ast, *view).is_none()).collect()
}

fn has_view(ast: &Ast, roots: &[NodeId], kinds: &[DiagramKind]) -> bool {
    kinds.iter().any(|kind| !views_of(ast, roots, *kind).is_empty())
}

/// Whether a logical view draws `node` as a function: an action definition, or an
/// action of its own. A typed action is one use of a tree, drawn from its definition.
fn is_function(ast: &Ast, node: NodeId) -> bool {
    ast[node].ty == Type::ActionDef || is_sub_function(ast, node)
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
