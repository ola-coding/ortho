grammar Sysml

// Written from the textual notation of the SysML v2 specification (§8.2.2).
// What the six views draw has alternatives of its own, close to the spec's;
// every other definition and usage is an OtherDefinition, OtherUsage or
// DefaultUsage, which read the spec's common shape and keep what they refer to
// as written, since no view follows it.

// A file is a root namespace (§8.2.2.5.1): its packages, and whatever else
// stands outside one.
entry Model:
    (packages+=PackageDecl | imports+=ImportDecl | aliases+=AliasMember | annotations+=AnnotatingElement
        | members+=Element)*;

// ---------------------------------------------------------------- Packages

// A `library` package holds a model library, a `standard library` one a library
// of the spec's own (§8.2.2.5.1).
PackageDecl:
    (visibility=Visibility)? ((isStandard?='standard')? isLibrary?='library')? (prefixMetadata+=PrefixMetadata)*
    'package' Identification Body;

// What an import makes visible is resolved in SysmlScopeProvider: `::*` imports
// a namespace's members, `::**` them and everything nested in them, and `all`
// private members too. The spec requires the visibility to be written out
// (§7.5.3), so it is here too.
ImportDecl:
    visibility=Visibility 'import' (all?='all')?
    target=[Member:QualifiedName] (wildcard?='::*')? (recursive?='::**')? ('[' filters+=Expression ']')*
    RelationshipBody;

// Another name for an element (§8.2.2.5.1).
AliasMember:
    (visibility=Visibility)? 'alias' ('<' shortName=Name '>')? (name=Name)? 'for' target=QualifiedName RelationshipBody;

fragment RelationshipBody:
    ';' | '{' (annotations+=AnnotatingElement)* '}';

// The body of every namespace: its members, imports, aliases, filters and
// comments, in a view what it exposes, and in a calculation or constraint the
// expression that is its result.
fragment Body:
    ';' | '{' (imports+=ImportDecl | aliases+=AliasMember | annotations+=AnnotatingElement | exposes+=Expose
        | filters+=ElementFilter | members+=Member)* (result=Expression)? '}';

ElementFilter:
    (visibility=Visibility)? 'filter' condition=Expression ';';

Member:
    PackageDecl | Element;

// ---------------------------------------------------------------- Comments

// Comments and documentation are model elements (§8.2.2.4). What they are
// `about` is kept as written: no view draws it.
AnnotatingElement:
    Comment | Documentation | TextualRepresentation;

Comment:
    ('comment' Identification ('about' about+=QualifiedName (',' about+=QualifiedName)*)?)?
    ('locale' locale=STRING)? body=REGULAR_COMMENT;

Documentation:
    'doc' Identification ('locale' locale=STRING)? body=REGULAR_COMMENT;

TextualRepresentation:
    ('rep' Identification)? 'language' language=STRING body=REGULAR_COMMENT;

// ---------------------------------------------------------------- Definitions and usages

