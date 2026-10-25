import { AstUtils } from 'langium';
import type { AstNode } from 'langium';
import {
    isActionDef, isAllocationUsage, isPackageDecl, isPartUsage, isUseCaseDef, isUseCaseUsage, isViewUsage
} from '../generated/ast.js';
import type { Member, ViewUsage } from '../generated/ast.js';

/** The standard view definitions ortho draws (§9.2.20), and what each may expose. */
export const STANDARD_VIEWS = {
    GeneralView: (e: Member) => isPackageDecl(e) || isActionDef(e),
    InterconnectionView: (e: Member) => isPartUsage(e),
    SequenceView: (e: Member) => isActionDef(e)
};

export type StandardView = keyof typeof STANDARD_VIEWS;

export type DiagramKind = 'use-case' | 'logical' | 'implementation' | 'physical' | 'deployment' | 'process';

export function standardView(view: ViewUsage): StandardView | undefined {
    const name = view.definition?.split('::').pop();
    return name && name in STANDARD_VIEWS ? name as StandardView : undefined;
}

const EXPOSES: Record<StandardView, string> = {
    GeneralView: 'packages or action definitions',
    InterconnectionView: 'parts',
    SequenceView: 'an action definition'
};

/**
 * Why ortho cannot draw a view, or nothing when it can. A model may declare
 * any view; only rendering one other than these fails.
 */
export function whyUndrawable(view: ViewUsage): string | undefined {
    const definition = standardView(view);
    if (!definition) {
        return `ortho draws a GeneralView, an InterconnectionView or a SequenceView, not '${view.definition ?? ''}'.`;
    }
    const article = definition.startsWith('I') ? 'An' : 'A';
    return exposedElements(view).some(e => !STANDARD_VIEWS[definition](e))
        ? `${article} ${definition} exposes ${EXPOSES[definition]}.`
        : undefined;
}

export function exposedElements(view: ViewUsage): Member[] {
    return view.exposes.map(e => e.element.ref).filter((e): e is Member => e !== undefined);
}

/** The views a file declares. */
export function declaredViews(root: AstNode): ViewUsage[] {
    return AstUtils.streamAllContents(root).filter(isViewUsage).toArray();
}

/** A diagram's heading is its view's name (§7.26.5), or for a view without one its definition's. */
export function viewHeading(view: ViewUsage): string {
    return view.name ?? view.definition ?? '';
}

/**
 * The file a view is rendered to: its short name, or else its name, in lower
 * case with hyphens. `view 'Logical view — Coffee Machine'` is written to
 * logical-view-coffee-machine.svg, `view <logical> 'Logical view'` to
 * logical.svg.
 */
export function viewFileName(view: ViewUsage): string {
    const slug = (view.shortName ?? viewHeading(view))
        .normalize('NFKD').replace(/\p{M}/gu, '')
        .toLowerCase().replace(/[^a-z0-9]+/g, '-').replace(/^-+|-+$/g, '');
    return `${slug || 'view'}.svg`;
}

/**
 * Which of ortho's six diagrams a view is. An InterconnectionView is the
 * physical view and a SequenceView the process view. A GeneralView presents
 * whatever it exposes, so its content decides: a function tree, use cases,
 * allocations, or else packages of software.
 */
export function diagramKind(view: ViewUsage): DiagramKind {
    const definition = standardView(view);
    if (definition === 'InterconnectionView') {
        return 'physical';
    }
    if (definition === 'SequenceView') {
        return 'process';
    }
    const exposed = exposedElements(view);
    if (exposed.some(isActionDef)) {
        return 'logical';
    }
    const contents = exposed.flatMap(e => [...AstUtils.streamAllContents(e)]);
    if (contents.some(n => isUseCaseDef(n) || isUseCaseUsage(n))) {
        return 'use-case';
    }
    if (contents.some(isAllocationUsage)) {
        return 'deployment';
    }
    return 'implementation';
}
