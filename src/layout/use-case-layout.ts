import type { DiagramGraph, GraphEdge, GraphNode } from '../model/graph.js';
import { USAGE_RADIUS } from '../model/graph.js';
import type { LaidOutDiagram, LaidOutEdge, LaidOutNode } from './elk-layout.js';

/** Clearance around the drawing; matches ELK's default root padding. */
const MARGIN = 12;
/** Between the subject and the use cases, and between the use cases and the actors. */
const ROW_GAP = 56;
/** Between two use cases side by side. */
const BOX_GAP = 36;
/** Between a use case and one it includes: room for «include» and its arrow. */
const INCLUDE_GAP = 84;
/** Between subjects, and between actors, side by side. */
const SIDE_GAP = 40;
/** A line to an actor stops this far above its head. */
const HEAD_GAP = 3;
/** How far below the row an «include» between use cases that are not neighbours runs. */
const DETOUR = 16;

type Point = { x: number; y: number };

const centreX = (n: { x: number; width: number }): number => n.x + n.width / 2;
const clamp = (v: number, lo: number, hi: number): number => Math.max(lo, Math.min(hi, v));

/** Lexical order of two keys. */
function compareKeys(a: number[], b: number[]): number {
    const i = a.findIndex((v, k) => v !== b[k]);
    return i === -1 ? 0 : a[i] - b[i];
}

/**
 * Lays out the use case view as OMG's examples are drawn (§7.25), and without
 * ELK: the subject above its use cases, each joined to it by a short line, and
 * the actors below them. OMG draws one use case, its actors beside it; a view
 * of several, sharing the subject and the actors, keeps every line clear of
 * every box only with the two facing the row of use cases from opposite
 * sides. Each use case stands between the use cases it includes, so an
 * «include» joins neighbours.
 */
