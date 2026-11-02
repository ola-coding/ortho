//! Resolves names as SysML v2 does. A name's first segment is looked up in the
//! namespace it is written in, among its own members and aliases, what a definition
//! inherits through `:>` and what a namespace imports, then in each namespace around
//! it, and last among the top-level elements. Each further `::` segment is a member
//! of the one before, or something it imports publicly. An element answers to its
//! short name too, and an unnamed one to the name it takes on (`effective_name`).
//!
//! Connector-end chains continue with dots: each segment after the first is a
//! feature of the one before: of its own, of its type, inherited ones included, or of
//! what it subsets or references.
//!
//! A reference is resolved the first time something needs it, which may be while
//! resolving another, so what is found depends on the order; this keeps Langium's.
use super::ast::{Ast, NodeId, Prop, RefId, Type};
use crate::model::graph::effective_name;
use std::collections::{HashMap, HashSet};

/// What a lookup that ran into a reference still being resolved gives up with.
type R<T> = Result<T, String>;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Link {
    Resolving,
    To(NodeId),
    Failed,
}

/// A reference resolved, in the order its file's references were.
#[derive(Debug)]
pub struct Logged {
    pub reference: RefId,
    /// Its place in its list, when it was resolved in its turn rather than on demand.
    pub index: Option<usize>,
    pub error: Option<String>,
}

pub struct Linked {
    pub targets: HashMap<RefId, NodeId>,
    /// Per file, in the order the files were given.
    pub logs: Vec<Vec<Logged>>,
}

/// What a reference may name.
#[derive(Clone, Copy)]
enum Kind {
    Member,
    Definition,
    PortDef,
    ActionDef,
    UseCaseDef,
    Feature,
}

impl Kind {
    fn of(container: Type, prop: Prop) -> Kind {
        use Type::*;
        match (container, prop) {
            (ImportDecl | Expose | Dependency, _) => Kind::Member,
            (ConnectorEnd, _) => Kind::Feature,
            (ActionUsage, _) => Kind::ActionDef,
            (UseCaseUsage | IncludeUsage, _) => Kind::UseCaseDef,
            (PortUsage, _) => Kind::PortDef,
            _ => Kind::Definition,
        }
    }

    fn name(self) -> &'static str {
        match self {
            Kind::Member => "Member",
            Kind::Definition => "Definition",
            Kind::PortDef => "PortDef",
            Kind::ActionDef => "ActionDef",
            Kind::UseCaseDef => "UseCaseDef",
            Kind::Feature => "Feature",
        }
    }

    fn fits(self, ty: Type) -> bool {
        match self {
            Kind::Member => ty.is_member(),
            Kind::Definition => ty.is_definition(),
            Kind::PortDef => ty == Type::PortDef,
            Kind::ActionDef => ty == Type::ActionDef,
            Kind::UseCaseDef => ty == Type::UseCaseDef,
            Kind::Feature => ty.is_feature(),
        }
    }
}

pub struct Linker<'a> {
    ast: &'a Ast,
    roots: &'a [NodeId],
    links: HashMap<RefId, Link>,
    logs: Vec<Vec<Logged>>,
    /// The elements outside any package, by name and by short name.
    top_level: Option<HashMap<&'a str, NodeId>>,
    /// Namespaces whose import is being resolved. A lookup does not consult their
    /// imports, which would be resolving the import by itself.
    resolving_imports: HashSet<NodeId>,
    /// Aliases being followed, so one naming itself ends rather than loops.
    following_aliases: HashSet<NodeId>,
    /// Features whose features are being gathered, so one subsetting itself ends: one
    /// set for each reference being resolved, since resolving one can set off
    /// resolving another, which needs the same features afresh.
    gatherings: Vec<HashSet<NodeId>>,
    /// References being resolved. A lookup does not follow one, which would be
    /// resolving it by itself: a usage's type is not looked for among the features
    /// that type gives it.
    linking: HashSet<RefId>,
}

