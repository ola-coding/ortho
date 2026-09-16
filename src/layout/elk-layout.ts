import ElkConstructor from 'elkjs';
import type { ELK, ElkNode, ElkExtendedEdge, ELKConstructorArguments } from 'elkjs/lib/elk-api.js';
import type { DiagramGraph, EdgeKind, GraphNode, NodeShape } from '../model/graph.js';
import { measureText } from '../render/text-metrics.js';
import { straightenEdges } from './straight-edges.js';

const ELK = ElkConstructor as unknown as { new(args?: ELKConstructorArguments): ELK };
const elk = new ELK();

export const PORT_SIZE = 10;

/** Port labels: font size, and the height of the box ELK reserves for one. */
export const PORT_LABEL_FONT = 9;
const PORT_LABEL_HEIGHT = 11;

/** Wire labels ELK places (interface names on the physical view). */
const EDGE_LABEL_FONT = 9;
const EDGE_LABEL_HEIGHT = 12;

/** Depth of the three-dimensional edge on a Deployment-view hardware node. */
export const NODE_DEPTH = 12;

export interface LayoutOptions {
    direction?: 'DOWN' | 'RIGHT';
    /**
     * `STRAIGHT` draws each line between ellipses and actors as one segment
     * wherever that clears every other shape (the use case view; see
     * straight-edges.ts). ELK still places the shapes, routing with `POLYLINE`
     * for the lines that cannot be straight.
     */
    edgeRouting?: 'ORTHOGONAL' | 'STRAIGHT';
    /**
     * `rectpacking` packs boxes into rows at a page-shaped aspect ratio, level
     * by level, instead of layering them. It is for edge-free views
     * (deployment), where layering would put every box in one long row.
     */
    algorithm?: 'layered' | 'rectpacking';
    /**
     * Lay out the contents of each innermost container — one whose children
     * hold nothing and no edge touches — as a grid ortho computes, instead of
     * ELK's single row. For the implementation view, where a package's modules
     * never carry an edge, and a long row of them set the whole drawing's width.
     */
    gridLeaves?: boolean;
}

/** Target width-to-height ratio when packing. */
const PACKING_ASPECT = '1.6';

/** Gaps between the cells of a grid. */
const GRID_GAP_X = 40;
const GRID_GAP_Y = 20;

export interface LaidOutPort {
    id: string;
    label: string;
    x: number;
    y: number;
    /** Top-left of the label's box, relative to the port, as ELK placed it. */
    labelX?: number;
    labelY?: number;
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
    /** The box ELK reserved for the label, when ELK placed it (root coordinates). */
    labelBox?: { x: number; y: number; width: number; height: number };
}

export interface LaidOutDiagram {
    width: number;
    height: number;
    nodes: LaidOutNode[];
    edges: LaidOutEdge[];
}

/** Contents ortho places itself (see `gridLeaves`), relative to the container's top-left. */
type Grid = Array<{ node: GraphNode; x: number; y: number }>;

interface Conversion {
    packing: boolean;
    /** Present when innermost containers are gridded; collects each one's grid. */
    grids?: Map<string, Grid>;
    /** Every node an edge touches. */
    endpoints: Set<string>;
}

/**
 * Room round a container's contents. The extra at the top leaves room for its
 * title; a 3-D node also has to clear its depth edge, drawn inside its bounds.
 */
function paddingOf(n: GraphNode): { top: number; left: number; bottom: number; right: number } {
    return n.shape === 'node3d'
        ? { top: 45 + NODE_DEPTH, left: 20, bottom: 20, right: 20 + NODE_DEPTH }
        : { top: 45, left: 25, bottom: 25, right: 25 };
}

/**
 * A container's contents as a grid, in declaration order: one row of up to
 * three, else as square as it goes, each box centred in its cell.
 */