// Every definition and usage (§8.2.2.6-27). The prefix is read once, and the
// keyword after it decides what follows, so choosing between them looks no
// further than a keyword or two.
Element:
    Prefix
    ( {infer PartDef} 'part' 'def' Identification
        ((':>' | 'specializes') supertypes+=[Definition:QualifiedName] (',' supertypes+=[Definition:QualifiedName])*)?
        Body
    | {infer PortDef} 'port' 'def' Identification
        ((':>' | 'specializes') supertypes+=[Definition:QualifiedName] (',' supertypes+=[Definition:QualifiedName])*)?
        Body
    | {infer InterfaceDef} 'interface' 'def' Identification WrittenSupertypes? Body
    // A definition of values without identity. Its one job here is to type a
    // message's payload, as the spec's own examples do.
    | {infer AttributeDef} 'attribute' 'def' Identification WrittenSupertypes? Body
    | {infer ActionDef} 'action' 'def' Identification WrittenSupertypes? Body
    // SysML v2 has no UML-style `extend`; scenarios compose via include/perform.
    | {infer UseCaseDef} 'use' 'case' 'def' Identification WrittenSupertypes? Body
    // Every other definition, from `item def` to `metadata def`, and an
    // `individual def`.
    | {infer OtherDefinition} (kind=DefinitionKind)? 'def' Identification WrittenSupertypes? (isParallel?='parallel')? Body

    | {infer PartUsage} 'part' Identification
        FeatureRelation* ((':' | 'defined' 'by') type=[Definition:QualifiedName] MoreTypes FeatureRelation*)?
        ValuePart? Body
    // Attribute types stay plain qualified names (not cross-references) until
    // the standard library types (ScalarValues::Real etc.) exist to resolve
    // against.
    | {infer AttributeUsage} 'attribute' Identification
        FeatureRelation* ((':' | 'defined' 'by') type=QualifiedName MoreTypes FeatureRelation*)?
        ValuePart? Body
    | {infer PortUsage} 'port' Identification
        FeatureRelation*
        ((':' | 'defined' 'by') (conjugated?='~')? type=[PortDef:QualifiedName] MoreTypes FeatureRelation*)?
        ValuePart? Body
    | {infer ActionUsage} 'action' Identification
        FeatureRelation* ((':' | 'defined' 'by') type=[ActionDef:QualifiedName] MoreTypes FeatureRelation*)?
        ValuePart? Body
    | {infer UseCaseUsage} 'use' 'case' Identification
        FeatureRelation* ((':' | 'defined' 'by') type=[UseCaseDef:QualifiedName] MoreTypes FeatureRelation*)?
        ValuePart? Body
    | {infer ActorUsage} 'actor' Identification
        FeatureRelation* ((':' | 'defined' 'by') type=[Definition:QualifiedName] MoreTypes FeatureRelation*)?
        ValuePart? Body
    | {infer SubjectUsage} 'subject' Identification
        FeatureRelation* ((':' | 'defined' 'by') type=[Definition:QualifiedName] MoreTypes FeatureRelation*)?
        ValuePart? Body
    // `include x` names a use case usage, never a definition; `include use
    // case x` declares one, which the use case view draws as the use case of
    // that name.
    | {infer IncludeUsage} 'include'
        ( useCase?='use' 'case' Identification
          FeatureRelation* ((':' | 'defined' 'by') type=[UseCaseDef:QualifiedName] MoreTypes FeatureRelation*)?
        | target=ConnectorEnd (WrittenTyping | FeatureRelation)* )
        ValuePart? Body
    // What the owning part carries out: an action of the function tree, or a
    // use case. The target is a feature chain like any other, so it starts at
    // a usage — `perform Functions::survey.communicate.streamVideo` — which is
    // what the spec requires (§7.17.6). `perform action x` performs an action
    // of its own.
    | {infer PerformUsage} 'perform'
        ( 'action' Identification (WrittenTyping | FeatureRelation)*
        | target=ConnectorEnd FeatureRelation* )
        ValuePart? Body
    // A connection, or an interface usage, between two features (§8.2.2.13-14).
    | {infer ConnectionUsage}
        ( kind=('connection' | 'interface') Identification
          FeatureRelation* ((':' | 'defined' 'by') type=[Definition:QualifiedName] MoreTypes FeatureRelation*)?
          ValuePart? ('connect' ConnectorPart)?
        | kind='interface' ConnectorPart
        | 'connect' ConnectorPart )
        Body
    | {infer AllocationUsage}
        ( 'allocation' Identification (WrittenTyping | FeatureRelation)* ('allocate' ConnectorPart)?
        | 'allocate' ConnectorPart )
        Body
    // Message ordering is document order; the optional leading `then` states
    // the succession explicitly, spec-style. The payload names a definition.
    | {infer MessageUsage} 'message'
        ( Identification (WrittenTyping | FeatureRelation)* ValuePart?
          ('of' (payloadName=Name ':')? payload=[Definition:QualifiedName] (payloadMultiplicity=Multiplicity)?
            ('=' payloadValue=Expression)?)?
          ('from' source=ConnectorEnd 'to' target=ConnectorEnd)?
        | source=ConnectorEnd 'to' target=ConnectorEnd )
        Body
    // A view is the diagram (§7.26.5): its name is the heading, its definition
    // one of the standard views, matched by name since the standard library is
    // not loaded, and what it exposes is the content.
    | {infer ViewUsage} 'view' Identification
        FeatureRelation* ((':' | 'defined' 'by') definition=QualifiedName MoreTypes FeatureRelation*)?
        ValuePart? Body

    | {infer SatisfyUsage} ('assert')? (negated?='not')? 'satisfy'
        ('requirement' Identification (WrittenTyping | FeatureRelation)* | requirement=FeatureChain FeatureRelation*)
        ValuePart? ('by' satisfier=FeatureChain)? Body
    | {infer BindingUsage} ('binding' Identification (WrittenTyping | FeatureRelation)*)?
        'bind' left=FeatureChain '=' right=FeatureChain Body
    | {infer FlowUsage} (succession?='succession')? 'flow'
        ( Identification (WrittenTyping | FeatureRelation)* ValuePart?
          ('of' (payloadName=Name ':')? payload=TypeName (payloadMultiplicity=Multiplicity)?
            ('=' payloadValue=Expression)?)?
          ('from' source=FeatureChain 'to' target=FeatureChain)?
        | source=FeatureChain 'to' target=FeatureChain )
        Body
    // `@Safety;` or `metadata m : Safety about engine;` (§8.2.2.27).
    | {infer MetadataUsage} ('@' | 'metadata') (Identification (':' | 'typed' 'by'))? type=QualifiedName
        ('about' about+=QualifiedName (',' about+=QualifiedName)*)? Body
    | {infer Dependency} 'dependency' (Identification 'from')?
        clients+=[Member:QualifiedName] (',' clients+=[Member:QualifiedName])*
        'to' suppliers+=[Member:QualifiedName] (',' suppliers+=[Member:QualifiedName])*
        RelationshipBody
    // Every other usage with a keyword of its own.
    | {infer OtherUsage}
        ( kind=UsageKind
        | kind='event' 'occurrence'?
        | kind='exhibit' 'state'?
        | kind='assert' 'not'? 'constraint'?
        | kind='render' 'rendering'?
        | kind='frame' 'concern'?
        | kind='verify' 'requirement'?
        | kind=('require' | 'assume') (prefixMetadata+=PrefixMetadata)* 'constraint'?
        | kind=('objective' | 'stakeholder')
        | kind=('merge' | 'decide' | 'join' | 'fork') )
        // A chain names a feature that already exists: `event driver.started`.
        (Identification | reference=ChainedName) (WrittenTyping | FeatureRelation)* ValuePart?
        (isParallel?='parallel')? Body
    // Behaviour (§8.2.2.17-18). No view draws it, so what it names is kept as
    // written. A succession, `first a then b;`, or after an action, `if g then b;`.
    | {infer SuccessionUsage} ('succession' Identification (WrittenTyping | FeatureRelation)*)?
        'first' first=SuccessionEnd (('if' guard=Expression)? 'then' next=SuccessionEnd)? Body
    | {infer SuccessionUsage} ('if' guard=Expression 'then' | isElse?='else') next=SuccessionEnd Body
    | {infer TransitionUsage} 'transition' (Identification 'first')? first=SuccessionEnd
        ('accept' AcceptPart)? ('if' guard=Expression)? ('do' effect=Effect)? 'then' next=SuccessionEnd Body
    | {infer StateAction} kind=('entry' | 'do' | 'exit') (';' | action=Element)
    | {infer AcceptAction} ('action' Identification)? 'accept' AcceptPart
        (('if' guard=Expression)? ('do' effect=Effect)? 'then' next=SuccessionEnd)? Body
    | {infer SendAction} ('action' Identification)? SendPart Body
    | {infer AssignAction} ('action' Identification)? AssignPart Body
    | {infer TerminateAction} ('action' Identification)? 'terminate' (target=Expression)? Body
    | {infer IfAction} ('action' Identification)? IfPart
    | {infer LoopAction} ('action' Identification)? ('while' condition=Expression | 'loop') loopBody=ActionBlock
        ('until' until=Expression ';')?
    | {infer ForAction} ('action' Identification)? 'for' variable=Name WrittenTyping? 'in' collection=Expression
        loopBody=ActionBlock
    // A usage without a keyword: a parameter, `in x : T;`, a redefinition,
    // `:>> mass = 1500;`, an enumerated value, `red;`, or a reference, `ref x;`.
    | {infer DefaultUsage}
        ( ('<' shortName=Name '>')? name=Name (WrittenTyping | FeatureRelation)* ValuePart?
        | (WrittenTyping | FeatureRelation)+ ValuePart?
        // An enumerated value may be its value alone, `= 60.0;`.
        | ValuePart )
        Body
    );

