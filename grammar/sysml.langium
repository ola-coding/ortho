grammar Sysml

// A file is a root namespace (§8.2.2.5.1): its packages, with any imports and
// comments that stand outside them.
entry Model:
    (packages+=PackageDecl | imports+=ImportDecl | annotations+=AnnotatingElement)*;

// A `library` package holds a model library, a `standard library` one a library
// of the spec's own (§8.2.2.5.1).
PackageDecl:
    ((isStandard?='standard')? isLibrary?='library')? 'package' Identification
    (';' | '{'
        (imports+=ImportDecl | aliases+=AliasMember | annotations+=AnnotatingElement | members+=PackageMember)*
    '}');

// What an import makes visible is resolved in SysmlScopeProvider: `::*` imports
// a namespace's members, `::**` them and everything nested in them, and `all`
// private members too. The spec requires the visibility to be written out
// (§7.5.3), so it is here too.
ImportDecl:
    visibility=Visibility 'import' (all?='all')?
    target=[Element:QualifiedName] (wildcard?='::*')? (recursive?='::**')? RelationshipBody;

// Another name for an element (§8.2.2.5.1).
AliasMember:
    'alias' ('<' shortName=Name '>')? (name=Name)? 'for' target=QualifiedName RelationshipBody;

fragment RelationshipBody:
    ';' | '{' (annotations+=AnnotatingElement)* '}';

// Comments and documentation are model elements (§8.2.2.4). What they are
// `about` is kept as written: no view draws it.
AnnotatingElement:
    Comment | Documentation | TextualRepresentation;

Comment:
    ('comment' OptionalIdentification ('about' about+=QualifiedName (',' about+=QualifiedName)*)?)?
    ('locale' locale=STRING)? body=REGULAR_COMMENT;

Documentation:
    'doc' OptionalIdentification ('locale' locale=STRING)? body=REGULAR_COMMENT;

TextualRepresentation:
    ('rep' OptionalIdentification)? 'language' language=STRING body=REGULAR_COMMENT;

// A name, and optionally a short one: `<'1.1'> 'Brew unit'` (§8.2.2.2).
fragment Identification:
    ('<' shortName=Name '>')? name=Name;

fragment OptionalIdentification:
    ('<' shortName=Name '>')? (name=Name)?;

Visibility returns string:
    'private' | 'protected' | 'public';

PackageMember:
    PackageDecl | PartDef | PortDef | InterfaceDef | AttributeDef | RequirementDef | UseCaseDef | ActionDef
    | PartUsage | RequirementUsage | ConnectionUsage | SatisfyUsage | UseCaseUsage | AllocationUsage | ActionUsage
    | ViewUsage;

PartDef:
    'part' 'def' Identification
    ((':>' | 'specializes') supertypes+=[PartDef:QualifiedName] (',' supertypes+=[PartDef:QualifiedName])*)?
    (';' | '{' (members+=UsageMember | annotations+=AnnotatingElement)* '}');

UsageMember:
    AttributeUsage | PortUsage | PartUsage | ConnectionUsage | SatisfyUsage | RequirementUsage
    | PerformUsage | AllocationUsage | ActionUsage;

PortDef:
    'port' 'def' Identification
    ((':>' | 'specializes') supertypes+=[PortDef:QualifiedName] (',' supertypes+=[PortDef:QualifiedName])*)?
    (';' | '{' (members+=AttributeUsage | annotations+=AnnotatingElement)* '}');

InterfaceDef:
    'interface' 'def' Identification
    (';' | '{' (ends+=InterfaceEnd | members+=AttributeUsage | annotations+=AnnotatingElement)* '}');

InterfaceEnd:
    'end' Identification ':' (conjugated?='~')? type=[PortDef:QualifiedName] ';';

RequirementDef:
    'requirement' 'def' Identification
    (';' | '{' (members+=RequirementMember | annotations+=AnnotatingElement)* '}');

RequirementMember:
    AttributeUsage | SubjectUsage;

SubjectUsage:
    'subject' Identification (':' type=[PartDef:QualifiedName])? ';';

PartUsage:
    'part' Identification (':' type=[PartDef:QualifiedName])? (multiplicity=Multiplicity)?
    (';' | '{' (members+=UsageMember | annotations+=AnnotatingElement)* '}');

Multiplicity:
    '[' lower=NUMBER ('..' upper=UpperBound)? ']';

UpperBound returns string:
    NUMBER | '*';

// A definition of values without identity. Its one job here is to type a
// message's payload, as the spec's own examples do.
AttributeDef:
    'attribute' 'def' Identification (';' | '{' (members+=AttributeUsage | annotations+=AnnotatingElement)* '}');

