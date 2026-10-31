import { GrammarUtils } from 'langium';
import type { AstNode, ValidationAcceptor, ValidationChecks } from 'langium';
import {
    isActorUsage, isAttributeUsage, isConnectionUsage, isModel, isOtherUsage, isPackageDecl, isPartUsage, isPortUsage,
    isSubjectUsage
} from '../generated/ast.js';
import type { ConnectorEnd, PackageDecl, PerformUsage, SysmlAstType, UseCaseDef, UseCaseUsage } from '../generated/ast.js';
import { LIBRARY } from '../model/library.js';
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
 * that declares actors declares its subject first. One more is only a warning:
 * a package should not take the name of a standard library package.
 */
export function registerValidationChecks(services: SysmlServices): void {
    const checks: ValidationChecks<SysmlAstType> = {
        AstNode: checkNameNotReserved,
        ConnectorEnd: checkEndStartsAtAccessibleFeature,
        PackageDecl: checkPackageNotNamedAfterLibrary,
        PerformUsage: checkPerformTarget,
        UseCaseDef: checkSubjectBeforeActors,
        UseCaseUsage: checkSubjectBeforeActors
    };
    services.validation.ValidationRegistry.register(checks);
}

/** A quoted name may be anything: `'part'` is a name where `part` is a keyword. */
function checkNameNotReserved(node: AstNode, accept: ValidationAcceptor): void {
    for (const property of ['name', 'shortName']) {
        const name = (node as unknown as Record<string, unknown>)[property];
        const written = GrammarUtils.findNodeForProperty(node.$cstNode, property)?.text;
        if (typeof name === 'string' && RESERVED_WORDS.has(name) && !written?.startsWith("'")) {
            accept('error', `'${name}' is a reserved word in SysML v2 and cannot be a name.`, { node, property });
        }
    }
}

/**
 * A top-level package named after a standard library package hides it, and
 * which of the two a reference finds depends on the tool. ortho finds the
 * model's; OMG's pilot implementation finds the library's when the reference is
 * in the same file. So the model renders here and may fail elsewhere, which is
 * worth a warning but no reason to stop.
 */
function checkPackageNotNamedAfterLibrary(pkg: PackageDecl, accept: ValidationAcceptor): void {
    if (isModel(pkg.$container) && !pkg.isLibrary && pkg.name && Object.hasOwn(LIBRARY, pkg.name)) {
        accept('warning',
            `Package '${pkg.name}' has the name of a standard library package and hides it, and other tools may find the library's instead. Rename it.`,
            { node: pkg, property: 'name' });
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
 * function tree, a use case, or other behaviour such as a calculation or a
 * verification case. A part, a port or an attribute is structure, not
 * behaviour. The chain is otherwise the same as any connector end, so it is
 * checked there.
 */
function checkPerformTarget(perform: PerformUsage, accept: ValidationAcceptor): void {
    // A syntax error can leave the target out.
    const target = perform.target?.segments.at(-1)?.ref;
    if (target && isStructure(target)) {
        accept('error', `'${performText(perform)}' is not an action or a use case, so it cannot be performed.`,
            { node: perform, property: 'target' });
    }
}

function isStructure(node: AstNode): boolean {
    return isPartUsage(node) || isPortUsage(node) || isAttributeUsage(node) || isActorUsage(node) || isSubjectUsage(node)
        || isConnectionUsage(node) || (isOtherUsage(node) && ['item', 'occurrence', 'enum'].includes(node.kind));
}

function performText(perform: PerformUsage): string {
    return perform.target?.segments.map(s => s.$refText).join('.') ?? perform.name ?? '';
}
