import { readFile } from 'node:fs/promises';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, expect, it } from 'vitest';
import { parseHelper } from 'langium/test';
import { createSysmlServices } from '../src/parser/sysml-module.js';
import type { Model } from '../src/generated/ast.js';
import type { DiagramGraph, GraphEdge, GraphNode } from '../src/model/graph.js';
import { extractLogicalGraph } from '../src/diagrams/logical.js';
import { layoutTree } from '../src/layout/tree-layout.js';
import { renderSvg } from '../src/render/svg-renderer.js';

const services = createSysmlServices();
const parse = parseHelper<Model>(services.Sysml);

async function parseExample(): Promise<Model> {
    const text = await readFile(resolve(dirname(fileURLToPath(import.meta.url)), 'fixtures/logical.sysml'), 'utf-8');
    const document = await parse(text, { validation: true });
    const errors = (document.diagnostics ?? []).filter(d => d.severity === 1);
    expect(errors, errors.map(e => e.message).join('; ')).toHaveLength(0);
    return document.parseResult.value;
}

describe('logical diagram extraction', () => {
    it('produces one rounded node per function', async () => {
        const graph = extractLogicalGraph(await parseExample());
        expect(graph.nodes.every(n => n.shape === 'rounded')).toBe(true);
        expect(graph.nodes.map(n => n.name)).toContain('OperateDrone');
        expect(graph.nodes.map(n => n.name)).toContain('generateThrust');
        expect(graph.nodes.map(n => n.name)).toContain('holdAltitude');
    });

    it('is a strict tree: one root, one parent per function', async () => {
        const graph = extractLogicalGraph(await parseExample());
        // A tree of n nodes has exactly n-1 branches.
        expect(graph.edges).toHaveLength(graph.nodes.length - 1);

        const parents = new Map<string, number>();
        for (const edge of graph.edges) {
            parents.set(edge.targetId, (parents.get(edge.targetId) ?? 0) + 1);
        }
        expect([...parents.values()].every(count => count === 1)).toBe(true);

        const roots = graph.nodes.filter(n => !parents.has(n.id));
        expect(roots.map(n => n.name)).toEqual(['OperateDrone']);
    });

    it('draws branches as plain decomposition lines, never arrows', async () => {
        const graph = extractLogicalGraph(await parseExample());
        expect(graph.edges.every(e => e.kind === 'decomposition')).toBe(true);
        expect(graph.edges.every(e => e.label === undefined)).toBe(true);
    });

    it('renders to SVG with no arrowheads on any branch', async () => {
        const graph = extractLogicalGraph(await parseExample());
        const svg = renderSvg(
            layoutTree(graph),
            { heading: 'Logical view — DroneFunctions', source: 'fixtures/logical.sysml' }
        );
        expect(svg).toContain('<svg');
        expect(svg).toContain('rx="11"');
        expect(svg).not.toContain('marker-end');
        expect(svg).not.toContain('marker-start');
    });

    it('lays the tree out as a work-breakdown chart', async () => {
        const graph = extractLogicalGraph(await parseExample());
        const laidOut = layoutTree(graph);
        expect(laidOut.nodes).toHaveLength(graph.nodes.length);

        const at = new Map(laidOut.nodes.map(n => [n.id, n]));
        const family = new Map<string, string[]>();
        for (const edge of graph.edges) {
            family.set(edge.sourceId, [...(family.get(edge.sourceId) ?? []), edge.targetId]);
        }
        for (const [parentId, kids] of family) {
            const parent = at.get(parentId)!;
            const boxes = kids.map(k => at.get(k)!);
            expect(boxes.every(b => b.y > parent.y + parent.height)).toBe(true);
            if (kids.every(k => !family.has(k))) {
                // The lowest level is listed: one column, in declaration order.
                expect(new Set(boxes.map(b => b.x)).size).toBe(1);
                boxes.slice(1).forEach((b, i) => expect(b.y).toBeGreaterThan(boxes[i].y));
            } else {
                // Any other family is a row. Its parent stands over the middle
                // child when there is one, and over the middle of the row when not.
                expect(new Set(boxes.map(b => b.y)).size).toBe(1);
                const centre = (b: { x: number; width: number }) => b.x + b.width / 2;
                const middle = boxes.length % 2 === 1
                    ? centre(boxes[(boxes.length - 1) / 2])
                    : (centre(boxes[0]) + centre(boxes[boxes.length - 1])) / 2;
                expect(centre(parent)).toBeCloseTo(middle, 5);
            }
        }

        // No two boxes overlap, and every branch is drawn with right angles.
        for (const a of laidOut.nodes) {
            for (const b of laidOut.nodes) {
                if (a !== b) {
                    const apart = a.x + a.width <= b.x || b.x + b.width <= a.x
                        || a.y + a.height <= b.y || b.y + b.height <= a.y;
                    expect(apart, `${a.name} overlaps ${b.name}`).toBe(true);
                }
            }
        }
        for (const edge of laidOut.edges) {
            edge.points.slice(1).forEach((p, i) => {
                expect(p.x === edge.points[i].x || p.y === edge.points[i].y).toBe(true);
            });
        }
    });

    describe('where the stem lands', () => {
        const box = (id: string, width: number): GraphNode =>
            ({ id, shape: 'rounded', stereotype: '', name: id, compartments: [], ports: [], width, height: 40 });
        const decompose = (parent: string, child: string): GraphEdge =>
            ({ id: `${parent}-${child}`, kind: 'decomposition', sourceId: parent, targetId: child });

        // `a` carries a wide listed leaf, so the row is lopsided: the middle of
        // the row is 67 px from the middle child, which the check below confirms
        // before relying on it.
        const lopsided = (extra: GraphNode[]): DiagramGraph => ({
            nodes: [box('root', 100), box('a', 40), box('a1', 150), box('b', 200), box('c', 60), ...extra],
            edges: [decompose('root', 'a'), decompose('a', 'a1'), decompose('root', 'b'), decompose('root', 'c'),
                ...extra.map(n => decompose('root', n.id))]
        });
        const centres = (graph: DiagramGraph) => {
            const laidOut = layoutTree(graph);
            const at = new Map(laidOut.nodes.map(n => [n.id, n]));
            return { laidOut, centre: (id: string) => at.get(id)!.x + at.get(id)!.width / 2 };
        };

        it('drops straight onto the middle child of an odd row', () => {
            const { laidOut, centre } = centres(lopsided([]));
            expect(Math.abs((centre('a') + centre('c')) / 2 - centre('b'))).toBeGreaterThan(50);

            expect(centre('root')).toBeCloseTo(centre('b'), 9);
            const stem = laidOut.edges.find(e => e.targetId === 'b')!;
            expect(new Set(stem.points.map(p => p.x)).size).toBe(1);
        });

        it('stays over the middle of an even row, which has no middle child', () => {
            const { centre } = centres(lopsided([box('d', 80)]));
            expect(centre('root')).toBeCloseTo((centre('a') + centre('d')) / 2, 9);
        });
    });
});