export function layoutUseCases(graph: DiagramGraph): LaidOutDiagram {
    const byId = new Map(graph.nodes.map(n => [n.id, n]));
    const useCases = graph.nodes.filter(n => n.keyword === 'use case' || n.keyword === 'use case def');
    const subjects = graph.nodes.filter(n => n.keyword === 'subject');
    const actors = graph.nodes.filter(n => n.shape === 'actor');
    const includes = graph.edges.filter(e => e.kind === 'include');
    const lines = graph.edges.filter(e => e.kind === 'association');

    // The row: each use case with half of what it includes on either side, so
    // a use case including two stands between them.
    const included = new Set(includes.map(e => e.targetId));
    const placed = new Set<string>();
    const sequence = (useCase: GraphNode): GraphNode[] => {
        placed.add(useCase.id);
        const targets = includes.filter(e => e.sourceId === useCase.id)
            .map(e => byId.get(e.targetId)!)
            .filter(target => !placed.has(target.id));
        targets.forEach(target => placed.add(target.id));
        const half = Math.floor(targets.length / 2);
        return [...targets.slice(0, half).flatMap(sequence), useCase, ...targets.slice(half).flatMap(sequence)];
    };
    const row: GraphNode[] = [];
    for (const root of [...useCases.filter(u => !included.has(u.id)), ...useCases]) {
        if (!placed.has(root.id)) {
            row.push(...sequence(root));
        }
    }

    const joins = (a: string, b: string): boolean =>
        includes.some(e => (e.sourceId === a && e.targetId === b) || (e.sourceId === b && e.targetId === a));
    const subjectHeight = Math.max(0, ...subjects.map(s => s.height));
    const rowTop = MARGIN + (subjects.length > 0 ? subjectHeight + ROW_GAP : 0);
    const rowHeight = Math.max(0, ...row.map(u => u.height));
    const at = new Map<string, Point>();
    let x = 0;
    row.forEach((useCase, i) => {
        at.set(useCase.id, { x, y: rowTop + (rowHeight - useCase.height) / 2 });
        x += useCase.width + (row[i + 1] && joins(useCase.id, row[i + 1].id) ? INCLUDE_GAP : BOX_GAP);
    });

    // The middle of the use cases a subject or an actor is joined to.
    const middle = (id: string): number => {
        const joined = lines
            .filter(e => e.sourceId === id || e.targetId === id)
            .map(e => byId.get(e.sourceId === id ? e.targetId : e.sourceId)!)
            .filter(n => at.has(n.id));
        return joined.length === 0 ? 0
            : joined.reduce((sum, n) => sum + at.get(n.id)!.x + n.width / 2, 0) / joined.length;
    };
    // Side by side, in the given order, each centred on what it is joined to
    // as far as the one before it allows.
    const spread = (nodes: GraphNode[], top: (n: GraphNode) => number, key: (n: GraphNode) => number[]): void => {
        let right = -Infinity;
        for (const n of [...nodes].sort((a, b) => compareKeys(key(a), key(b)))) {
            const left = Math.max(middle(n.id) - n.width / 2, right + SIDE_GAP);
            at.set(n.id, { x: left, y: top(n) });
            right = left + n.width;
        }
    };
    spread(subjects, s => MARGIN + subjectHeight - s.height, s => [middle(s.id)]);
    // A primary actor before a secondary one: the extractor points a
    // secondary actor's lines at the actor.
    const secondary = (id: string): number => (lines.some(e => e.targetId === id) ? 1 : 0);
    spread(actors, () => rowTop + rowHeight + ROW_GAP, a => [secondary(a.id), middle(a.id)]);

    const drawn = graph.nodes.filter(n => at.has(n.id));
    if (drawn.length === 0) {
        return { width: 2 * MARGIN, height: 2 * MARGIN, nodes: [], edges: [] };
    }
    const shift = MARGIN - Math.min(...drawn.map(n => at.get(n.id)!.x));
    const nodes: LaidOutNode[] = drawn.map(n => ({
        id: n.id,
        shape: n.shape,
        keyword: n.keyword,
        name: n.name,
        compartments: n.compartments,
        ports: [],
        hasChildren: false,
        x: at.get(n.id)!.x + shift,
        y: at.get(n.id)!.y,
        width: n.width,
        height: n.height
    }));
    const box = new Map(nodes.map(n => [n.id, n]));

    /**
     * An «include» runs straight to a neighbour, or round below the row to any
     * other. A subject's or actor's line meets a use case's top or bottom edge,
     * straight up or down where the two overlap, leaving the subject's bottom
     * edge or the actor's head.
     */
    const route = (e: GraphEdge, source: LaidOutNode, target: LaidOutNode): Point[] => {
        if (e.kind === 'include') {
            const neighbours = Math.abs(row.findIndex(u => u.id === source.id) - row.findIndex(u => u.id === target.id)) === 1;
            if (neighbours) {
                const y = source.y + source.height / 2;
                const rightwards = centreX(target) > centreX(source);
                return [
                    { x: rightwards ? source.x + source.width : source.x, y },
                    { x: rightwards ? target.x : target.x + target.width, y }
                ];
            }
            const below = rowTop + rowHeight + DETOUR;
            return [
                { x: centreX(source), y: source.y + source.height },
                { x: centreX(source), y: below },
                { x: centreX(target), y: below },
                { x: centreX(target), y: target.y + target.height }
            ];
        }
        const fromUseCase = source.shape !== 'actor' && source.keyword !== 'subject';
        const [useCase, other] = fromUseCase ? [source, target] : [target, source];
        const onUseCase = (x: number): number => clamp(x, useCase.x + USAGE_RADIUS, useCase.x + useCase.width - USAGE_RADIUS);
        let ends: Point[];
        if (other.shape === 'actor') {
            const head = { x: centreX(other), y: other.y - HEAD_GAP };
            ends = [head, { x: onUseCase(head.x), y: useCase.y + useCase.height }];
        } else {
            const start = { x: clamp(centreX(useCase), other.x + USAGE_RADIUS, other.x + other.width - USAGE_RADIUS), y: other.y + other.height };
            ends = [start, { x: onUseCase(start.x), y: useCase.y }];
        }
        // In the edge's own direction, from its source to its target.
        return fromUseCase ? ends.reverse() : ends;
    };

    const edges = graph.edges.flatMap((e): LaidOutEdge[] => {
        const source = box.get(e.sourceId);
        const target = box.get(e.targetId);
        return source && target
            ? [{ id: e.id, kind: e.kind, sourceId: e.sourceId, targetId: e.targetId, label: e.label, points: route(e, source, target) }]
            : [];
    });

    return {
        width: Math.max(...nodes.map(n => n.x + n.width)) + MARGIN,
        height: Math.max(...nodes.map(n => n.y + n.height)) + MARGIN,
        nodes,
        edges
    };
}