// Attribute types stay plain qualified names (not cross-references) until the
// standard library types (ScalarValues::Real etc.) exist to resolve against.
// A direction is what makes a port's feature an API rather than a value: it
// says which way the thing crosses (§7.12.1).
AttributeUsage:
    (direction=Direction)? 'attribute' Identification (':' type=QualifiedName)? ('=' value=Literal)? ';';

Direction returns string:
    'in' | 'out' | 'inout';

PortUsage:
    'port' Identification (':' (conjugated?='~')? type=[PortDef:QualifiedName])? ';';

ConnectionUsage:
    ('connection' OptionalIdentification (':' type=[InterfaceDef:QualifiedName])?)?
    'connect' source=ConnectorEnd 'to' target=ConnectorEnd ';';

// Dotted feature chain, e.g. `battery.powerOut` or `Physical::fc.port`; the
// first segment resolves lexically (qualified names allowed), later segments
// against the type of the previous segment (SysmlScopeProvider).
ConnectorEnd:
    segments+=[Feature:QualifiedName] ('.' segments+=[Feature:Name])*;

RequirementUsage:
    'requirement' Identification (':' type=[RequirementDef:QualifiedName])?
    (';' | '{' (members+=RequirementMember | annotations+=AnnotatingElement)* '}');

// Both ends are usages: a requirement usage, satisfied by a part usage.
SatisfyUsage:
    'satisfy' requirement=[RequirementUsage:QualifiedName] 'by' satisfier=[PartUsage:QualifiedName] ';';

// SysML v2 has no UML-style `extend`; scenarios compose via include/perform.
UseCaseDef:
    'use' 'case' 'def' Identification
    (';' | '{' (members+=UseCaseMember | annotations+=AnnotatingElement)* '}');

UseCaseUsage:
    'use' 'case' Identification (':' type=[UseCaseDef:QualifiedName])?
    (';' | '{' (members+=UseCaseMember | annotations+=AnnotatingElement)* '}');

UseCaseMember:
    ActorUsage | SubjectUsage | IncludeUsage | AttributeUsage;

ActorUsage:
    'actor' Identification (':' type=[PartDef:QualifiedName])? ';';

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
    ('allocation' OptionalIdentification)?
    'allocate' source=ConnectorEnd 'to' target=ConnectorEnd ';';

ActionDef:
    'action' 'def' Identification (';' | '{' (members+=ActionMember | annotations+=AnnotatingElement)* '}');

ActionUsage:
    'action' Identification (':' type=[ActionDef:QualifiedName])?
    (';' | '{' (members+=ActionMember | annotations+=AnnotatingElement)* '}');

ActionMember:
    MessageUsage | ActionUsage | PartUsage | AttributeUsage;

// Message ordering is document order; the optional leading `then` states the
// succession explicitly, spec-style. The payload names an attribute def.
MessageUsage:
    (then?='then')? 'message' OptionalIdentification ('of' payload=[AttributeDef:QualifiedName])?
    'from' source=ConnectorEnd 'to' target=ConnectorEnd ';';

// A view is the diagram (§7.26.5): its name is the heading, its definition one
// of the standard views, matched by name since the standard library is not
// loaded, and what it exposes is the content.
ViewUsage:
    'view' Identification ':' definition=QualifiedName
    '{' (exposes+=Expose | annotations+=AnnotatingElement)* '}';

Expose:
    'expose' element=[Exposable:QualifiedName] ';';

type Exposable = PackageDecl | ActionDef | PartUsage;

type Feature = PartUsage | PortUsage | AttributeUsage | InterfaceEnd | SubjectUsage | ActionUsage | UseCaseUsage;

// What an import may name: a namespace, or one element in it.
type Element = PackageDecl | PartDef | PortDef | InterfaceDef | AttributeDef | RequirementDef | UseCaseDef | ActionDef
    | PartUsage | PortUsage | AttributeUsage | RequirementUsage | UseCaseUsage | ActionUsage | ViewUsage;

// A quoted name is the same name without its quotes; SysmlValueConverter
// removes them (§8.2.2.1.2).
Name returns string:
    ID | UNRESTRICTED_NAME;

QualifiedName returns string:
    Name ('::' Name)*;

Literal returns string:
    NUMBER | STRING | ID;

// `/* ... */` is a comment element; notes, `// ...` and `//* ... */`, are not
// part of the model (§8.2.2.1.2).
hidden terminal WS: /\s+/;
hidden terminal ML_NOTE: /\/\/\*[\s\S]*?\*\//;
hidden terminal SL_NOTE: /\/\/[^\n\r]*/;
terminal REGULAR_COMMENT: /\/\*[\s\S]*?\*\//;
terminal ID: /[_a-zA-Z][\w_]*/;
terminal NUMBER: /[0-9]+(\.[0-9]+)?/;
terminal STRING: /"(\\.|[^"\\\r\n])*"/;
terminal UNRESTRICTED_NAME: /'(\\.|[^'\\\r\n])*'/;
