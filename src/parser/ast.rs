//! The tree of a model: what `grammar.rs` builds and everything after it reads. One
//! arena of nodes, each with its type and what the grammar assigned to it, in the
//! order it assigned it. Name resolution walks the contents of any node, whatever
//! its type, so a node needs no Rust type of its own.
use std::ops::{Index, IndexMut};

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct NodeId(u32);

/// A node's type, as the grammar infers it.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Type {
    Model,
    PackageDecl,
    ImportDecl,
    AliasMember,
    ElementFilter,
    Expose,
    Comment,
    Documentation,
    TextualRepresentation,
    /// An element whose keyword has not been read yet.
    Element,
    PartDef,
    PortDef,
    InterfaceDef,
    AttributeDef,
    ActionDef,
    UseCaseDef,
    OtherDefinition,
    PartUsage,
    AttributeUsage,
    PortUsage,
    ActionUsage,
    UseCaseUsage,
    ActorUsage,
    SubjectUsage,
    IncludeUsage,
    PerformUsage,
    ConnectionUsage,
    AllocationUsage,
    MessageUsage,
    ViewUsage,
    SatisfyUsage,
    BindingUsage,
    FlowUsage,
    MetadataUsage,
    Dependency,
    OtherUsage,
    SuccessionUsage,
    TransitionUsage,
    StateAction,
    AcceptAction,
    SendAction,
    AssignAction,
    TerminateAction,
    IfAction,
    LoopAction,
    ForAction,
    DefaultUsage,
    SuccessionEnd,
    ActionBlock,
    ElseIf,
    Effect,
    ConnectorEnd,
    Multiplicity,
    IfExpression,
    OperatorExpression,
    PrefixExpression,
    ClassificationExpression,
    ExtentExpression,
    FeatureChainExpression,
    MetadataAccessExpression,
    CollectExpression,
    SelectExpression,
    InvocationOperation,
    BracketExpression,
    IndexExpression,
    LiteralExpression,
    ConstructorExpression,
    FeatureReferenceExpression,
    SequenceExpression,
    BodyExpression,
    Argument,
}

/// What the grammar assigns to, each by the name it has there.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Prop {
    About,
    Action,
    Aliases,
    All,
    Annotations,
    Arguments,
    Assigned,
    Body,
    Chain,
    Clients,
    Collection,
    Condition,
    Conjugated,
    CrossName,
    Crosses,
    Definition,
    Direction,
    Effect,
    Element,
    Else,
    ElseBlock,
    ElseIf,
    EndMultiplicity,
    Ends,
    Exposes,
    Filters,
    First,
    Function,
    Guard,
    Imports,
    Index,
    Invocation,
    IsAbstract,
    IsConstant,
    IsDefault,
    IsDerived,
    IsElse,
    IsEnd,
    IsIndividual,
    IsInitial,
    IsLibrary,
    IsParallel,
    IsReference,
    IsReturn,
    IsStandard,
    IsVariant,
    IsVariation,
    Items,
    Kind,
    Language,
    Left,
    Locale,
    LoopBody,
    Lower,
    Member,
    Members,
    Multiplicities,
    Multiplicity,
    Name,
    Negated,
    Next,
    Operand,
    Operator,
    Packages,
    Payload,
    PayloadMultiplicity,
    PayloadName,
    PayloadValue,
    Performed,
    PortionKind,
    PrefixMetadata,
    Recursive,
    Redefines,
    Reference,
    References,
    Requirement,
    Result,
    Right,
    Satisfier,
    Segments,
    ShortName,
    Source,
    Subsets,
    Succession,
    Supertypes,
    Suppliers,
    Target,
    Then,
    ThenBlock,
    To,
    Trigger,
    TriggerValue,
    Type,
    Types,
    Until,
    Upper,
    UseCase,
    Value,
    Variable,
    Via,
    Visibility,
    Wildcard,
}

#[derive(Debug)]
pub enum Value {
    /// A name or a chain as written, without quotes, or a keyword or literal.
    Text(String),
    Flag,
    /// A reference: its text, and what name resolution found it to name.
    Reference {
        text: String,
        target: Option<NodeId>,
    },
    Child(NodeId),
}

#[derive(Debug)]
pub struct Field {
    pub prop: Prop,
    /// Assigned with `+=`, so its values make a list.
    pub list: bool,
    /// Where the value starts in the file, in bytes.
    pub offset: u32,
    pub value: Value,
}

