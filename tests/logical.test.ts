import { readFile } from 'node:fs/promises';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, expect, it } from 'vitest';
import { parseHelper } from 'langium/test';
import { createSysmlServices } from '../src/parser/sysml-module.js';
import type { Member, Model } from '../src/generated/ast.js';
import type { DiagramGraph, GraphEdge, GraphNode } from '../src/model/graph.js';
import { USAGE_RADIUS } from '../src/model/graph.js';
import { declaredViews, exposedElements } from '../src/model/views.js';
import { extractLogicalGraph } from '../src/diagrams/logical.js';
import { layoutTree } from '../src/layout/tree-layout.js';
import { renderSvg } from '../src/render/svg-renderer.js';

/** Exposes the whole test model, as a view of all of it would. */
const everything = (model: Model): [Member[], Model] => [model.packages, model];

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
    it('produces one box per function: «action def» for the root, «action» below it', async () => {
        const graph = extractLogicalGraph(...everything(await parseExample()));
        const [root, ...functions] = graph.nodes;
        expect([root.name, root.shape, root.keyword]).toEqual(['OperateDrone', 'definition', 'action def']);
        expect(functions.every(n => n.shape === 'usage' && n.keyword === 'action')).toBe(true);
        expect(graph.nodes.map(n => n.name)).toContain('OperateDrone');
        expect(graph.nodes.map(n => n.name)).toContain('generateThrust');
        expect(graph.nodes.map(n => n.name)).toContain('holdAltitude');
    });

    it('is a strict tree: one root, one parent per function', async () => {
        const graph = extractLogicalGraph(...everything(await parseExample()));
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
        const graph = extractLogicalGraph(...everything(await parseExample()));
        expect(graph.edges.every(e => e.kind === 'decomposition')).toBe(true);
        expect(graph.edges.every(e => e.label === undefined)).toBe(true);
    });

    it('renders each branch with a filled diamond at its parent, and no arrowhead', async () => {
        const graph = extractLogicalGraph(...everything(await parseExample()));
        const laidOut = layoutTree(graph);
        const svg = renderSvg(laidOut, { heading: 'Logical view — DroneFunctions', source: 'fixtures/logical.sysml' });
        expect(svg).toContain(`rx="${USAGE_RADIUS}"`);
        // A function is owned by its parent (§8.2.3.6): the diamond sits where
        // every branch leaves it, so siblings share one.
        expect(svg.match(/marker-start="url\(#diamond\)"/g)).toHaveLength(graph.edges.length);
        expect(svg).not.toContain('marker-end');
        const start = (targetId: string) => laidOut.edges.find(e => e.targetId === targetId)!.points[0];
        for (const parentId of new Set(graph.edges.map(e => e.sourceId))) {
            const children = graph.edges.filter(e => e.sourceId === parentId).map(e => e.targetId);
            expect(new Set(children.map(c => JSON.stringify(start(c)))).size, parentId).toBe(1);
        }
    });

    it('lays the tree out as a work-breakdown chart', async () => {
        const graph = extractLogicalGraph(...everything(await parseExample()));
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

    describe('the use cases the trees refine', () => {
        /** A model whose view exposes the given elements. */
        const refining = async (exposes: string[], refinements = ['HandleOrder to SystemUseCases::Order']) => {
            const document = await parse(`
                package SystemUseCases {
                    use case def Order;
                    use case def Return;
                }
                package Functions {
                    private import ModelingMetadata::*;
                    action def HandleOrder { action pack; action ship; }
                    action def HandleReturn { action refund; }
                    ${refinements.map(r => `#refinement dependency ${r};`).join('\n')}
                    view v : GeneralView { ${exposes.map(e => `expose ${e};`).join(' ')} }
                }
            `, { validation: true });
            const model = document.parseResult.value;
            const errors = (document.diagnostics ?? []).filter(d => d.severity === 1).map(d => d.message);
            return { errors, graph: extractLogicalGraph(exposedElements(declaredViews(model)[0]), model) };
        };
        const centre = (b: { x: number; width: number }) => b.x + b.width / 2;

        it('stands a use case above its tree, with a dashed arrow up to it', async () => {
            const { graph, errors } = await refining(['HandleOrder', 'SystemUseCases::Order']);
            expect(errors).toEqual([]);
            const laidOut = layoutTree(graph);
            const at = new Map(laidOut.nodes.map(n => [n.name, n]));
            const [root, useCase] = [at.get('HandleOrder')!, at.get('Order')!];
            expect([useCase.shape, useCase.keyword]).toEqual(['definition', 'use case def']);
            expect(useCase.y + useCase.height).toBeLessThan(root.y);
            expect(centre(useCase)).toBeCloseTo(centre(root));

            // From the client up to the supplier, straight.
            const arrow = laidOut.edges.find(e => e.kind === 'dependency')!;
            expect(arrow.points).toEqual([
                { x: centre(root), y: root.y },
                { x: centre(root), y: useCase.y + useCase.height }
            ]);
            const svg = renderSvg(laidOut, { heading: 'Functions', source: 'inline' });
            expect(svg).toContain('stroke-dasharray="6 4" marker-end="url(#openArrow)"');
        });

        it('draws neither a use case nor an arrow the view does not expose', async () => {
            const { graph } = await refining(['HandleOrder']);
            expect(graph.nodes.map(n => n.name)).toEqual(['HandleOrder', 'pack', 'ship']);
            expect(graph.edges.every(e => e.kind === 'decomposition')).toBe(true);
        });

        it('shows a use case nothing refines, last and with no arrow', async () => {
            const { graph } = await refining(['HandleOrder', 'SystemUseCases::Return', 'SystemUseCases::Order']);
            const laidOut = layoutTree(graph);
            expect(laidOut.edges.filter(e => e.kind === 'dependency').map(e => e.targetId)).toEqual(['SystemUseCases::Order']);
            const at = new Map(laidOut.nodes.map(n => [n.name, n]));
            expect(at.get('Return')!.x).toBeGreaterThan(at.get('Order')!.x + at.get('Order')!.width);
        });

        it('centres a use case over all the trees that refine it, and keeps a row of them apart', async () => {
            const both = await refining(
                ['HandleOrder', 'HandleReturn', 'SystemUseCases::Order'],
                ['HandleOrder to SystemUseCases::Order', 'HandleReturn to SystemUseCases::Order']
            );
            const shared = new Map(layoutTree(both.graph).nodes.map(n => [n.name, n]));
            expect(centre(shared.get('Order')!))
                .toBeCloseTo((centre(shared.get('HandleOrder')!) + centre(shared.get('HandleReturn')!)) / 2);

            // One tree refining both: its use cases side by side, not overlapping.
            const one = await refining(
                ['HandleOrder', 'SystemUseCases::Order', 'SystemUseCases::Return'],
                ['HandleOrder to SystemUseCases::Order, SystemUseCases::Return']
            );
            const laidOut = layoutTree(one.graph);
            expect(laidOut.edges.filter(e => e.kind === 'dependency')).toHaveLength(2);
            const row = new Map(laidOut.nodes.map(n => [n.name, n]));
            expect(row.get('Return')!.x).toBeGreaterThanOrEqual(row.get('Order')!.x + row.get('Order')!.width);
            expect(laidOut.width).toBeGreaterThanOrEqual(row.get('Return')!.x + row.get('Return')!.width);
        });

        it('fails on a dependency that names nothing, so a typo cannot pass', async () => {
            const { errors } = await refining(['HandleOrder'], ['HandleOrder to SystemUseCases::Ordr']);
            expect(errors).toEqual(["Could not resolve reference to Member named 'SystemUseCases::Ordr'."]);
        });
    });

    describe('where the stem lands', () => {
        const box = (id: string, width: number): GraphNode =>
            ({ id, shape: 'usage', keyword: 'action', name: id, compartments: [], ports: [], width, height: 40 });
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
