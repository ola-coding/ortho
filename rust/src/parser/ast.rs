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
    /// The text of a reference, which name resolution follows.
    Reference(String),
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
            Some(Value::Text(text) | Value::Reference(text)) => Some(text),
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
            Value::Text(text) | Value::Reference(text) => Some(text.as_str()),
            _ => None,
        })
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
