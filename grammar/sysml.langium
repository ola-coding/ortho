grammar Sysml

entry Model:
    (packages+=PackageDecl)*;

PackageDecl:
    'package' name=ID '{'
        (imports+=ImportDecl | members+=PackageMember)*
    '}';

// Imports parse but are not yet semantically enforced: all definitions are
// visible by qualified name across packages (see SysmlScopeComputation). The
// spec requires the visibility to be written out (§7.5.3), so it is here too.
ImportDecl:
    visibility=Visibility 'import' path=QualifiedName (wildcard?='::*')? ';';

Visibility returns string:
    'private' | 'protected' | 'public';

PackageMember:
    PackageDecl | PartDef | PortDef | InterfaceDef | AttributeDef | RequirementDef | UseCaseDef | ActionDef
    | PartUsage | RequirementUsage | ConnectionUsage | SatisfyUsage | UseCaseUsage | AllocationUsage | ActionUsage;

PartDef:
    'part' 'def' name=ID
    ((':>' | 'specializes') supertypes+=[PartDef:QualifiedName] (',' supertypes+=[PartDef:QualifiedName])*)?
    (';' | '{' (members+=UsageMember)* '}');

UsageMember:
    AttributeUsage | PortUsage | PartUsage | ConnectionUsage | SatisfyUsage | RequirementUsage
    | PerformUsage | AllocationUsage | ActionUsage;

PortDef:
    'port' 'def' name=ID
    ((':>' | 'specializes') supertypes+=[PortDef:QualifiedName] (',' supertypes+=[PortDef:QualifiedName])*)?
    (';' | '{' (members+=AttributeUsage)* '}');

InterfaceDef:
    'interface' 'def' name=ID
    (';' | '{' (ends+=InterfaceEnd | members+=AttributeUsage)* '}');

InterfaceEnd:
    'end' name=ID ':' type=[PortDef:QualifiedName] ';';

RequirementDef:
    'requirement' 'def' name=ID
    (';' | '{' (members+=RequirementMember)* '}');

RequirementMember:
    AttributeUsage | SubjectUsage;

SubjectUsage:
    'subject' name=ID (':' type=[PartDef:QualifiedName])? ';';

PartUsage:
    'part' name=ID (':' type=[PartDef:QualifiedName])? (multiplicity=Multiplicity)?
    (';' | '{' (members+=UsageMember)* '}');

Multiplicity:
    '[' lower=NUMBER ('..' upper=UpperBound)? ']';

UpperBound returns string:
    NUMBER | '*';

// A definition of values without identity. Its one job here is to type a
// message's payload, as the spec's own examples do.
AttributeDef:
    'attribute' 'def' name=ID (';' | '{' (members+=AttributeUsage)* '}');

// Attribute types stay plain qualified names (not cross-references) until the
// standard library types (ScalarValues::Real etc.) exist to resolve against.
AttributeUsage:
    'attribute' name=ID (':' type=QualifiedName)? ('=' value=Literal)? ';';

PortUsage:
    'port' name=ID (':' type=[PortDef:QualifiedName])? ';';

ConnectionUsage:
    ('connection' (name=ID)? (':' type=[InterfaceDef:QualifiedName])?)?
    'connect' source=ConnectorEnd 'to' target=ConnectorEnd ';';

// Dotted feature chain, e.g. `battery.powerOut` or `Physical::fc.port`; the
// first segment resolves lexically (qualified names allowed), later segments
// against the type of the previous segment (SysmlScopeProvider).
ConnectorEnd:
    segments+=[Feature:QualifiedName] ('.' segments+=[Feature:ID])*;

RequirementUsage:
    'requirement' name=ID (':' type=[RequirementDef:QualifiedName])?
    (';' | '{' (members+=RequirementMember)* '}');

// Both ends are usages: a requirement usage, satisfied by a part usage.
SatisfyUsage:
    'satisfy' requirement=[RequirementUsage:QualifiedName] 'by' satisfier=[PartUsage:QualifiedName] ';';

// SysML v2 has no UML-style `extend`; scenarios compose via include/perform.
UseCaseDef:
    'use' 'case' 'def' name=ID
    (';' | '{' (members+=UseCaseMember)* '}');

UseCaseUsage:
    'use' 'case' name=ID (':' type=[UseCaseDef:QualifiedName])?
    (';' | '{' (members+=UseCaseMember)* '}');

UseCaseMember:
    ActorUsage | SubjectUsage | IncludeUsage | AttributeUsage;

ActorUsage:
    'actor' name=ID (':' type=[PartDef:QualifiedName])? ';';

// `include use case x` is spec notation and names a use case usage, never a
// definition.
IncludeUsage:
    'include' ('use' 'case')? target=[UseCaseUsage:QualifiedName] ';';

// What the owning part carries out: an action of the function tree, or a use
// case. The target is a feature chain like any other, so it starts at a usage
// — `perform Functions::survey.communicate.streamVideo` — which is what the
// spec requires (§7.17.6).
PerformUsage:
    'perform' target=ConnectorEnd ';';

// Only the usage form of allocation exists; `allocation def` is not supported.
AllocationUsage:
    ('allocation' (name=ID)?)?
    'allocate' source=ConnectorEnd 'to' target=ConnectorEnd ';';

ActionDef:
    'action' 'def' name=ID (';' | '{' (members+=ActionMember)* '}');

ActionUsage:
    'action' name=ID (':' type=[ActionDef:QualifiedName])?
    (';' | '{' (members+=ActionMember)* '}');

ActionMember:
    MessageUsage | ActionUsage | PartUsage | AttributeUsage;

// Message ordering is document order; the optional leading `then` states the
// succession explicitly, spec-style. The payload names an attribute def.
MessageUsage:
    (then?='then')? 'message' (name=ID)? ('of' payload=[AttributeDef:QualifiedName])?
    'from' source=ConnectorEnd 'to' target=ConnectorEnd ';';

type Feature = PartUsage | PortUsage | AttributeUsage | InterfaceEnd | SubjectUsage | ActionUsage | UseCaseUsage;

QualifiedName returns string:
    ID ('::' ID)*;

Literal returns string:
    NUMBER | STRING | ID;

hidden terminal WS: /\s+/;
terminal ID: /[_a-zA-Z][\w_]*/;
terminal NUMBER: /[0-9]+(\.[0-9]+)?/;
terminal STRING: /"[^"]*"/;
hidden terminal ML_COMMENT: /\/\*[\s\S]*?\*\//;
hidden terminal SL_COMMENT: /\/\/[^\n\r]*/;
