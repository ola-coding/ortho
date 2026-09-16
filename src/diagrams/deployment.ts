import { AstUtils } from 'langium';
import { isAllocationUsage, isPartDef, isPartUsage } from '../generated/ast.js';
import type { AllocationUsage, Model, PartDef, PartUsage } from '../generated/ast.js';
import type { DiagramGraph, GraphNode } from '../model/graph.js';
import { qualifiedName } from '../model/graph.js';
import { endPart, partLabel } from './connector-ends.js';
import { measureText } from '../render/text-metrics.js';
import { NODE_DEPTH } from '../layout/elk-layout.js';

/** A deployed program: a plain box, since everything inside a host is one. */
function softwareNode(usage: PartUsage): GraphNode {
    const name = partLabel(usage);
    return {
        id: qualifiedName(usage),
        shape: 'box',
        stereotype: '',
        name,
        compartments: [],
        ports: [],
        width: Math.max(120, measureText(name, 12, 'bold') + 30),
        height: 40
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

/** A physical device the hosts are fitted in, drawn as an assembly frame. */
function deviceFrame(id: string, name: string): GraphNode {
    return {
        id,
        shape: 'box',
        stereotype: '',
        name,
        compartments: [],
        ports: [],
        children: [],
        width: Math.max(150, measureText(name, 12, 'bold') + 40),
        height: 60
    };
}

/**
 * The Deployment view renders the allocation as containment: each hardware host
 * is a three-dimensional node and the software allocated to it is drawn inside.
 * There are no allocation edges — a host with nothing drawn in it hosts nothing,
 * which is the fact the view exists to show.
 *
 * Hosts sit inside a frame for the device they are fitted in, borrowed from the
 * physical containment: both computers inside the drone, the tablet in the
 * ground station. Which programs run in the air and which on the ground then
 * reads at a glance. A device fitted exactly once is labelled by that usage
 * (`drone : SurveyDrone`) and nested in the frame of the device that
 * contains it; a top-level definition, or one fitted in several places, is a
 * frame of its own. A host declared directly in a package has no frame.
 *
 * Software allocated to more than one host would need to appear twice, which
 * containment cannot express; the first host named wins and the rest are
 * skipped, so every software part is drawn exactly once.
 */
export function extractDeploymentGraph(model: Model): DiagramGraph {
    const contents = [...AstUtils.streamAllContents(model)];
    const allocations = contents.filter((n): n is AllocationUsage => isAllocationUsage(n));

    const usagesOf = new Map<PartDef, PartUsage[]>();
    for (const node of contents) {
        if (isPartUsage(node) && node.type?.ref) {
            usagesOf.set(node.type.ref, [...(usagesOf.get(node.type.ref) ?? []), node]);
        }
    }

    const roots: GraphNode[] = [];
    const frames = new Map<PartDef, GraphNode>();
    const frameFor = (def: PartDef, path: Set<PartDef>): GraphNode => {
        const existing = frames.get(def);
        if (existing) {
            return existing;
        }
        const uses = usagesOf.get(def) ?? [];
        const frame = deviceFrame(`device:${qualifiedName(def)}`, uses.length === 1 ? partLabel(uses[0]) : def.name);
        frames.set(def, frame);
        const owner = uses.length === 1 ? uses[0].$container : undefined;
        if (isPartDef(owner) && !path.has(owner)) {
            frameFor(owner, new Set([...path, def])).children!.push(frame);
        } else {
            roots.push(frame);
        }
        return frame;
    };

    const hosts = new Map<string, GraphNode>();
    const placed = new Set<string>();
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
            const device = host.$container;
            if (isPartDef(device)) {
                frameFor(device, new Set()).children!.push(hostGraphNode);
            } else {
                roots.push(hostGraphNode);
            }
        }
        const softwareId = qualifiedName(software);
        if (placed.has(softwareId)) {
            continue;
        }
        placed.add(softwareId);
        hostGraphNode.children!.push(softwareNode(software));
    }

    return { nodes: roots, edges: [] };
}