impl<'a> Linker<'a> {
    pub fn new(ast: &'a Ast, roots: &'a [NodeId]) -> Linker<'a> {
        Linker {
            ast,
            roots,
            links: HashMap::new(),
            logs: roots.iter().map(|_| Vec::new()).collect(),
            top_level: None,
            resolving_imports: HashSet::new(),
            following_aliases: HashSet::new(),
            gatherings: vec![HashSet::new()],
            linking: HashSet::new(),
        }
    }

    /// Resolves every reference, file by file, each in its turn unless something
    /// needed it sooner.
    pub fn link(mut self) -> Linked {
        for root in self.roots {
            for node in self.ast.with_all_contents(*root) {
                for (reference, index) in self.ast.references(node) {
                    if !self.links.contains_key(&reference) {
                        self.resolve_reference(reference, index);
                    }
                }
            }
        }
        let targets = self.links.iter().filter_map(|(reference, link)| match link {
            Link::To(target) => Some((*reference, *target)),
            _ => None,
        });
        Linked { targets: targets.collect(), logs: self.logs }
    }

    fn resolve_reference(&mut self, reference: RefId, index: Option<usize>) {
        self.links.insert(reference, Link::Resolving);
        let text = self.ast.reference_text(reference);
        let found = self.candidate(reference, index, text);
        let (link, error) = match found {
            Ok(Some(target)) => (Link::To(target), None),
            Ok(None) => {
                let kind = Kind::of(self.ast[reference.node].ty, self.prop_of(reference));
                (Link::Failed, Some(format!("Could not resolve reference to {} named '{text}'.", kind.name())))
            }
            Err(thrown) => {
                (Link::Failed, Some(format!("An error occurred while resolving reference to '{text}': {thrown}")))
            }
        };
        self.links.insert(reference, link);
        let root = self.ast.root_of(reference.node);
        let file = self.roots.iter().position(|candidate| *candidate == root).unwrap();
        self.logs[file].push(Logged { reference, index, error });
    }

    fn prop_of(&self, reference: RefId) -> Prop {
        self.ast[reference.node].fields[reference.field as usize].prop
    }

    /// What a reference names, resolving it if nothing has yet.
    fn target_of(&mut self, reference: RefId) -> R<Option<NodeId>> {
        if !self.links.contains_key(&reference) {
            self.resolve_reference(reference, None);
        }
        match self.links[&reference] {
            Link::To(target) => Ok(Some(target)),
            Link::Failed => Ok(None),
            Link::Resolving => Err(format!(
                "Cyclic reference resolution detected: {}/{} (symbol '{}')",
                self.ast.node_path(reference.node),
                self.prop_of(reference).name(),
                self.ast.reference_text(reference)
            )),
        }
    }

    /// What a reference names, unless it is being resolved.
    fn follow(&mut self, reference: Option<RefId>) -> R<Option<NodeId>> {
        match reference {
            Some(reference) if !self.linking.contains(&reference) => self.target_of(reference),
            _ => Ok(None),
        }
    }

    fn candidate(&mut self, reference: RefId, index: Option<usize>, text: &'a str) -> R<Option<NodeId>> {
        let container = reference.node;
        let ty = self.ast[container].ty;
        let found = if ty == Type::ConnectorEnd {
            // A reference resolved on demand comes without its index, so a chain's
            // segment finds its own.
            let segments = self.ast.references_of(container, Prop::Segments);
            let index = index.or_else(|| segments.iter().position(|segment| *segment == reference)).unwrap_or(0);
            if index > 0 {
                let previous = self.target_of(segments[index - 1])?;
                self.while_linking(reference, |linker| linker.feature_named(previous, text))?
            } else {
                self.while_linking(reference, |linker| linker.resolve(container, text, Some(Kind::Feature), None))?
            }
        } else {
            let kind = Kind::of(ty, self.prop_of(reference));
            let importer = if ty == Type::ImportDecl { self.ast[container].parent } else { None };
            self.while_linking(reference, |linker| {
                linker.without_imports_of(importer, |linker| linker.resolve(container, text, Some(kind), None))
            })?
        };
        if let Some(found) = found
            && text.is_empty()
        {
            return Err(format!("Node at path {} has no name.", self.ast.node_path(found)));
        }
        Ok(found)
    }

    fn while_linking(
        &mut self,
        reference: RefId,
        look_up: impl FnOnce(&mut Self) -> R<Option<NodeId>>,
    ) -> R<Option<NodeId>> {
        self.linking.insert(reference);
        self.gatherings.push(HashSet::new());
        let found = look_up(self);
        self.gatherings.pop();
        self.linking.remove(&reference);
        found
    }

    fn without_imports_of(
        &mut self,
        namespace: Option<NodeId>,
        look_up: impl FnOnce(&mut Self) -> R<Option<NodeId>>,
    ) -> R<Option<NodeId>> {
        let Some(namespace) = namespace.filter(|namespace| !self.resolving_imports.contains(namespace)) else {
            return look_up(self);
        };
        self.resolving_imports.insert(namespace);
        let found = look_up(self);
        self.resolving_imports.remove(&namespace);
        found
    }

    /// A name's `::` segments, the first looked up from `from`, each next a member of the one before.
    fn resolve(&mut self, from: NodeId, text: &str, kind: Option<Kind>, exclude: Option<NodeId>) -> R<Option<NodeId>> {
        let segments: Vec<&str> = text.split("::").collect();
        let fits = |i: usize| if i == segments.len() - 1 { kind } else { None };
        let mut found = self.look_up(from, segments[0], fits(0), exclude)?;
        for (i, segment) in segments.iter().enumerate().skip(1) {
            let Some(current) = found else {
                break;
            };
            found = self.visible_member(current, segment, fits(i), &mut HashSet::new(), false)?;
        }
        Ok(found)
    }

    fn look_up(&mut self, from: NodeId, name: &str, kind: Option<Kind>, exclude: Option<NodeId>) -> R<Option<NodeId>> {
        // What an element inherits is visible in its body, not in its own
        // declaration: a part def's supertypes, a connection's ends.
        let declaring = if self.ast[from].ty == Type::ConnectorEnd { self.ast[from].parent } else { Some(from) };
        let mut namespace = Some(from);
        while let Some(current) = namespace {
            let inherited = current != from && Some(current) != declaring;
            if let Some(found) = self.owned_member(current, name, kind, inherited, Some(from), exclude)? {
                return Ok(Some(found));
            }
            if let Some(found) = self.imported(current, name, kind, false, &mut HashSet::new())? {
                return Ok(Some(found));
            }
            namespace = self.ast[current].parent;
        }
        let found = self.top_level_elements().get(name).copied();
        Ok(found.filter(|found| self.fits(*found, kind)))
    }

    /// What a namespace shows outside: its own members and its public imports, or
    /// all its imports for an `import all`.
    fn visible_member(
        &mut self,
        namespace: NodeId,
        name: &str,
        kind: Option<Kind>,
        seen: &mut HashSet<NodeId>,
        all: bool,
    ) -> R<Option<NodeId>> {
        if let Some(found) = self.owned_member(namespace, name, kind, true, None, None)? {
            return Ok(Some(found));
        }
        self.imported(namespace, name, kind, !all, seen)
    }

    /// An unnamed element takes its name from what it refers to, so it does not
    /// answer that reference itself: `perform fly` names fly.
    fn eligible(&self, member: NodeId, from: Option<NodeId>, exclude: Option<NodeId>) -> bool {
        if Some(member) == exclude {
            return false;
        }
        let Some(from) = from else {
            return true;
        };
        if self.ast.text(member, Prop::Name).is_some() {
            return true;
        }
        let mut container = Some(from);
        while let Some(current) = container {
            if current == member {
                return false;
            }
            container = self.ast[current].parent;
        }
        true
    }

    fn owned_member(
        &mut self,
        namespace: NodeId,
        name: &str,
        kind: Option<Kind>,
        inherited: bool,
        from: Option<NodeId>,
        exclude: Option<NodeId>,
    ) -> R<Option<NodeId>> {
        for member in self.ast.contents(namespace) {
            if !self.eligible(member, from, exclude) {
                continue;
            }
            if self.ast[member].ty == Type::AliasMember {
                if self.answers_to(member, name)
                    && let Some(target) = self.aliased(member)?
                    && self.fits(target, kind)
                {
                    return Ok(Some(target));
                }
            } else if self.answers_to(member, name) && self.fits(member, kind) {
                return Ok(Some(member));
            }
        }
        // A definition's body sees what it inherits, a usage's what its type has. A
        // perform's target is not followed: the name may be part of it.
        let ty = self.ast[namespace].ty;
        let reachable = if !inherited {
            Vec::new()
        } else if ty == Type::PartDef {
            self.inherited_features(namespace)?
        } else if ty.is_feature() {
            self.features_of(Some(namespace), false)?
        } else {
            Vec::new()
        };
        let eligible = |linker: &Self, feature: NodeId| {
            linker.eligible(feature, from, exclude) && linker.answers_to(feature, name) && linker.fits(feature, kind)
        };
        Ok(reachable.into_iter().find(|feature| eligible(self, *feature)))
    }

    fn aliased(&mut self, alias: NodeId) -> R<Option<NodeId>> {
        if !self.following_aliases.insert(alias) {
            return Ok(None);
        }
        let container = self.ast[alias].parent.unwrap();
        let target = self.ast.text(alias, Prop::Target).unwrap_or("");
        let found = self.resolve(container, target, None, None);
        self.following_aliases.remove(&alias);
        found
    }

    /// What a namespace's imports bring in: `Pkg::Element` that element, under the
    /// name the import gives it, `Pkg::*` the members of Pkg, and `::**` everything
    /// nested below as well. An import of a package ortho does not have, such as the
    /// standard library's, brings in nothing.
    fn imported(
        &mut self,
        namespace: NodeId,
        name: &str,
        kind: Option<Kind>,
        public_only: bool,
        seen: &mut HashSet<NodeId>,
    ) -> R<Option<NodeId>> {
        if seen.contains(&namespace) || self.resolving_imports.contains(&namespace) {
            return Ok(None);
        }
        seen.insert(namespace);
        let imports: Vec<NodeId> = self.ast.children(namespace, Prop::Imports).collect();
        for import in imports {
            if public_only && self.ast.text(import, Prop::Visibility) != Some("public") {
                continue;
            }
            let reference = self.ast.reference_of(import, Prop::Target);
            let Some(target) = self.follow(reference)? else {
                continue;
            };
            // An alias imported by name brings in what it names, under its own name.
            let written = self.ast.text(import, Prop::Target).unwrap_or("");
            let imported_name = written.rsplit("::").next().unwrap_or(written);
            let mut found = if self.ast.flag(import, Prop::Wildcard) {
                let all = self.ast.flag(import, Prop::All);
                self.visible_member(target, name, kind, seen, all)?
            } else if (self.answers_to(target, name) || imported_name == name) && self.fits(target, kind) {
                Some(target)
            } else {
                None
            };
            if found.is_none() && self.ast.flag(import, Prop::Recursive) {
                found = self.nested_member(target, name, kind, seen)?;
            }
            if found.is_some() {
                return Ok(found);
            }
        }
        Ok(None)
    }

    /// A member of a namespace or of any namespace nested in it, for `::**`.
    fn nested_member(
        &mut self,
        namespace: NodeId,
        name: &str,
        kind: Option<Kind>,
        seen: &mut HashSet<NodeId>,
    ) -> R<Option<NodeId>> {
        if let Some(found) = self.visible_member(namespace, name, kind, seen, false)? {
            return Ok(Some(found));
        }
        for member in self.ast.contents(namespace) {
            if let Some(found) = self.nested_member(member, name, kind, seen)? {
                return Ok(Some(found));
            }
        }
        Ok(None)
    }

    /// A feature of a feature, or an alias declared among them.
    fn feature_named(&mut self, feature: Option<NodeId>, name: &str) -> R<Option<NodeId>> {
        let features = self.features_of(feature, true)?;
        if let Some(found) = features.into_iter().find(|found| self.answers_to(*found, name)) {
            return Ok(Some(found));
        }
        let Some(feature) = feature else {
            return Ok(None);
        };
        let ty = self.type_of(feature)?;
        let own = self.ast.children(feature, Prop::Aliases);
        let aliases: Vec<NodeId> =
            own.chain(ty.into_iter().flat_map(|ty| self.ast.children(ty, Prop::Aliases))).collect();
        match aliases.into_iter().find(|alias| self.answers_to(*alias, name)) {
            Some(alias) => self.aliased(alias),
            None => Ok(None),
        }
    }

    /// A feature's features: its own, those of its type and the type's supertypes,
    /// and those of what it subsets, references or redefines: a perform
    /// `perform action takePhoto references takePicture` has takePicture's.
    fn features_of(&mut self, feature: Option<NodeId>, through_target: bool) -> R<Vec<NodeId>> {
        let Some(feature) = feature else {
            return Ok(Vec::new());
        };
        if !self.gatherings.last_mut().unwrap().insert(feature) {
            return Ok(Vec::new());
        }
        let features = self.gather_features(feature, through_target);
        self.gatherings.last_mut().unwrap().remove(&feature);
        features
    }

    fn gather_features(&mut self, feature: NodeId, through_target: bool) -> R<Vec<NodeId>> {
        let ast = self.ast;
        let ty = self.type_of(feature)?;
        let mut features = own_features(ast, feature);
        if let Some(ty) = ty {
            features.extend(self.definition_features(ty, &mut HashSet::new())?);
        }
        // What a usage is typed by where ortho keeps the type as written, `in c : Counter`.
        let written_type =
            if ast.reference_of(feature, Prop::Type).is_none() { ast.text(feature, Prop::Type) } else { None };
        for name in written_type.into_iter().chain(ast.texts(feature, Prop::Types)) {
            let name = name.strip_prefix('~').unwrap_or(name);
            let from = ast[feature].parent.unwrap_or(feature);
            if let Some(definition) = self.resolve(from, name, None, None)? {
                features.extend(self.definition_features(definition, &mut HashSet::new())?);
            }
        }
        for prop in [Prop::Subsets, Prop::References, Prop::Redefines] {
            for chain in ast.texts(feature, prop) {
                let related = self.resolve_chain(feature, chain)?;
                features.extend(self.features_of(related, true)?);
            }
        }
        if through_target && ast[feature].ty == Type::PerformUsage {
            let target = ast.child(feature, Prop::Target);
            let last = target.and_then(|target| ast.references_of(target, Prop::Segments).pop());
            let followed = self.follow(last)?;
            features.extend(self.features_of(followed, true)?);
        }
        // A connector's named end has the features of what it references.
        if ast[feature].ty == Type::ConnectorEnd {
            let last = ast.references_of(feature, Prop::Segments).pop();
            let followed = self.follow(last)?;
            features.extend(self.features_of(followed, true)?);
        }
        Ok(features)
    }

    /// A chain written as text, `a.b.c`, resolved from where its feature stands. It
    /// never names the feature itself: `part :>> engine` redefines another engine.
    fn resolve_chain(&mut self, feature: NodeId, chain: &str) -> R<Option<NodeId>> {
        let mut names = chain.split('.');
        let first = names.next().unwrap_or("");
        let mut found = self.resolve(feature, first, None, Some(feature))?;
        for name in names {
            found = match found {
                Some(current) if current != feature => self.feature_named(Some(current), name)?,
                _ => None,
            };
        }
        Ok(found.filter(|found| *found != feature))
    }

    /// What a usage is typed by, where that is a reference ortho resolves.
    fn type_of(&mut self, feature: NodeId) -> R<Option<NodeId>> {
        let reference = self.ast.reference_of(feature, Prop::Type);
        self.follow(reference)
    }

    /// A definition's features, and those of the definitions it specializes.
    fn definition_features(&mut self, definition: NodeId, seen: &mut HashSet<NodeId>) -> R<Vec<NodeId>> {
        if !seen.insert(definition) {
            return Ok(Vec::new());
        }
        let mut features = own_features(self.ast, definition);
        for supertype in self.ast.references_of(definition, Prop::Supertypes) {
            if let Some(resolved) = self.follow(Some(supertype))? {
                features.extend(self.definition_features(resolved, seen)?);
            }
        }
        Ok(features)
    }

    /// Features a definition inherits through `:>` but does not declare itself.
    /// Locally declared members shadow inherited ones, so only genuinely inherited
    /// names are added.
    fn inherited_features(&mut self, definition: NodeId) -> R<Vec<NodeId>> {
        let ast = self.ast;
        let mut taken: HashSet<Option<&str>> =
            own_features(ast, definition).iter().map(|f| effective_name(ast, *f)).collect();
        let mut inherited = Vec::new();
        for supertype in ast.references_of(definition, Prop::Supertypes) {
            let Some(resolved) = self.follow(Some(supertype))? else {
                continue;
            };
            for feature in self.definition_features(resolved, &mut HashSet::new())? {
                let name = effective_name(ast, feature);
                if name.is_some_and(|name| !name.is_empty()) && taken.insert(name) {
                    inherited.push(feature);
                }
            }
        }
        Ok(inherited)
    }

    fn fits(&self, node: NodeId, kind: Option<Kind>) -> bool {
        kind.is_none_or(|kind| kind.fits(self.ast[node].ty))
    }

    fn answers_to(&self, node: NodeId, name: &str) -> bool {
        effective_name(self.ast, node) == Some(name) || self.ast.text(node, Prop::ShortName) == Some(name)
    }

    fn top_level_elements(&mut self) -> &HashMap<&'a str, NodeId> {
        let (ast, roots) = (self.ast, self.roots);
        self.top_level.get_or_insert_with(|| {
            let mut elements = HashMap::new();
            for root in roots {
                // Only what has a name of its own is indexed, whatever else it answers to.
                for node in ast.contents(*root).into_iter().filter(|node| ast.text(*node, Prop::Name).is_some()) {
                    for name in [effective_name(ast, node), ast.text(node, Prop::ShortName)].into_iter().flatten() {
                        if !name.is_empty() {
                            elements.entry(name).or_insert(node);
                        }
                    }
                }
            }
            elements
        })
    }
}

/// A usage's own features: its members, and a connection's named ends.
fn own_features(ast: &Ast, node: NodeId) -> Vec<NodeId> {
    let mut features: Vec<NodeId> =
        ast.children(node, Prop::Members).filter(|member| ast[*member].ty.is_feature()).collect();
    // A connector's end may be named: `connect wheel ::> axle.wheel to ...`.
    let ends = [ast.child(node, Prop::Source), ast.child(node, Prop::Target)]
        .into_iter()
        .flatten()
        .chain(ast.children(node, Prop::Ends));
    features.extend(ends.filter(|end| {
        ast[*end].ty == Type::ConnectorEnd && ast.text(*end, Prop::Name).is_some_and(|name| !name.is_empty())
    }));
    features
}