function gridOf(n: GraphNode, children: GraphNode[]): { cells: Grid; width: number; height: number } {
    const columns = children.length <= 3 ? children.length : Math.ceil(Math.sqrt(children.length));
    const widths = Array.from({ length: columns }, (_, column) =>
        Math.max(...children.filter((_, i) => i % columns === column).map(child => child.width)));
    const heights = Array.from({ length: Math.ceil(children.length / columns) }, (_, row) =>
        Math.max(...children.slice(row * columns, (row + 1) * columns).map(child => child.height)));
    const before = (sizes: number[], index: number, gap: number): number =>
        sizes.slice(0, index).reduce((sum, size) => sum + size + gap, 0);
    const pad = paddingOf(n);
    return {
        cells: children.map((node, i) => {
            const column = i % columns;
            const row = Math.floor(i / columns);
            return {
                node,
                x: pad.left + before(widths, column, GRID_GAP_X) + (widths[column] - node.width) / 2,
                y: pad.top + before(heights, row, GRID_GAP_Y) + (heights[row] - node.height) / 2
            };
        }),
        width: pad.left + before(widths, columns, GRID_GAP_X) - GRID_GAP_X + pad.right,
        height: pad.top + before(heights, heights.length, GRID_GAP_Y) - GRID_GAP_Y + pad.bottom
    };
}