Expose:
    (visibility=Visibility)? 'expose' element=[Member:QualifiedName] (wildcard?='::*')? (recursive?='::**')?
    ('[' filters+=Expression ']')* RelationshipBody;

SuccessionEnd:
    (multiplicity=Multiplicity)? chain=FeatureChain;

// What an `accept` receives: a payload, a time or a condition, through a port.
fragment AcceptPart:
    (payloadName=Name)? (WrittenTyping | FeatureRelation)* (trigger=('at' | 'after' | 'when') triggerValue=Expression)?
    ('via' via=Expression)?;

fragment SendPart:
    'send' (payload=Expression)? ('via' via=Expression)? ('to' to=Expression)?;

fragment AssignPart:
    'assign' target=Expression ':=' assigned=Expression;

fragment IfPart:
    'if' condition=Expression thenBlock=ActionBlock ('else' (elseBlock=ActionBlock | elseIf=ElseIf))?;

ElseIf:
    IfPart;

ActionBlock:
    ('action' Identification (WrittenTyping | FeatureRelation)*)? Body;

// A transition's effect, `do send new Sig() to p` or `do action a`.
Effect:
    ('action' Identification (WrittenTyping | FeatureRelation)* EffectPart? | EffectPart)
    ('{' (members+=Member | annotations+=AnnotatingElement)* '}')?;

