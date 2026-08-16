import type { AstNode } from 'langium';
import {
    isAttributeUsage, isConnectionUsage, isInterfaceDef, isPackageDecl, isPartDef, isPartUsage,
    isPortUsage, isRequirementDef, isRequirementUsage, isSatisfyUsage, isSubjectUsage
} from '../generated/ast.js';
import type {
    AttributeUsage, ConnectionUsage, ConnectorEnd, InterfaceDef, Model, Multiplicity,
    PartDef, PartUsage, RequirementDef, RequirementUsage, SatisfyUsage
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

function interfaceDefNode(def: InterfaceDef): GraphNode {
    const ends = def.ends.map(e => `end ${e.name} : ${e.type.ref?.name ?? e.type.$refText}`);
    return makeNode({
        id: qualifiedName(def),
        stereotype: 'interface def',
        name: def.name,
        compartments: [{ lines: def.members.map(attributeLine) }, { lines: ends }],
        ports: []
    });
}

function requirementDefNode(def: RequirementDef): GraphNode {
    const lines: string[] = [];
    for (const member of def.members) {
        if (isAttributeUsage(member)) {
            lines.push(attributeLine(member));
        } else if (isSubjectUsage(member)) {
            lines.push(`subject ${member.name}${member.type ? ` : ${member.type.ref?.name ?? member.type.$refText}` : ''}`);
        }
    }
    return makeNode({
        id: qualifiedName(def),
        stereotype: 'requirement def',
        name: def.name,
        compartments: [{ lines }],
        ports: []
    });
}

function requirementUsageNode(usage: RequirementUsage): GraphNode {
    const lines = usage.members.filter(isAttributeUsage).map(attributeLine);
    const type = usage.type ? ` : ${usage.type.ref?.name ?? usage.type.$refText}` : '';
    return makeNode({
        id: qualifiedName(usage),
        stereotype: 'requirement',
        name: `${usage.name}${type}`,
        compartments: [{ lines }],
        ports: []
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
 * Maps a connector end (`battery.powerOut`) to a diagram anchor. Part usages
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
 * Port defs are deliberately not rendered. No edge kind here can terminate on
 * one — composition, connection, specialization and satisfy all anchor to part
 * or requirement nodes — so a port def could only ever appear as a disconnected
 * box. The topology it would document is already carried by the port markers on
 * each part, and drawing a typing edge from every port usage to its def would
 * bury the diagram. Interface contracts and their attributes belong on a
 * dedicated interface view instead.
 */
export function extractPartDefinitionGraph(model: Model): DiagramGraph {
    const nodes: GraphNode[] = [];
    const edges: GraphEdge[] = [];
    const connections: ConnectionUsage[] = [];
    const satisfies: SatisfyUsage[] = [];
    let edgeCounter = 0;

    const partDefs: PartDef[] = [];
    const requirementUsages: RequirementUsage[] = [];
    for (const pkg of collectPackages(model)) {
        for (const member of pkg.members) {
            if (isPartDef(member)) {
                partDefs.push(member);
                nodes.push(partDefNode(member));
            } else if (isInterfaceDef(member)) {
                nodes.push(interfaceDefNode(member));
            } else if (isRequirementDef(member)) {
                nodes.push(requirementDefNode(member));
            } else if (isRequirementUsage(member)) {
                requirementUsages.push(member);
                nodes.push(requirementUsageNode(member));
            } else if (isPartUsage(member)) {
                nodes.push(partUsageNode(member));
            } else if (isConnectionUsage(member)) {
                connections.push(member);
            } else if (isSatisfyUsage(member)) {
                satisfies.push(member);
            }
        }
    }

    const nodeIds = new Set(nodes.map(n => n.id));
    const nodePorts = new Map(nodes.map(n => [n.id, new Set(n.ports.map(p => p.id))]));

    for (const def of partDefs) {
        const defId = qualifiedName(def);
        // Specializations: laid out super → sub so generalizations point upward;
        // the renderer draws the triangle at the layout-source (super) end.
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
            } else if (isSatisfyUsage(member)) {
                satisfies.push(member);
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
                label: connection.type?.ref?.name ?? connection.name
            });
        }
    }

    // A requirement usage is typed by the def that carries the actual values, and
    // `satisfy` links to the usage. Without this edge the def — the box holding
    // the number the design has to meet — would sit on the diagram unconnected.
    for (const usage of requirementUsages) {
        const def = usage.type?.ref;
        if (!def) {
            continue;
        }
        const usageId = qualifiedName(usage);
        const defId = qualifiedName(def);
        if (nodeIds.has(usageId) && nodeIds.has(defId)) {
            edges.push({
                id: `e${edgeCounter++}`,
                kind: 'typing',
                sourceId: usageId,
                targetId: defId,
                label: 'typed by'
            });
        }
    }

    for (const satisfy of satisfies) {
        const satisfier = satisfy.satisfier.ref;
        const requirement = satisfy.requirement.ref;
        if (satisfier && requirement) {
            const sourceId = qualifiedName(satisfier);
            const targetId = qualifiedName(requirement);
            if (nodeIds.has(sourceId) && nodeIds.has(targetId)) {
                edges.push({
                    id: `e${edgeCounter++}`,
                    kind: 'satisfy',
                    sourceId,
                    targetId,
                    label: 'satisfy'
                });
            }
        }
    }

    return { nodes, edges };
}
