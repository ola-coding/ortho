grammar Sysml

entry Model:
    (packages+=PackageDecl)*;

PackageDecl:
    'package' name=ID '{'
        (imports+=ImportDecl | members+=PackageMember)*
    '}';

// Imports parse but are not yet semantically enforced: all definitions are
// visible by qualified name across packages (see SysmlScopeComputation).
ImportDecl:
    'import' path=QualifiedName (wildcard?='::*')? ';';

PackageMember:
    PackageDecl | PartDef | PortDef | InterfaceDef | RequirementDef | UseCaseDef | ActionDef
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

SatisfyUsage:
    'satisfy' requirement=[RequirementElement:QualifiedName] 'by' satisfier=[SatisfyTarget:QualifiedName] ';';

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

IncludeUsage:
    'include' ('use' 'case')? target=[UseCaseElement:QualifiedName] ';';

PerformUsage:
    'perform' ('use' 'case')? target=[UseCaseElement:QualifiedName] ';';

// Allocation defs are out of scope until Phase 7; only the usage form exists.
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
// succession explicitly, spec-style. Payload stays a plain qualified name
// until item defs exist.
MessageUsage:
    (then?='then')? 'message' (name=ID)? ('of' payload=QualifiedName)?
    'from' source=ConnectorEnd 'to' target=ConnectorEnd ';';

type Feature = PartUsage | PortUsage | AttributeUsage | InterfaceEnd | SubjectUsage;
type RequirementElement = RequirementDef | RequirementUsage;
type SatisfyTarget = PartDef | PartUsage;
type UseCaseElement = UseCaseDef | UseCaseUsage;

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
