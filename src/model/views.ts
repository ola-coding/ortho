import { AstUtils } from 'langium';
import type { AstNode } from 'langium';
import {
    isActionDef, isAllocationUsage, isPackageDecl, isPartUsage, isUseCaseDef, isUseCaseUsage, isViewUsage
} from '../generated/ast.js';
import type { Exposable, ViewUsage } from '../generated/ast.js';

/** The standard view definitions ortho draws (§9.2.20), and what each may expose. */
export const STANDARD_VIEWS = {
    GeneralView: (e: Exposable) => isPackageDecl(e) || isActionDef(e),
    InterconnectionView: (e: Exposable) => isPartUsage(e),
    SequenceView: (e: Exposable) => isActionDef(e)
};

export type StandardView = keyof typeof STANDARD_VIEWS;

export type DiagramKind = 'use-case' | 'logical' | 'implementation' | 'physical' | 'deployment' | 'process';

export function standardView(view: ViewUsage): StandardView | undefined {
    return view.definition in STANDARD_VIEWS ? view.definition as StandardView : undefined;
}

export function exposedElements(view: ViewUsage): Exposable[] {
    return view.exposes.map(e => e.element.ref).filter((e): e is Exposable => e !== undefined);
}

/** The views a file declares. */
export function declaredViews(root: AstNode): ViewUsage[] {
    return AstUtils.streamAllContents(root).filter(isViewUsage).toArray();
}

/** A diagram's heading is its view's name (§7.26.5). */
export function viewHeading(view: ViewUsage): string {
    return view.name;
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
