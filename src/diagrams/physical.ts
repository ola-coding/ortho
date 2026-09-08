import {
    isAttributeUsage, isConnectionUsage, isPackageDecl, isPartDef, isPartUsage, isPortUsage
} from '../generated/ast.js';
import type {
    AttributeUsage, ConnectionUsage, ConnectorEnd, Model, Multiplicity, PartDef, PartUsage
} from '../generated/ast.js';
import type { DiagramGraph, GraphEdge, GraphNode, GraphPort } from '../model/graph.js';
import { qualifiedName } from '../model/graph.js';
import { collectPackages } from '../model/packages.js';
import { partLabel } from './connector-ends.js';
import { measureText } from '../render/text-metrics.js';

const HEADER_HEIGHT = 32;
const LINE_HEIGHT = 15;
const COMPARTMENT_PADDING = 8;
const MIN_WIDTH = 110;

function multiplicityText(multiplicity: Multiplicity | undefined): string {
    if (!multiplicity) {
        return '';
    }
    return multiplicity.upper !== undefined
        ? ` [${multiplicity.lower}..${multiplicity.upper}]`
        : ` [${multiplicity.lower}]`;
}

function attributeLine(attribute: AttributeUsage): string {
    let line = attribute.name;
    if (attribute.type) {
        line += ` : ${attribute.type}`;
    }
    if (attribute.value !== undefined) {
        line += ` = ${attribute.value}`;
    }
    return line;
}

function partLine(part: PartUsage): string {
    return `${partLabel(part)}${multiplicityText(part.multiplicity)}`;
}

function nodeSize(node: Omit<GraphNode, 'width' | 'height'>): { width: number; height: number } {
    let width = Math.max(
        MIN_WIDTH,
        measureText(node.name, 12, 'bold') + 24,
        measureText(`«${node.stereotype}»`, 9) + 24
    );
    let height = HEADER_HEIGHT;
    for (const compartment of node.compartments) {
        height += COMPARTMENT_PADDING + compartment.lines.length * LINE_HEIGHT;
        for (const line of compartment.lines) {
            width = Math.max(width, measureText(line, 11) + 20);
        }
    }
    if (node.compartments.length === 0) {
        height += 10;
    }
    if (node.ports.length > 0) {
        width = Math.max(width, 130);
        for (const port of node.ports) {
            // port square + beside-label must fit twice over (label may sit
            // on either side of the square)
            width = Math.max(width, 2 * (measureText(port.label, 9) + 18) + 20);
        }
    }
    return { width, height };
}

function makeNode(partial: Omit<GraphNode, 'width' | 'height' | 'shape'>): GraphNode {
    const compartments = partial.compartments.filter(c => c.lines.length > 0);
    const sized = nodeSize({ ...partial, shape: 'box', compartments });
    return { ...partial, shape: 'box', compartments, ...sized };
}

function partDefNode(def: PartDef): GraphNode {
    const id = qualifiedName(def);
    const attributes = def.members.filter(isAttributeUsage).map(attributeLine);
    const parts = def.members.filter(isPartUsage).map(partLine);
    const ports: GraphPort[] = def.members.filter(isPortUsage).map(p => ({
        id: `${id}.${p.name}`,
        label: p.name
    }));
    return makeNode({
        id,
        stereotype: 'part def',
        name: def.name,
        compartments: [{ lines: attributes }, { lines: parts }],
        ports
    });
}

function partUsageNode(usage: PartUsage): GraphNode {
    const id = qualifiedName(usage);
    const type = usage.type ? ` : ${usage.type.ref?.name ?? usage.type.$refText}` : '';
    const attributes = usage.members.filter(isAttributeUsage).map(attributeLine);
    const parts = usage.members.filter(isPartUsage).map(partLine);
    const ports: GraphPort[] = usage.members.filter(isPortUsage).map(p => ({
        id: `${id}.${p.name}`,
        label: p.name
    }));
    return makeNode({
        id,
        stereotype: 'part',
        name: `${usage.name}${type}${multiplicityText(usage.multiplicity)}`,
        compartments: [{ lines: attributes }, { lines: parts }],
        ports
    });
}

interface EdgeAnchor {
    nodeId: string;
    portId?: string;
}