fragment EffectPart:
    SendPart | AssignPart | 'accept' AcceptPart | 'perform' performed=FeatureChain;

fragment ConnectorPart:
    source=ConnectorEnd 'to' target=ConnectorEnd
    | '(' ends+=ConnectorEnd ',' ends+=ConnectorEnd (',' ends+=ConnectorEnd)* ')';

// Dotted feature chain, e.g. `battery.powerOut` or `Physical::fc.port`; the
// first segment resolves lexically (qualified names allowed), later segments
// against the type of the previous segment (SysmlScopeProvider).
ConnectorEnd:
    (multiplicity=Multiplicity)? (name=Name ('::>' | 'references'))?
    segments+=[Feature:QualifiedName] ('.' segments+=[Feature:Name])*;

DefinitionKind returns string:
    'item' | 'connection' | 'flow' | 'allocation' | 'occurrence' | 'calc' | 'constraint' | 'state' | 'requirement'
    | 'concern' | 'case' | 'analysis' | 'verification' | 'view' | 'viewpoint' | 'rendering' | 'metadata' | 'enum';

UsageKind returns string:
    'item' | 'occurrence' | 'enum' | 'calc' | 'constraint' | 'state' | 'requirement' | 'concern' | 'case'
    | 'analysis' | 'verification' | 'viewpoint' | 'rendering';

// ---------------------------------------------------------------- Parts of a declaration

// A name, and optionally a short one, `<'1.1'> 'Brew unit'`; either may be
// left out (§8.2.2.2).
fragment Identification:
    ('<' shortName=Name '>')? (name=Name)?;

// The prefixes of definitions and usages (§8.2.2.6), read in any combination.
// A usage after `then` follows the one before it (§8.2.2.9.3).
fragment Prefix:
    (visibility=Visibility)? (isVariant?='variant')? (then?='then')? (isReturn?='return')?
    (isEnd?='end' (crossName=Name? endMultiplicity=Multiplicity ('ordered' | 'nonunique')*)?)? (direction=Direction)?
    (isDerived?='derived')? (isAbstract?='abstract' | isVariation?='variation')? (isConstant?='constant')?
    (isReference?='ref')? (isIndividual?='individual')? (portionKind=PortionKind)? (prefixMetadata+=PrefixMetadata)*;

// A metadata annotation used as a keyword, `#Safety` (§8.2.2.27).
PrefixMetadata returns string:
    '#' QualifiedName;

Visibility returns string:
    'private' | 'protected' | 'public';

// A direction is what makes a port's feature an API rather than a value: it
// says which way the thing crosses (§7.12.1).
Direction returns string:
    'in' | 'out' | 'inout';

PortionKind returns string:
    'snapshot' | 'timeslice';

// Subsetting, redefinition, reference, crossing and multiplicity (§8.2.2.6.5-6),
// in any order, kept as written.
fragment FeatureRelation:
      (':>' | 'subsets') subsets+=FeatureChain (',' subsets+=FeatureChain)*
    | (':>>' | 'redefines') redefines+=FeatureChain (',' redefines+=FeatureChain)*
    | ('::>' | 'references') references+=FeatureChain
    | ('=>' | 'crosses') crosses+=FeatureChain
    | multiplicities+=Multiplicity
    | 'ordered'
    | 'nonunique';

// Further typings beyond the first, kept as written.
fragment MoreTypes:
    (',' types+=TypeName)*;

fragment WrittenTyping:
    (':' | 'defined' 'by') types+=TypeName MoreTypes;

fragment WrittenSupertypes:
    (':>' | 'specializes') supertypes+=QualifiedName (',' supertypes+=QualifiedName)*;

fragment ValuePart:
    ('=' | isInitial?=':=' | isDefault?='default' ('=' | isInitial?=':=')?) value=Expression;

Multiplicity:
    '[' lower=MultiplicityBound ('..' upper=MultiplicityBound)? ']';

MultiplicityBound returns string:
    NUMBER | '*' | QualifiedName;

// ---------------------------------------------------------------- Names

