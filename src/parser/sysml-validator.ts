import type { AstNode, ValidationAcceptor, ValidationChecks } from 'langium';
import {
    isActionUsage, isActorUsage, isPackageDecl, isSubjectUsage, isUseCaseUsage
} from '../generated/ast.js';
import type { ConnectorEnd, PerformUsage, SysmlAstType, UseCaseDef, UseCaseUsage } from '../generated/ast.js';
import type { SysmlServices } from './sysml-module.js';

/**
 * The reserved keywords of SysML v2 (formal/2026-03-02, 8.2.2.1.2). ortho's
 * own grammar reserves only the ones it parses, so the rest would otherwise
 * be accepted as names, and a model using one would load here but fail in
 * any other SysML v2 tool.
 */
export const RESERVED_WORDS: ReadonlySet<string> = new Set([
    'about', 'abstract', 'accept', 'action', 'actor', 'after', 'alias', 'all', 'allocate',
    'allocation', 'analysis', 'and', 'as', 'assert', 'assign', 'assume', 'at', 'attribute', 'bind',
    'binding', 'by', 'calc', 'case', 'comment', 'concern', 'connect', 'connection', 'constant',
    'constraint', 'crosses', 'decide', 'def', 'default', 'defined', 'dependency', 'derived', 'do',
    'doc', 'else', 'end', 'entry', 'enum', 'event', 'exhibit', 'exit', 'expose', 'false', 'filter',
    'first', 'flow', 'for', 'fork', 'frame', 'from', 'hastype', 'if', 'implies', 'import', 'in',
    'include', 'individual', 'inout', 'interface', 'istype', 'item', 'join', 'language', 'library',
    'locale', 'loop', 'merge', 'message', 'meta', 'metadata', 'nonunique', 'not', 'null',
    'objective', 'occurrence', 'of', 'or', 'ordered', 'out', 'package', 'parallel', 'part',
    'perform', 'port', 'private', 'protected', 'public', 'redefines', 'ref', 'references', 'render',
    'rendering', 'rep', 'require', 'requirement', 'return', 'satisfy', 'send', 'snapshot',
    'specializes', 'stakeholder', 'standard', 'state', 'subject', 'subsets', 'succession',
    'terminate', 'then', 'timeslice', 'to', 'transition', 'true', 'until', 'use', 'variant',
    'variation', 'verification', 'verify', 'via', 'view', 'viewpoint', 'when', 'while', 'xor'
]);

/**
 * Checks that keep a model ortho accepts a valid SysML v2 model, where the
 * grammar alone cannot: a name must not be a reserved word, a connector end
 * may not name a feature through its owner by qualified name, and a use case
 * that declares actors declares its subject first.
 */
export function registerValidationChecks(services: SysmlServices): void {
    const checks: ValidationChecks<SysmlAstType> = {
        AstNode: checkNameNotReserved,
        ConnectorEnd: checkEndStartsAtAccessibleFeature,
        PerformUsage: checkPerformTarget,
        UseCaseDef: checkSubjectBeforeActors,
        UseCaseUsage: checkSubjectBeforeActors
    };
    services.validation.ValidationRegistry.register(checks);
}

function checkNameNotReserved(node: AstNode, accept: ValidationAcceptor): void {
    const name = (node as { name?: unknown }).name;
    if (typeof name === 'string' && RESERVED_WORDS.has(name)) {
        accept('error', `'${name}' is a reserved word in SysML v2 and cannot be a name.`, { node, property: 'name' });
    }
}

/**
 * A qualified name may walk through packages only. `Hardware::CoffeeMachine::controller`
 * names a feature through the definition that owns it, which SysML v2 rejects:
 * the chain has to start at a usage and continue with dots, as in
 * `Hardware::machine.controller`.
 */
function checkEndStartsAtAccessibleFeature(end: ConnectorEnd, accept: ValidationAcceptor): void {
    const first = end.segments[0];
    const target = first?.ref;
    if (target && first.$refText.includes('::') && !isPackageDecl(target.$container)) {
        accept('error',
            `'${first.$refText}' names a feature through its owner. Start at a usage and continue with dots.`,
            { node: end, property: 'segments', index: 0 });
    }
}

/**
 * The subject is a use case's first parameter and its actors follow, so a use
 * case that declares an actor has to declare its subject before it, even when
 * its definition already has one. A secondary actor, added on a single use
 * case, therefore comes with that use case's own `subject` line.
 */
function checkSubjectBeforeActors(useCase: UseCaseDef | UseCaseUsage, accept: ValidationAcceptor): void {
    const first = useCase.members.find(m => isSubjectUsage(m) || isActorUsage(m));
    if (first && isActorUsage(first)) {
        accept('error',
            `Declare the subject of '${useCase.name}' before its actors: the subject is a use case's first parameter.`,
            { node: first });
    }
}

/**
 * `perform` names something its owner can carry out: an action of the
 * function tree, or a use case. A port or an attribute is neither, and the
 * chain is otherwise the same as any connector end, so it is checked there.
 */
function checkPerformTarget(perform: PerformUsage, accept: ValidationAcceptor): void {
    const target = perform.target.segments[perform.target.segments.length - 1]?.ref;
    if (target && !isActionUsage(target) && !isUseCaseUsage(target)) {
        accept('error', `'${performText(perform)}' is not an action or a use case, so it cannot be performed.`,
            { node: perform, property: 'target' });
    }
}

function performText(perform: PerformUsage): string {
    return perform.target.segments.map(s => s.$refText).join('.');
}
