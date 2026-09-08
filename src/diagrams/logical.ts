import { isActionDef, isActionUsage } from '../generated/ast.js';
import type { ActionDef, ActionUsage, Model } from '../generated/ast.js';
import type { DiagramGraph, GraphEdge, GraphNode } from '../model/graph.js';
import { qualifiedName } from '../model/graph.js';
import { collectPackages } from '../model/packages.js';
import { measureText } from '../render/text-metrics.js';

type Function_ = ActionDef | ActionUsage;

function functionNode(fn: Function_): GraphNode {
    return {
        id: qualifiedName(fn),
        shape: 'rounded',
        stereotype: '',
        name: fn.name,
        compartments: [],
        ports: [],
        width: Math.max(110, measureText(fn.name, 12) + 34),
        height: 40
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
 * Messages are deliberately ignored. A scenario file also nests actions, but
 * its content belongs to the Process view; keeping one file per view is what
 * separates the two.
 */
export function extractLogicalGraph(model: Model): DiagramGraph {
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

    for (const pkg of collectPackages(model)) {
        for (const member of pkg.members) {
            if (isActionDef(member) || isActionUsage(member)) {
                visit(member);
            }
        }
    }

    return { nodes, edges };
}
