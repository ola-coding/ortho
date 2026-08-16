import { readFile } from 'node:fs/promises';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, expect, it } from 'vitest';
import { parseHelper } from 'langium/test';
import { createSysmlServices } from '../src/parser/sysml-module.js';
import { isAllocationUsage } from '../src/generated/ast.js';
import type { Model, PackageDecl } from '../src/generated/ast.js';
import { extractPackageGraph } from '../src/diagrams/package.js';
import { extractAllocationGraph } from '../src/diagrams/allocation.js';
import { layoutGraph } from '../src/layout/elk-layout.js';
import { renderSvg } from '../src/render/svg-renderer.js';

const services = createSysmlServices();
const parse = parseHelper<Model>(services.Sysml);

async function parseOk(text: string) {
    const document = await parse(text, { validation: true });
    const errors = (document.diagnostics ?? []).filter(d => d.severity === 1);
    expect(errors, errors.map(e => e.message).join('; ')).toHaveLength(0);
    return document;
}

async function parseExample(): Promise<Model> {
    const text = await readFile(resolve(dirname(fileURLToPath(import.meta.url)), 'fixtures/phase3-deployment.sysml'), 'utf-8');
    return (await parseOk(text)).parseResult.value;
}

describe('Phase 3 grammar', () => {
    it('parses allocate with relatively qualified connector ends', async () => {
        const model = await parseExample();
        const software = model.packages[0].members.find(
            (m): m is PackageDecl => 'name' in m && m.name === 'Software'
        ) as PackageDecl;
        const allocations = software.members.filter(isAllocationUsage);
        expect(allocations).toHaveLength(4);
        expect(allocations[0].source.segments.map(s => s.ref?.name)).toEqual(['flightStack', 'attitudeControl']);
        expect(allocations[0].target.segments[0].ref?.name).toBe('fc');
    });
});

describe('package diagram extraction', () => {
    it('nests packages and resolves import edges', async () => {
        const graph = extractPackageGraph(await parseExample());
        expect(graph.nodes).toHaveLength(1);
        const root = graph.nodes[0];
        expect(root.shape).toBe('package');
        expect(root.children!.map(c => c.name).sort()).toEqual(['Logical', 'Physical', 'Requirements', 'Software']);

        const imports = graph.edges.map(e => `${e.sourceId} -> ${e.targetId}`).sort();
        expect(imports).toEqual([
            'DroneSystem::Logical -> DroneSystem::Requirements',
            'DroneSystem::Physical -> DroneSystem::Logical',
            'DroneSystem::Software -> DroneSystem::Logical',
            'DroneSystem::Software -> DroneSystem::Physical'
        ]);
        expect(graph.edges.every(e => e.kind === 'import')).toBe(true);
    });
});

describe('allocation diagram extraction', () => {
    it('creates one node per allocation participant and dashed allocate edges', async () => {
        const graph = extractAllocationGraph(await parseExample());
        const names = graph.nodes.map(n => n.name).sort();
        expect(names).toEqual([
            'attitudeControl', 'cc : CompanionComputer', 'fc : FlightController', 'navigation', 'perception', 'telemetry'
        ]);
        expect(graph.edges).toHaveLength(4);
        expect(graph.edges.every(e => e.kind === 'allocate')).toBe(true);
        const toFc = graph.edges.filter(e => e.targetId === 'DroneSystem::Physical::fc');
        expect(toFc.map(e => e.sourceId).sort()).toEqual([
            'DroneSystem::Software::flightStack::attitudeControl',
            'DroneSystem::Software::flightStack::telemetry'
        ]);
    });

    it('renders both phase 3 diagrams end-to-end', async () => {
        const model = await parseExample();

        const title = { heading: 'Development View — DroneSystem', source: 'examples/phase3-deployment.sysml' };
        const packageSvg = renderSvg(await layoutGraph(extractPackageGraph(model)), title);
        expect(packageSvg).toContain('&#171;import&#187;');

        const allocationSvg = renderSvg(await layoutGraph(extractAllocationGraph(model)), { ...title, heading: 'Physical View — DroneSystem' });
        expect(allocationSvg).toContain('&#171;allocate&#187;');
        expect(allocationSvg).toContain('fc : FlightController');
    });
});
