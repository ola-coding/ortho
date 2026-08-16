import type { AstNode } from 'langium';

export type NodeShape = 'box' | 'ellipse' | 'actor' | 'boundary' | 'package';

export interface Compartment {
    lines: string[];
}

export interface GraphPort {
    id: string;
    label: string;
}

export interface GraphNode {
    id: string;
    shape: NodeShape;
    stereotype: string;
    name: string;
    compartments: Compartment[];
    ports: GraphPort[];
    /** Nested nodes (system boundary contents); sized by the layouter, not the extractor. */
    children?: GraphNode[];
    width: number;
    height: number;
}

export type EdgeKind =
    'specialization' | 'composition' | 'connection' | 'satisfy'
    | 'association' | 'include' | 'import' | 'allocate' | 'typing';

export interface GraphEdge {
    id: string;
    kind: EdgeKind;
    sourceId: string;
    targetId: string;
    sourcePortId?: string;
    targetPortId?: string;
    label?: string;
}

export interface DiagramGraph {
    nodes: GraphNode[];
    edges: GraphEdge[];
}

export function qualifiedName(node: AstNode & { name: string }): string {
    let qualified = node.name;
    let container: AstNode | undefined = node.$container;
    while (container) {
        const containerName = (container as { name?: string }).name;
        if (containerName) {
            qualified = `${containerName}::${qualified}`;
        }
        container = container.$container;
    }
    return qualified;
}
