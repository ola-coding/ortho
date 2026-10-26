import type { AstNode } from 'langium';
import { isDefinition } from '../generated/ast.js';

/**
 * How a node is drawn. A definition is a box with square corners and a usage
 * one with rounded corners (§8.2.3.6); the rest have shapes of their own.
 */
export type NodeShape = 'definition' | 'usage' | 'actor' | 'package' | 'node3d';

export interface Compartment {
    /** Drawn small and grey above the lines, as the spec labels a compartment. */
    title?: string;
    lines: string[];
}

/** Box metrics shared by the renderer, the layouter and the extractors. */
export const BOX_HEADER_HEIGHT = 32;
/** A usage's corners; a definition's are square (§8.2.3.6). */
export const USAGE_RADIUS = 10;
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
    /** The keyword drawn above the name, such as `part def`; empty for none. */
    keyword: string;
    name: string;
    compartments: Compartment[];
    ports: GraphPort[];
    /** Nested nodes, such as the parts of an assembly; sized by the layouter, not the extractor. */
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

/** The keyword of each element a view draws as a box (§8.2.3). */
const KEYWORDS: Record<string, string> = {
    PartDef: 'part def',
    PartUsage: 'part',
    ActionDef: 'action def',
    ActionUsage: 'action',
    UseCaseDef: 'use case def',
    UseCaseUsage: 'use case',
    SubjectUsage: 'subject'
};

/** How SysML v2 draws an element as a box: its corners, and the keyword above its name. */
export function boxOf(element: AstNode): { shape: 'definition' | 'usage'; keyword: string } {
    return { shape: isDefinition(element) ? 'definition' : 'usage', keyword: KEYWORDS[element.$type] ?? '' };
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
