import { readFile } from 'node:fs/promises';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, expect, it } from 'vitest';
import { parseHelper } from 'langium/test';
import { createSysmlServices } from '../src/parser/sysml-module.js';
import type { Model } from '../src/generated/ast.js';
import { extractLogicalGraph } from '../src/diagrams/logical.js';
import { layoutGraph } from '../src/layout/elk-layout.js';
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
            await layoutGraph(graph, { direction: 'DOWN' }),
            { heading: 'Logical view — DroneFunctions', source: 'fixtures/logical.sysml' }
        );
        expect(svg).toContain('<svg');
        expect(svg).toContain('rx="11"');
        expect(svg).not.toContain('marker-end');
        expect(svg).not.toContain('marker-start');
    });
});