#[derive(Debug)]
pub struct Node {
    pub ty: Type,
    pub parent: Option<NodeId>,
    /// Where the node starts in the file, in bytes.
    pub offset: u32,
    pub fields: Vec<Field>,
}

#[derive(Default, Debug)]
pub struct Ast {
    nodes: Vec<Node>,
}

impl Ast {
    pub(super) fn add(&mut self, ty: Type, offset: u32) -> NodeId {
        self.nodes.push(Node { ty, parent: None, offset, fields: Vec::new() });
        NodeId(self.nodes.len() as u32 - 1)
    }

    pub(super) fn len(&self) -> usize {
        self.nodes.len()
    }

    /// Forgets the nodes added since it held `len` of them.
    pub(super) fn truncate(&mut self, len: usize) {
        self.nodes.truncate(len);
    }

    /// What is assigned to `prop`: the last value, since assigning again replaces it.
    fn value(&self, id: NodeId, prop: Prop) -> Option<&Value> {
        self[id].fields.iter().rev().find(|field| field.prop == prop).map(|field| &field.value)
    }

    pub fn text(&self, id: NodeId, prop: Prop) -> Option<&str> {
        match self.value(id, prop) {
            Some(Value::Text(text) | Value::Reference { text, .. }) => Some(text),
            _ => None,
        }
    }

    pub fn flag(&self, id: NodeId, prop: Prop) -> bool {
        self.value(id, prop).is_some()
    }

    pub fn child(&self, id: NodeId, prop: Prop) -> Option<NodeId> {
        match self.value(id, prop) {
            Some(Value::Child(child)) => Some(*child),
            _ => None,
        }
    }

    /// The nodes assigned to `prop`, in the order they were written.
    pub fn children(&self, id: NodeId, prop: Prop) -> impl Iterator<Item = NodeId> + '_ {
        self[id].fields.iter().filter(move |field| field.prop == prop).filter_map(|field| match field.value {
            Value::Child(child) => Some(child),
            _ => None,
        })
    }

    /// The names or references assigned to `prop`, in the order they were written.
    pub fn texts(&self, id: NodeId, prop: Prop) -> impl Iterator<Item = &str> + '_ {
        self[id].fields.iter().filter(move |field| field.prop == prop).filter_map(|field| match &field.value {
            Value::Text(text) | Value::Reference { text, .. } => Some(text.as_str()),
            _ => None,
        })
    }
}

/// A reference: the node it is on, and which of the node's fields it is.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct RefId {
    pub node: NodeId,
    pub field: u32,
}

impl Ast {
    /// A reference's text.
    pub fn reference_text(&self, reference: RefId) -> &str {
        match &self[reference.node].fields[reference.field as usize].value {
            Value::Reference { text, .. } => text,
            value => panic!("not a reference: {value:?}"),
        }
    }

    pub(super) fn set_target(&mut self, reference: RefId, found: NodeId) {
        if let Value::Reference { target, .. } = &mut self[reference.node].fields[reference.field as usize].value {
            *target = Some(found);
        }
    }

    /// What a reference names, once linked.
    pub fn reference_target(&self, reference: RefId) -> Option<NodeId> {
        match self[reference.node].fields[reference.field as usize].value {
            Value::Reference { target, .. } => target,
            _ => None,
        }
    }

    /// The references assigned to `prop`, in the order they were written.
    pub fn references_of(&self, id: NodeId, prop: Prop) -> Vec<RefId> {
        let fields = self[id].fields.iter().enumerate();
        fields
            .filter(|(_, field)| field.prop == prop && matches!(field.value, Value::Reference { .. }))
            .map(|(i, _)| RefId { node: id, field: i as u32 })
            .collect()
    }

    /// The reference assigned to `prop`, if what is assigned is one.
    pub fn reference_of(&self, id: NodeId, prop: Prop) -> Option<RefId> {
        self.references_of(id, prop).pop()
    }

    /// What the reference assigned to `prop` names.
    pub fn target(&self, id: NodeId, prop: Prop) -> Option<NodeId> {
        self.reference_of(id, prop).and_then(|reference| self.reference_target(reference))
    }