type Feature = PartUsage | PortUsage | AttributeUsage | SubjectUsage | ActorUsage | ActionUsage | UseCaseUsage
    | PerformUsage | OtherUsage | DefaultUsage | ConnectionUsage | AllocationUsage | MessageUsage | FlowUsage
    | BindingUsage | SatisfyUsage | IncludeUsage | MetadataUsage | ViewUsage | SuccessionUsage | TransitionUsage
    | ConnectorEnd
    | AcceptAction | SendAction | AssignAction | TerminateAction | IfAction | LoopAction | ForAction;

type Definition = PartDef | PortDef | InterfaceDef | AttributeDef | ActionDef | UseCaseDef | OtherDefinition;

// A quoted name is the same name without its quotes; SysmlValueConverter
// removes them (§8.2.2.1.2).
Name returns string:
    ID | UNRESTRICTED_NAME;

QualifiedName returns string:
    Name ('::' Name)*;

FeatureChain returns string:
    QualifiedName ('.' Name)*;

// A qualified name or feature chain of at least two names.
ChainedName returns string:
    Name (('::' | '.') Name)+;

TypeName returns string:
    '~'? QualifiedName;

// ---------------------------------------------------------------- Expressions

// Expressions (KerML §8.2.5.8), read so that a model using them loads: values
// with units, `1500 [kg]`, constraints, filters. No view evaluates them.
Expression:
    {infer IfExpression} 'if' condition=Expression '?' then=Expression 'else' else=Expression
    | OperatorExpression;

// The operators by precedence, tightest first (KerML Table 6).
infix OperatorExpression on UnaryExpression:
    right assoc '^' | '**'
    > '*' | '/' | '%'
    > '+' | '-'
    > '..'
    > '<' | '>' | '<=' | '>='
    > 'istype' | 'hastype' | '@' | '@@' | 'as' | 'meta'
    > '==' | '!=' | '===' | '!=='
    > '&'
    > 'xor'
    > '|'
    > 'and'
    > 'or'
    > 'implies'
    > '??';

UnaryExpression:
    {infer PrefixExpression} operator=('+' | '-' | '~' | 'not') operand=UnaryExpression
    // `@Safety` in a filter tests the element the filter is applied to.
    | {infer ClassificationExpression} operator=('istype' | 'hastype' | '@' | '@@' | 'as' | 'meta') type=QualifiedName
    | {infer ExtentExpression} 'all' type=QualifiedName
    | PrimaryExpression;

PrimaryExpression:
    BaseExpression
    ( {infer FeatureChainExpression.source=current} '.' member=Name (invocation?='(' Arguments? ')')?
    | {infer MetadataAccessExpression.source=current} '.' 'metadata'
    | {infer CollectExpression.source=current} '.' body=BodyExpression
    | {infer SelectExpression.source=current} '.?' body=BodyExpression
    | {infer InvocationOperation.source=current} '->' function=QualifiedName
        (body=BodyExpression | reference=QualifiedName | ArgumentList)
    | {infer BracketExpression.source=current} '[' (index+=Expression (',' index+=Expression)*)? ']'
    | {infer IndexExpression.source=current} '#' '(' index+=Expression (',' index+=Expression)* ')'
    )*;

BaseExpression:
    {infer LiteralExpression} value=('true' | 'false' | 'null' | STRING | NUMBER | Fraction | '*')
    | {infer ConstructorExpression} 'new' type=QualifiedName ArgumentList
    | {infer FeatureReferenceExpression} name=QualifiedName (invocation?='(' Arguments? ')')?
    | {infer SequenceExpression} '(' (items+=Expression (',' items+=Expression)* ','?)? ')';

BodyExpression:
    '{' (members+=Member)* (result=Expression)? '}';

// A real number without its leading zero, `.6`.
Fraction returns string:
    '.' NUMBER;

fragment ArgumentList:
    '(' Arguments? ')';

fragment Arguments:
    arguments+=Argument (',' arguments+=Argument)*;

Argument:
    (name=Name '=')? value=Expression;

// `/* ... */` is a comment element; notes, `// ...` and `//* ... */`, are not
// part of the model (§8.2.2.1.2).
hidden terminal WS: /\s+/;
hidden terminal ML_NOTE: /\/\/\*[\s\S]*?\*\//;
hidden terminal SL_NOTE: /\/\/[^\n\r]*/;
terminal REGULAR_COMMENT: /\/\*[\s\S]*?\*\//;
terminal ID: /[_a-zA-Z][\w_]*/;
terminal NUMBER: /[0-9]+(\.[0-9]+)?([eE][+-]?[0-9]+)?/;
terminal STRING: /"(\\.|[^"\\\r\n])*"/;
terminal UNRESTRICTED_NAME: /'(\\.|[^'\\\r\n])*'/;