function toElkNode(n: GraphNode, conversion: Conversion): ElkNode {
    const { packing, grids, endpoints } = conversion;
    const elkNode: ElkNode = {
        id: n.id,
        layoutOptions: { 'elk.portConstraints': 'FREE' }
    };
    const children = n.children ?? [];
    if (grids && children.length > 0
        && children.every(c => !c.children?.length && c.ports.length === 0 && !endpoints.has(c.id))) {
        // ELK sees a plain box of the grid's size; flattening fills it in.
        const grid = gridOf(n, children);
        grids.set(n.id, grid.cells);
        elkNode.width = grid.width;
        elkNode.height = grid.height;
    } else if (children.length > 0) {
        elkNode.children = children.map(child => toElkNode(child, conversion));
        const pad = paddingOf(n);
        elkNode.layoutOptions!['elk.padding'] = `[top=${pad.top},left=${pad.left},bottom=${pad.bottom},right=${pad.right}]`;
        elkNode.layoutOptions!['elk.spacing.nodeNode'] = packing ? '24' : '40';
        if (packing) {
            // Packing lays out each level on its own, so every container
            // names the algorithm for its children.
            elkNode.layoutOptions!['elk.algorithm'] = 'rectpacking';
            elkNode.layoutOptions!['elk.aspectRatio'] = PACKING_ASPECT;
        }
    } else {
        elkNode.width = n.width;
        elkNode.height = n.height;
    }
    if (n.ports.length > 0) {
        // ELK places each port's label outside the box beside its port, flips
        // it to the other side when a neighbouring label would collide, and
        // grows the box until the ports fit.
        const options = elkNode.layoutOptions!;
        options['elk.portLabels.placement'] = 'OUTSIDE';
        options['elk.nodeSize.constraints'] = 'PORTS PORT_LABELS MINIMUM_SIZE';
        options['elk.nodeSize.minimum'] = `(${n.width}, ${n.height})`;
        elkNode.ports = n.ports.map(p => ({
            id: p.id,
            width: PORT_SIZE,
            height: PORT_SIZE,
            labels: [{
                id: `${p.id}@label`,
                text: p.label,
                width: measureText(p.label, PORT_LABEL_FONT) + 2,
                height: PORT_LABEL_HEIGHT
            }],
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
    const packing = options.algorithm === 'rectpacking';
    const conversion: Conversion = {
        packing,
        grids: options.gridLeaves ? new Map() : undefined,
        endpoints: new Set(graph.edges.flatMap(e => [e.sourceId, e.targetId]))
    };
    const elkGraph: ElkNode = {
        id: 'root',
        layoutOptions: packing
            ? { 'elk.algorithm': 'rectpacking', 'elk.aspectRatio': PACKING_ASPECT, 'elk.spacing.nodeNode': '30' }
            : {
                'elk.algorithm': 'layered',
                'elk.direction': options.direction ?? 'DOWN',
                'elk.hierarchyHandling': 'INCLUDE_CHILDREN',
                'elk.spacing.nodeNode': '50',
                'elk.layered.spacing.nodeNodeBetweenLayers': '70',
                'elk.spacing.edgeNode': '30',
                'elk.spacing.edgeEdge': '20',
                'elk.edgeRouting': options.edgeRouting === 'STRAIGHT' ? 'POLYLINE' : 'ORTHOGONAL'
            },
        children: graph.nodes.map(n => toElkNode(n, conversion)),
        edges: graph.edges.map(e => ({
            id: e.id,
            sources: [e.sourcePortId ?? e.sourceId],
            targets: [e.targetPortId ?? e.targetId],
            // Wire labels on the physical view (interface names), «include» on
            // the use case view and «import» on the implementation view are
            // real ELK labels, so the layout keeps them clear of boxes and
            // other lines; a midpoint label lands wherever the route happens to
            // be busiest, or on the arrowhead of a short edge.
            labels: e.label && (e.kind === 'connection' || e.kind === 'include' || e.kind === 'import')
                ? [{
                    id: `${e.id}@label`,
                    text: e.label,
                    // A keyword is drawn with its guillemets; size it so.
                    width: measureText(e.kind === 'connection' ? e.label : `«${e.label}»`, EDGE_LABEL_FONT) + 4,
                    height: EDGE_LABEL_HEIGHT
                }]
                : undefined
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
                    y: p.y ?? 0,
                    labelX: p.labels?.[0]?.x,
                    labelY: p.labels?.[0]?.y
                })),
                x,
                y,
                width: c.width ?? 80,
                height: c.height ?? 40
            });
            for (const cell of conversion.grids?.get(c.id) ?? []) {
                nodes.push({
                    id: cell.node.id,
                    shape: cell.node.shape,
                    stereotype: cell.node.stereotype,
                    name: cell.node.name,
                    compartments: cell.node.compartments,
                    ports: [],
                    hasChildren: false,
                    x: x + cell.x,
                    y: y + cell.y,
                    width: cell.node.width,
                    height: cell.node.height
                });
            }
            flatten(c.children, x, y);
        }
    };
    flatten(result.children, 0, 0);

    const absolutePosition = new Map(nodes.map(n => [n.id, { x: n.x, y: n.y }]));

    // ELK reports each edge relative to the node that contains it, and names
    // that node in `container`. Usually that is the least common ancestor of the
    // two ends — but an edge from a box's own port to one of its children lives
    // inside that box, which no ancestor walk finds. The container is
    // authoritative; the ancestor walk is only a fallback.
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
        const container = (e as { container?: string }).container;
        const offset = container !== undefined
            ? absolutePosition.get(container) ?? { x: 0, y: 0 }
            : source ? edgeOffset(source.sourceId, source.targetId) : { x: 0, y: 0 };
        const label = e.labels?.[0];
        return {
            id: e.id,
            kind: source?.kind ?? 'connection',
            sourceId: source?.sourceId ?? '',
            targetId: source?.targetId ?? '',
            label: source?.label,
            points: rawPoints.map(p => ({ x: p.x + offset.x, y: p.y + offset.y })),
            labelBox: label?.x !== undefined && label.y !== undefined
                ? { x: label.x + offset.x, y: label.y + offset.y, width: label.width ?? 0, height: label.height ?? 0 }
                : undefined
        };
    });

    const laidOut = {
        width: result.width ?? 400,
        height: result.height ?? 300,
        nodes,
        edges
    };
    return options.edgeRouting === 'STRAIGHT' ? straightenEdges(laidOut) : laidOut;
}
