import { AstUtils } from 'langium';
import { isActionDef, isActionUsage, isDependency, isUseCaseDef, isUseCaseUsage } from '../generated/ast.js';
import type { ActionDef, ActionUsage, Member, Model } from '../generated/ast.js';
import type { DiagramGraph, GraphEdge, GraphNode } from '../model/graph.js';
import { boxOf, qualifiedName } from '../model/graph.js';
import { exposedMembers } from '../model/packages.js';
import { boxSize } from './connector-ends.js';
import { useCaseNode } from './use-case.js';

type Function_ = ActionDef | ActionUsage;

/** «action def» for the root of a tree, «action» for the functions in it. */
function functionNode(fn: Function_): GraphNode {
    const { shape, keyword } = boxOf(fn);
    return {
        id: qualifiedName(fn),
        shape,
        keyword,
        name: fn.name ?? '',
        compartments: [],
        ports: [],
        ...boxSize(fn.name ?? '', [], 110, 40, keyword)
    };
}

/** Nested action usages, in declaration order — the function's children. */
function childFunctions(fn: Function_): ActionUsage[] {
    return fn.members.filter(isActionUsage);
}

/**
 * The Logical view is a capability tree: every function is one box with exactly
 * one parent, and a branch is a plain line rather than an arrow. Decomposition
 * comes from nesting in the source — an `action` declared inside another is its
 * child — so a function appears exactly once no matter how many things use it.
 *
 * A typed action usage in a package is one use of a tree, not a tree of its
 * own: `action survey : PerformAerialSurvey` is what a realizing part's
 * `perform` chain starts from, and the tree it names is already drawn from
 * its definition. Drawing it too would double the diagram.
 *
 * The functions detail what the use cases ask for. A use case the view exposes
 * is drawn too, and a dependency joining two things on the view, such as a
 * tree's `#refinement dependency` on its use case, is a dashed arrow (§8.2.3.3).
 */
export function extractLogicalGraph(exposed: Member[], model: Model): DiagramGraph {
    const nodes: GraphNode[] = [];
    const edges: GraphEdge[] = [];
    const seen = new Set<string>();
    let edgeCounter = 0;

    const visit = (fn: Function_): void => {
        const id = qualifiedName(fn);
        if (seen.has(id)) {
            return;
        }
        seen.add(id);
        nodes.push(functionNode(fn));
        for (const child of childFunctions(fn)) {
            const childId = qualifiedName(child);
            visit(child);
            edges.push({
                id: `e${edgeCounter++}`,
                kind: 'decomposition',
                sourceId: id,
                targetId: childId
            });
        }
    };

    for (const member of exposedMembers(exposed)) {
        if (isActionDef(member) || (isActionUsage(member) && !member.type)) {
            visit(member);
        } else if ((isUseCaseDef(member) || isUseCaseUsage(member)) && !seen.has(qualifiedName(member))) {
            seen.add(qualifiedName(member));
            nodes.push(useCaseNode(member));
        }
    }

    for (const dependency of AstUtils.streamAllContents(model).filter(isDependency)) {
        const ids = (ends: typeof dependency.clients): string[] => ends
            .flatMap(end => (end.ref ? [qualifiedName(end.ref)] : []))
            .filter(id => seen.has(id));
        for (const sourceId of ids(dependency.clients)) {
            for (const targetId of ids(dependency.suppliers)) {
                edges.push({ id: `e${edgeCounter++}`, kind: 'dependency', sourceId, targetId });
            }
        }
    }

    return { nodes, edges };
}
