import { AstUtils } from 'langium';
import { isAllocationUsage } from '../generated/ast.js';
import type { AllocationUsage, Model, PartUsage } from '../generated/ast.js';
import type { DiagramGraph, GraphEdge, GraphNode } from '../model/graph.js';
import { qualifiedName } from '../model/graph.js';
import { endPart, partLabel } from './connector-ends.js';
import { measureText } from '../render/text-metrics.js';

function partNode(usage: PartUsage): GraphNode {
    const name = partLabel(usage);
    return {
        id: qualifiedName(usage),
        shape: 'box',
        stereotype: 'part',
        name,
        compartments: [],
        ports: [],
        width: Math.max(120, measureText(name, 12, 'bold') + 30),
        height: 42
    };
}

export function extractAllocationGraph(model: Model): DiagramGraph {
    const nodes = new Map<string, GraphNode>();
    const edges: GraphEdge[] = [];
    let edgeCounter = 0;

    const allocations = [...AstUtils.streamAllContents(model)].filter((n): n is AllocationUsage => isAllocationUsage(n));
    for (const allocation of allocations) {
        const source = endPart(allocation.source);
        const target = endPart(allocation.target);
        if (!source || !target || source === target) {
            continue;
        }
        for (const part of [source, target]) {
            const id = qualifiedName(part);
            if (!nodes.has(id)) {
                nodes.set(id, partNode(part));
            }
        }
        edges.push({
            id: `e${edgeCounter++}`,
            kind: 'allocate',
            sourceId: qualifiedName(source),
            targetId: qualifiedName(target),
            label: allocation.name ?? 'allocate'
        });
    }

    return { nodes: [...nodes.values()], edges };
}
