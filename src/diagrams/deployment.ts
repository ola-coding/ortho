import { AstUtils } from 'langium';
import { isAllocationUsage, isPartDef, isPartUsage } from '../generated/ast.js';
import type { AllocationUsage, ConnectorEnd, Definition, Member, Model, PartDef, PartUsage } from '../generated/ast.js';
import type { DiagramGraph, GraphNode } from '../model/graph.js';
import { boxOf, qualifiedName } from '../model/graph.js';
import { boxSize, endPart, usageLabel } from './connector-ends.js';
import { measureText } from '../render/text-metrics.js';

/** A connector end as written: `sw.recipes`. */
function endText(end: ConnectorEnd): string {
    return end.segments.map(segment => segment.$refText).join('.');
}

/**
 * A host: a «part» box listing what is allocated to it in its *relationships*
 * compartment (§8.2.3.5), `allocated sw.recipes`.
 */
function hostNode(host: PartUsage, allocated: string[]): GraphNode {
    const name = usageLabel(host);
    const compartments = [{ title: 'relationships', lines: allocated.map(software => `allocated ${software}`) }];
    const { shape, keyword } = boxOf(host);
    return {
        id: qualifiedName(host),
        shape,
        keyword,
        name,
        compartments,
        ports: [],
        ...boxSize(name, compartments, 150, 40, keyword)
    };
}

/**
 * A physical device the hosts are fitted in, drawn as an assembly frame: the
 * device's one usage, or its definition when it has none or several.
 */
function deviceFrame(id: string, device: PartDef | PartUsage): GraphNode {
    const name = isPartUsage(device) ? usageLabel(device) : device.name ?? '';
    const { shape, keyword } = boxOf(device);
    return {
        id,
        shape,
        keyword,
        name,
        compartments: [],
        ports: [],
        children: [],
        width: Math.max(150, measureText(name, 12, 'bold') + 40),
        height: 60
    };
}

/**
 * The Deployment view lists on each host what is allocated to it (§8.2.3.15).
 * Nothing is drawn inside a host, since a box drawn inside another is its
 * member, which software is not; and an «allocate» arrow per program would
 * bury the hosts under arrows. A host appears once something is allocated to
 * it, and a program allocated to two hosts is listed on both.
 *
 * Hosts sit inside a frame for the device they are fitted in, borrowed from the
 * physical containment: both computers inside the drone, the tablet in the
 * ground station. Which programs run in the air and which on the ground then
 * reads at a glance. A device fitted exactly once is labelled by that usage
 * (`drone : SurveyDrone`) and nested in the frame of the device that
 * contains it; a top-level definition, or one fitted in several places, is a
 * frame of its own. A host declared directly in a package has no frame.
 */
export function extractDeploymentGraph(exposed: Member[], model: Model): DiagramGraph {
    const allocations = exposed.flatMap(e => [...AstUtils.streamAllContents(e)])
        .filter((n): n is AllocationUsage => isAllocationUsage(n));
    const contents = [...AstUtils.streamAllContents(model)];

    const usagesOf = new Map<Definition, PartUsage[]>();
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
        const frame = deviceFrame(`device:${qualifiedName(def)}`, uses.length === 1 ? uses[0] : def);
        frames.set(def, frame);
        const owner = uses.length === 1 ? uses[0].$container : undefined;
        if (isPartDef(owner) && !path.has(owner)) {
            frameFor(owner, new Set([...path, def])).children!.push(frame);
        } else {
            roots.push(frame);
        }
        return frame;
    };

    // What each host runs, in the order it was allocated.
    const allocated = new Map<PartUsage, string[]>();
    for (const allocation of allocations) {
        const host = endPart(allocation.target);
        if (!host || !allocation.source || endPart(allocation.source) === host) {
            continue;
        }
        const software = allocated.get(host) ?? [];
        if (!software.includes(endText(allocation.source))) {
            software.push(endText(allocation.source));
        }
        allocated.set(host, software);
    }

    for (const [host, software] of allocated) {
        const node = hostNode(host, software);
        const device = host.$container;
        if (isPartDef(device)) {
            frameFor(device, new Set()).children!.push(node);
        } else {
            roots.push(node);
        }
    }

    return { nodes: roots, edges: [] };
}
