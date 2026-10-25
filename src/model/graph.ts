import type { AstNode } from 'langium';

export type NodeShape = 'box' | 'ellipse' | 'actor' | 'boundary' | 'package' | 'rounded' | 'node3d';

export interface Compartment {
    /** Drawn small and grey above the lines, as the spec labels a compartment. */
    title?: string;
    lines: string[];
}

/** Box metrics shared by the renderer, the layouter and the extractors. */
export const BOX_HEADER_HEIGHT = 32;
export const COMPARTMENT_LINE_HEIGHT = 15;
export const COMPARTMENT_TITLE_HEIGHT = 13;
export const COMPARTMENT_PADDING = 8;

/** How much height a box's compartments take below its header. */
export function compartmentsHeight(compartments: Compartment[]): number {
    return compartments.reduce(
        (sum, c) => sum + COMPARTMENT_PADDING + (c.title ? COMPARTMENT_TITLE_HEIGHT : 0)
            + c.lines.length * COMPARTMENT_LINE_HEIGHT,
        0
    );
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

export type EdgeKind = 'connection' | 'association' | 'include' | 'import' | 'decomposition';

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

/**
 * An element's name, or the one an unnamed element takes on: a redefinition,
 * `part :>> engine`, is named after what it redefines, and a perform,
 * `perform providePower.generateTorque`, after what it performs.
 */
export function effectiveName(node: AstNode): string | undefined {
    const element = node as { name?: string; redefines?: string[]; target?: { segments?: Array<{ $refText: string }> } };
    if (element.name !== undefined) {
        return element.name;
    }
    const named = element.redefines?.[0] ?? (node.$type === 'PerformUsage' ? element.target?.segments?.at(-1)?.$refText : undefined);
    return named?.split(/::|\./).at(-1);
}

export function qualifiedName(node: AstNode): string {
    let qualified = effectiveName(node) ?? '';
    let container: AstNode | undefined = node.$container;
    while (container) {
        const containerName = effectiveName(container);
        if (containerName) {
            qualified = `${containerName}::${qualified}`;
        }
        container = container.$container;
    }
    return qualified;
}