    /// A node's fields grouped by property, each property where it was first
    /// assigned: the order every walk of the tree goes in. Assigned again, a single
    /// value replaces the one before.
    fn grouped(&self, id: NodeId) -> Vec<(u32, &Field)> {
        let fields = &self[id].fields;
        let mut seen: Vec<Prop> = Vec::new();
        let mut grouped = Vec::with_capacity(fields.len());
        for field in fields {
            if seen.contains(&field.prop) {
                continue;
            }
            seen.push(field.prop);
            if field.list {
                let of_prop = fields.iter().enumerate().filter(|(_, other)| other.prop == field.prop);
                grouped.extend(of_prop.map(|(i, other)| (i as u32, other)));
            } else {
                let last = fields.iter().rposition(|other| other.prop == field.prop).unwrap();
                grouped.push((last as u32, &fields[last]));
            }
        }
        grouped
    }

    /// The nodes directly inside a node.
    pub fn contents(&self, id: NodeId) -> Vec<NodeId> {
        self.grouped(id)
            .into_iter()
            .filter_map(|(_, field)| match field.value {
                Value::Child(child) => Some(child),
                _ => None,
            })
            .collect()
    }

    /// A node and everything inside it, each node before what it contains.
    pub fn with_all_contents(&self, id: NodeId) -> Vec<NodeId> {
        let mut all = vec![id];
        for child in self.contents(id) {
            all.extend(self.with_all_contents(child));
        }
        all
    }

    /// The references on a node, each with its place in its list if it is in one.
    pub fn references(&self, id: NodeId) -> Vec<(RefId, Option<usize>)> {
        let mut references = Vec::new();
        let mut last: Option<(Prop, usize)> = None;
        for (i, field) in self.grouped(id) {
            if matches!(field.value, Value::Reference { .. }) {
                let index = match last {
                    Some((prop, index)) if prop == field.prop => index + 1,
                    _ => 0,
                };
                last = Some((field.prop, index));
                references.push((RefId { node: id, field: i }, field.list.then_some(index)));
            }
        }
        references
    }

    /// The node at the root of a file that holds `id`.
    pub fn root_of(&self, id: NodeId) -> NodeId {
        let mut root = id;
        while let Some(parent) = self[root].parent {
            root = parent;
        }
        root
    }

    /// Where a node is in its file: `/packages@0/members@2`.
    pub fn node_path(&self, id: NodeId) -> String {
        let Some(parent) = self[id].parent else {
            return String::new();
        };
        let fields = &self[parent].fields;
        let at = fields.iter().position(|field| matches!(field.value, Value::Child(child) if child == id)).unwrap();
        let field = &fields[at];
        let segment = if field.list {
            let index = fields[..at].iter().filter(|other| other.prop == field.prop).count();
            format!("{}@{index}", field.prop.name())
        } else {
            field.prop.name()
        };
        format!("{}/{segment}", self.node_path(parent))
    }
}

impl Prop {
    /// As the grammar names it: `ShortName` is `shortName`.
    pub fn name(self) -> String {
        let name = format!("{self:?}");
        format!("{}{}", name[..1].to_lowercase(), &name[1..])
    }
}

impl Type {
    pub fn is_definition(self) -> bool {
        use Type::*;
        matches!(self, PartDef | PortDef | InterfaceDef | AttributeDef | ActionDef | UseCaseDef | OtherDefinition)
    }

    pub fn is_feature(self) -> bool {
        use Type::*;
        matches!(
            self,
            PartUsage
                | PortUsage
                | AttributeUsage
                | SubjectUsage
                | ActorUsage
                | ActionUsage
                | UseCaseUsage
                | PerformUsage
                | OtherUsage
                | DefaultUsage
                | ConnectionUsage
                | AllocationUsage
                | MessageUsage
                | FlowUsage
                | BindingUsage
                | SatisfyUsage
                | IncludeUsage
                | MetadataUsage
                | ViewUsage
                | SuccessionUsage
                | TransitionUsage
                | ConnectorEnd
                | AcceptAction
                | SendAction
                | AssignAction
                | TerminateAction
                | IfAction
                | LoopAction
                | ForAction
        )
    }

    /// What a package holds: a package, or any definition or usage.
    pub fn is_member(self) -> bool {
        use Type::*;
        self == PackageDecl
            || self == Element
            || self == Dependency
            || self == StateAction
            || self.is_definition()
            || (self.is_feature() && self != ConnectorEnd)
    }
}

impl Index<NodeId> for Ast {
    type Output = Node;

    fn index(&self, id: NodeId) -> &Node {
        &self.nodes[id.0 as usize]
    }
}

impl IndexMut<NodeId> for Ast {
    fn index_mut(&mut self, id: NodeId) -> &mut Node {
        &mut self.nodes[id.0 as usize]
    }
}
