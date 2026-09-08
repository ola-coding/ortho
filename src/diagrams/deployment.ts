import { AstUtils } from 'langium';
import { isAllocationUsage } from '../generated/ast.js';
import type { AllocationUsage, Model, PartUsage } from '../generated/ast.js';
import type { DiagramGraph, GraphNode } from '../model/graph.js';
import { qualifiedName } from '../model/graph.js';
import { endPart, partLabel } from './connector-ends.js';
import { measureText } from '../render/text-metrics.js';
import { NODE_DEPTH } from '../layout/elk-layout.js';

function softwareNode(usage: PartUsage): GraphNode {
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

function hostNode(usage: PartUsage): GraphNode {
    const name = partLabel(usage);
    return {
        id: qualifiedName(usage),
        shape: 'node3d',
        stereotype: '',
        name,
        compartments: [],
        ports: [],
        children: [],
        // Only used when the host ends up with nothing allocated to it; a host
        // with children is sized by the layouter.
        width: Math.max(150, measureText(name, 12, 'bold') + 40 + NODE_DEPTH),
        height: 60 + NODE_DEPTH
    };
}

/**
 * The Deployment view renders the allocation as containment: each hardware host
 * is a three-dimensional node and the software allocated to it is drawn inside.
 * There are no allocation edges — a host with nothing drawn in it hosts nothing,
 * which is the fact the view exists to show.
 *
 * Software allocated to more than one host would need to appear twice, which
 * containment cannot express; the first host named wins and the rest are
 * skipped, so every software part is drawn exactly once.
 */
export function extractDeploymentGraph(model: Model): DiagramGraph {
    const hosts = new Map<string, GraphNode>();
    const placed = new Set<string>();

    const allocations = [...AstUtils.streamAllContents(model)]
        .filter((n): n is AllocationUsage => isAllocationUsage(n));

    for (const allocation of allocations) {
        const software = endPart(allocation.source);
        const host = endPart(allocation.target);
        if (!software || !host || software === host) {
            continue;
        }
        const hostId = qualifiedName(host);
        let hostGraphNode = hosts.get(hostId);
        if (!hostGraphNode) {
            hostGraphNode = hostNode(host);
            hosts.set(hostId, hostGraphNode);
        }
        const softwareId = qualifiedName(software);
        if (placed.has(softwareId)) {
            continue;
        }
        placed.add(softwareId);
        hostGraphNode.children!.push(softwareNode(software));
    }

    return { nodes: [...hosts.values()], edges: [] };
}