/**
 * Maps a connector end (`tank.waterOut`) to a diagram anchor. Part usages
 * anchor to their type's definition node (or to their own node when they are
 * package-level usages); ports anchor to the port marker on that node.
 */
function endAnchor(end: ConnectorEnd, nodePorts: Map<string, Set<string>>): EdgeAnchor | undefined {
    const last = end.segments[end.segments.length - 1]?.ref;
    if (!last) {
        return undefined;
    }
    if (isPortUsage(last)) {
        const owner = last.$container;
        let ownerId: string | undefined;
        if (isPartDef(owner)) {
            ownerId = qualifiedName(owner);
        } else if (isPartUsage(owner)) {
            ownerId = isPackageDecl(owner.$container)
                ? qualifiedName(owner)
                : owner.type?.ref ? qualifiedName(owner.type.ref) : undefined;
        }
        if (!ownerId || !nodePorts.has(ownerId)) {
            return undefined;
        }
        const portId = `${ownerId}.${last.name}`;
        return nodePorts.get(ownerId)!.has(portId)
            ? { nodeId: ownerId, portId }
            : { nodeId: ownerId };
    }
    if (isPartUsage(last)) {
        if (isPackageDecl(last.$container)) {
            const id = qualifiedName(last);
            return nodePorts.has(id) ? { nodeId: id } : undefined;
        }
        const def = last.type?.ref;
        if (def) {
            const id = qualifiedName(def);
            return nodePorts.has(id) ? { nodeId: id } : undefined;
        }
    }
    return undefined;
}

/**
 * The Physical view is topology: the parts of the product, the ports on them,
 * and what is connected, mounted or wired to what.
 *
 * Port defs are not rendered — no edge kind here can terminate on one, so they
 * could only ever be disconnected boxes, and the topology they document is
 * already carried by the port markers on each part. Interface defs are not
 * rendered either: an interface names the contract a connection satisfies, so
 * it belongs on the connection's label rather than in a box of its own.
 */
export function extractPhysicalGraph(model: Model): DiagramGraph {
    const nodes: GraphNode[] = [];
    const edges: GraphEdge[] = [];
    const connections: ConnectionUsage[] = [];
    let edgeCounter = 0;

    const partDefs: PartDef[] = [];
    for (const pkg of collectPackages(model)) {
        for (const member of pkg.members) {
            if (isPartDef(member)) {
                partDefs.push(member);
                nodes.push(partDefNode(member));
            } else if (isPartUsage(member)) {
                nodes.push(partUsageNode(member));
            } else if (isConnectionUsage(member)) {
                connections.push(member);
            }
        }
    }

    const nodeIds = new Set(nodes.map(n => n.id));
    const nodePorts = new Map(nodes.map(n => [n.id, new Set(n.ports.map(p => p.id))]));

    for (const def of partDefs) {
        const defId = qualifiedName(def);
        // Specializations are laid out super → sub so generalizations point
        // upward; the renderer draws the triangle at the layout-source end.
        for (const supertype of def.supertypes) {
            if (supertype.ref) {
                const superId = qualifiedName(supertype.ref);
                if (nodeIds.has(superId)) {
                    edges.push({
                        id: `e${edgeCounter++}`,
                        kind: 'specialization',
                        sourceId: superId,
                        targetId: defId
                    });
                }
            }
        }
        for (const member of def.members) {
            if (isPartUsage(member) && member.type?.ref) {
                const typeId = qualifiedName(member.type.ref);
                if (nodeIds.has(typeId) && typeId !== defId) {
                    edges.push({
                        id: `e${edgeCounter++}`,
                        kind: 'composition',
                        sourceId: defId,
                        targetId: typeId,
                        label: `${member.name}${multiplicityText(member.multiplicity)}`
                    });
                }
            } else if (isConnectionUsage(member)) {
                connections.push(member);
            }
        }
    }

    for (const connection of connections) {
        const source = endAnchor(connection.source, nodePorts);
        const target = endAnchor(connection.target, nodePorts);
        if (source && target) {
            edges.push({
                id: `e${edgeCounter++}`,
                kind: 'connection',
                sourceId: source.nodeId,
                targetId: target.nodeId,
                sourcePortId: source.portId,
                targetPortId: target.portId,
                // A typed connection carries its interface name on the line.
                label: connection.type?.ref?.name ?? connection.name
            });
        }
    }

    return { nodes, edges };
}
