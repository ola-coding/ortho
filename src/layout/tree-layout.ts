import type { DiagramGraph, GraphEdge, GraphNode } from '../model/graph.js';
import type { LaidOutDiagram, LaidOutEdge, LaidOutNode } from './elk-layout.js';

/** Clearance around the drawing; matches ELK's default root padding. */
const MARGIN = 12;
/** Horizontal gap between sibling subtrees. */
const SIBLING_GAP = 28;
/** Gap between a parent and the row of children below it; the bus runs at its middle. */
const LEVEL_GAP = 44;
/** Gap between a parent and the first leaf listed below it. */
const LIST_TOP_GAP = 14;
/** Vertical gap between listed leaves. */
const LIST_GAP = 10;
/** Distance of a list's spine from its parent's left edge. */
const SPINE_INSET = 18;
/** Distance from the spine to a listed leaf's left edge. */
const BRANCH = 16;

type Point = { x: number; y: number };

interface Measure {
    width: number;
    height: number;
    /** Offset of the node's own box from the left edge of its subtree. */
    boxLeft: number;
    /** Offset of the children's row from the left edge of the subtree. */
    rowLeft: number;
    /** Children listed vertically beneath the node rather than spread in a row. */
    listed: boolean;
}

function withoutRepeats(points: Point[]): Point[] {
    return points.filter((p, i) => i === 0 || p.x !== points[i - 1].x || p.y !== points[i - 1].y);
}

/**
 * Lays out a capability tree as a work-breakdown chart, deterministically and
 * without ELK. A family whose children are all leaves is listed vertically
 * beneath its parent, hanging off a spine. Any other family is spread in a
 * row, with a single stem dropping to a bus that feeds every child. The
 * parent stands over its middle child when the row has one, and over the
 * middle of the row when it does not. Listing the lowest level is
 * what keeps a broad tree page-shaped: a row holding every leaf is a strip.
 *
 * The graph is expected to be a forest. An edge that would give a function a
 * second parent, or close a cycle, is drawn as a straight line but takes no
 * part in the layout.
 */
