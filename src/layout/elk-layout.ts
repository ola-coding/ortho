import ElkConstructor from 'elkjs';
import type { ELK, ElkNode, ElkExtendedEdge, ELKConstructorArguments } from 'elkjs/lib/elk-api.js';
import type { DiagramGraph, EdgeKind, GraphNode, NodeShape } from '../model/graph.js';

const ELK = ElkConstructor as unknown as { new(args?: ELKConstructorArguments): ELK };
const elk = new ELK();

export const PORT_SIZE = 10;

export interface LayoutOptions {
    direction?: 'DOWN' | 'RIGHT';
}

export interface LaidOutPort {
    id: string;
    label: string;
    x: number;
    y: number;
}

export interface LaidOutNode {
    id: string;
    shape: NodeShape;
    stereotype: string;
    name: string;
    compartments: Array<{ lines: string[] }>;
    ports: LaidOutPort[];
    hasChildren: boolean;
    x: number;
    y: number;
    width: number;
    height: number;
}

export interface LaidOutEdge {
    id: string;
    kind: EdgeKind;
    sourceId: string;
    targetId: string;
    label?: string;
    points: Array<{ x: number; y: number }>;
}

export interface LaidOutDiagram {
    width: number;
    height: number;
    nodes: LaidOutNode[];
    edges: LaidOutEdge[];
}

function toElkNode(n: GraphNode): ElkNode {
    const elkNode: ElkNode = {
        id: n.id,
        layoutOptions: { 'elk.portConstraints': 'FREE' }
    };
    if (n.children?.length) {
        elkNode.children = n.children.map(toElkNode);
        // Extra top padding leaves room for the boundary title.
        elkNode.layoutOptions!['elk.padding'] = '[top=45,left=25,bottom=25,right=25]';
        elkNode.layoutOptions!['elk.spacing.nodeNode'] = '40';
    } else {
        elkNode.width = n.width;
        elkNode.height = n.height;
    }
    if (n.ports.length > 0) {
        elkNode.ports = n.ports.map(p => ({
            id: p.id,
            width: PORT_SIZE,
            height: PORT_SIZE,
            // straddle the node border, EA-style
            layoutOptions: { 'elk.port.borderOffset': `${-PORT_SIZE / 2}` }
        }));
    }
    return elkNode;
}

function indexHierarchy(nodes: GraphNode[], byId: Map<string, GraphNode>, parentOf: Map<string, string>): void {
    for (const node of nodes) {
        byId.set(node.id, node);
        for (const child of node.children ?? []) {
            parentOf.set(child.id, node.id);
        }
        indexHierarchy(node.children ?? [], byId, parentOf);
    }
}

export async function layoutGraph(graph: DiagramGraph, options: LayoutOptions = {}): Promise<LaidOutDiagram> {
    const elkGraph: ElkNode = {
        id: 'root',
        layoutOptions: {
            'elk.algorithm': 'layered',
            'elk.direction': options.direction ?? 'DOWN',
            'elk.hierarchyHandling': 'INCLUDE_CHILDREN',
            'elk.spacing.nodeNode': '50',
            'elk.layered.spacing.nodeNodeBetweenLayers': '70',
            'elk.spacing.edgeNode': '30',
            'elk.spacing.edgeEdge': '20',
            'elk.edgeRouting': 'ORTHOGONAL'
        },
        children: graph.nodes.map(toElkNode),
        edges: graph.edges.map(e => ({
            id: e.id,
            sources: [e.sourcePortId ?? e.sourceId],
            targets: [e.targetPortId ?? e.targetId]
        }))
    };

    const result = await elk.layout(elkGraph);

    const graphNodeById = new Map<string, GraphNode>();
    const parentOf = new Map<string, string>();
    indexHierarchy(graph.nodes, graphNodeById, parentOf);
    const graphEdgeById = new Map(graph.edges.map(e => [e.id, e]));

    // Flatten the hierarchy to absolute coordinates, parents before children,
    // so paint order keeps boundary boxes behind their contents.
    const nodes: LaidOutNode[] = [];
    const flatten = (children: ElkNode[] | undefined, offsetX: number, offsetY: number) => {
        for (const c of children ?? []) {
            const source = graphNodeById.get(c.id);
            const portLabels = new Map((source?.ports ?? []).map(p => [p.id, p.label]));
            const x = (c.x ?? 0) + offsetX;
            const y = (c.y ?? 0) + offsetY;
            nodes.push({
                id: c.id,
                shape: source?.shape ?? 'box',
                stereotype: source?.stereotype ?? '',
                name: source?.name ?? c.id,
                compartments: source?.compartments ?? [],
                hasChildren: (source?.children?.length ?? 0) > 0,
                ports: (c.ports ?? []).map(p => ({
                    id: p.id,
                    label: portLabels.get(p.id) ?? p.id,
                    x: p.x ?? 0,
                    y: p.y ?? 0
                })),
                x,
                y,
                width: c.width ?? 80,
                height: c.height ?? 40
            });
            flatten(c.children, x, y);
        }
    };
    flatten(result.children, 0, 0);

    const absolutePosition = new Map(nodes.map(n => [n.id, { x: n.x, y: n.y }]));

    // ELK reports edge coordinates relative to the least common ancestor
    // container of the endpoints; translate them back to root coordinates.
    const ancestors = (id: string): string[] => {
        const chain: string[] = [];
        let current = parentOf.get(id);
        while (current) {
            chain.push(current);
            current = parentOf.get(current);
        }
        return chain;
    };
    const edgeOffset = (sourceId: string, targetId: string): { x: number; y: number } => {
        const sourceAncestors = new Set(ancestors(sourceId));
        const lca = ancestors(targetId).find(id => sourceAncestors.has(id));
        return (lca && absolutePosition.get(lca)) || { x: 0, y: 0 };
    };

    const edges: LaidOutEdge[] = (result.edges ?? []).map((e: ElkExtendedEdge) => {
        const section = e.sections?.[0];
        const rawPoints = section
            ? [section.startPoint, ...(section.bendPoints ?? []), section.endPoint]
            : [];
        const source = graphEdgeById.get(e.id);
        const offset = source ? edgeOffset(source.sourceId, source.targetId) : { x: 0, y: 0 };
        return {
            id: e.id,
            kind: source?.kind ?? 'connection',
            sourceId: source?.sourceId ?? '',
            targetId: source?.targetId ?? '',
            label: source?.label,
            points: rawPoints.map(p => ({ x: p.x + offset.x, y: p.y + offset.y }))
        };
    });

    return {
        width: result.width ?? 400,
        height: result.height ?? 300,
        nodes,
        edges
    };
}