export function layoutTree(graph: DiagramGraph): LaidOutDiagram {
    const byId = new Map(graph.nodes.map(n => [n.id, n]));
    const children = new Map<string, string[]>();
    const parentOf = new Map<string, string>();
    const treeEdge = new Map<string, GraphEdge>();
    const extra: GraphEdge[] = [];

    const closesCycle = (parent: string, child: string): boolean => {
        for (let p: string | undefined = parent; p; p = parentOf.get(p)) {
            if (p === child) {
                return true;
            }
        }
        return false;
    };

    for (const edge of graph.edges) {
        const { sourceId: parent, targetId: child } = edge;
        if (!byId.has(parent) || !byId.has(child) || parentOf.has(child) || closesCycle(parent, child)) {
            extra.push(edge);
            continue;
        }
        parentOf.set(child, parent);
        children.set(parent, [...(children.get(parent) ?? []), child]);
        treeEdge.set(child, edge);
    }

    const kids = (id: string): string[] => children.get(id) ?? [];
    const measures = new Map<string, Measure>();

    const measure = (id: string): Measure => {
        const node = byId.get(id)!;
        const list = kids(id);
        let m: Measure;
        if (list.length === 0) {
            m = { width: node.width, height: node.height, boxLeft: 0, rowLeft: 0, listed: false };
        } else if (list.every(k => kids(k).length === 0)) {
            const leaves = list.map(k => byId.get(k)!);
            list.forEach(measure);
            const listWidth = SPINE_INSET + BRANCH + Math.max(...leaves.map(l => l.width));
            const listHeight = leaves.reduce((sum, l) => sum + l.height, 0) + LIST_GAP * (leaves.length - 1);
            m = {
                width: Math.max(node.width, listWidth),
                height: node.height + LIST_TOP_GAP + listHeight,
                boxLeft: 0,
                rowLeft: 0,
                listed: true
            };
        } else {
            const row = list.map(measure);
            const rowWidth = row.reduce((sum, r) => sum + r.width, 0) + SIBLING_GAP * (row.length - 1);
            const first = byId.get(list[0])!;
            const last = byId.get(list[list.length - 1])!;
            const firstCentre = row[0].boxLeft + first.width / 2;
            const lastCentre = rowWidth - row[row.length - 1].width + row[row.length - 1].boxLeft + last.width / 2;
            // An odd row has a middle child, and the stem drops straight onto
            // it: centring on the row instead is off by however much the box
            // widths differ, which reads as a wobble. An even row has no
            // middle child, so the parent stays over the middle of the row.
            const middle = (row.length - 1) / 2;
            const stemAt = row.length % 2 === 1
                ? row.slice(0, middle).reduce((sum, r) => sum + r.width + SIBLING_GAP, 0)
                    + row[middle].boxLeft + byId.get(list[middle])!.width / 2
                : (firstCentre + lastCentre) / 2;
            let boxLeft = stemAt - node.width / 2;
            let rowLeft = 0;
            if (boxLeft < 0) {
                rowLeft = -boxLeft;
                boxLeft = 0;
            }
            m = {
                width: Math.max(rowLeft + rowWidth, boxLeft + node.width),
                height: node.height + LEVEL_GAP + Math.max(...row.map(r => r.height)),
                boxLeft,
                rowLeft,
                listed: false
            };
        }
        measures.set(id, m);
        return m;
    };

    const nodes: LaidOutNode[] = [];
    const edges: LaidOutEdge[] = [];
    const position = new Map<string, Point>();

    const branch = (childId: string, points: Point[]): void => {
        const edge = treeEdge.get(childId)!;
        edges.push({
            id: edge.id,
            kind: edge.kind,
            sourceId: edge.sourceId,
            targetId: edge.targetId,
            label: edge.label,
            points: withoutRepeats(points)
        });
    };

    const place = (id: string, left: number, top: number): void => {
        const node: GraphNode = byId.get(id)!;
        const m = measures.get(id)!;
        const x = left + m.boxLeft;
        nodes.push({
            id: node.id,
            shape: node.shape,
            stereotype: node.stereotype,
            name: node.name,
            compartments: node.compartments,
            ports: [],
            hasChildren: false,
            x,
            y: top,
            width: node.width,
            height: node.height
        });
        position.set(id, { x, y: top });

        const bottom = top + node.height;
        const list = kids(id);
        if (m.listed) {
            const spineX = x + SPINE_INSET;
            let y = bottom + LIST_TOP_GAP;
            for (const leafId of list) {
                const leaf = byId.get(leafId)!;
                place(leafId, spineX + BRANCH, y);
                const middle = y + leaf.height / 2;
                branch(leafId, [{ x: spineX, y: bottom }, { x: spineX, y: middle }, { x: spineX + BRANCH, y: middle }]);
                y += leaf.height + LIST_GAP;
            }
        } else if (list.length > 0) {
            const busY = bottom + LEVEL_GAP / 2;
            const rowTop = bottom + LEVEL_GAP;
            const lefts: number[] = [];
            let childLeft = left + m.rowLeft;
            for (const childId of list) {
                lefts.push(childLeft);
                childLeft += measures.get(childId)!.width + SIBLING_GAP;
            }
            const centreOf = (i: number): number =>
                lefts[i] + measures.get(list[i])!.boxLeft + byId.get(list[i])!.width / 2;
            // Over an odd row the stem is taken from the middle child itself,
            // not from the parent's centre, so the two meet exactly rather than
            // to within rounding, and the middle branch is one straight line.
            const stemX = list.length % 2 === 1 ? centreOf((list.length - 1) / 2) : x + node.width / 2;
            list.forEach((childId, i) => {
                place(childId, lefts[i], rowTop);
                const childX = centreOf(i);
                branch(childId, [{ x: stemX, y: bottom }, { x: stemX, y: busY }, { x: childX, y: busY }, { x: childX, y: rowTop }]);
            });
        }
    };

    const roots = graph.nodes.filter(n => !parentOf.has(n.id)).map(n => n.id);
    let left = MARGIN;
    let height = 0;
    for (const root of roots) {
        const m = measure(root);
        place(root, left, MARGIN);
        left += m.width + SIBLING_GAP * 2;
        height = Math.max(height, m.height);
    }

    for (const edge of extra) {
        const s = position.get(edge.sourceId);
        const t = position.get(edge.targetId);
        if (!s || !t) {
            continue;
        }
        const source = byId.get(edge.sourceId)!;
        const target = byId.get(edge.targetId)!;
        edges.push({
            id: edge.id,
            kind: edge.kind,
            sourceId: edge.sourceId,
            targetId: edge.targetId,
            label: edge.label,
            points: [{ x: s.x + source.width / 2, y: s.y + source.height }, { x: t.x + target.width / 2, y: t.y }]
        });
    }

    return {
        width: roots.length > 0 ? left - SIBLING_GAP * 2 + MARGIN : 2 * MARGIN,
        height: height + 2 * MARGIN,
        nodes,
        edges
    };
}
