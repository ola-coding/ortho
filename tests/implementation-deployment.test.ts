import { readFile } from 'node:fs/promises';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, expect, it } from 'vitest';
import { parseHelper } from 'langium/test';
import { createSysmlServices } from '../src/parser/sysml-module.js';
import { isAllocationUsage } from '../src/generated/ast.js';
import type { Model, PackageDecl } from '../src/generated/ast.js';
import { extractImplementationGraph } from '../src/diagrams/implementation.js';
import { extractDeploymentGraph } from '../src/diagrams/deployment.js';
import { layoutGraph } from '../src/layout/elk-layout.js';
import { renderSvg } from '../src/render/svg-renderer.js';

const services = createSysmlServices();
const parse = parseHelper<Model>(services.Sysml);

async function parseExample(): Promise<Model> {
    const text = await readFile(resolve(dirname(fileURLToPath(import.meta.url)), 'fixtures/deployment.sysml'), 'utf-8');
    const document = await parse(text, { validation: true });
    const errors = (document.diagnostics ?? []).filter(d => d.severity === 1);
    expect(errors, errors.map(e => e.message).join('; ')).toHaveLength(0);
    return document.parseResult.value;
}

describe('allocation grammar', () => {
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

describe('implementation diagram extraction', () => {
    it('nests packages and resolves import edges', async () => {
        const graph = extractImplementationGraph(await parseExample());
        expect(graph.nodes).toHaveLength(1);
        const root = graph.nodes[0];
        expect(root.shape).toBe('package');
        expect(root.children!.filter(c => c.shape === 'package').map(c => c.name).sort())
            .toEqual(['Logical', 'Physical', 'Requirements', 'Software']);

        const imports = graph.edges.map(e => `${e.sourceId} -> ${e.targetId}`).sort();
        expect(imports).toEqual([
            'DroneSystem::Logical -> DroneSystem::Requirements',
            'DroneSystem::Physical -> DroneSystem::Logical',
            'DroneSystem::Software -> DroneSystem::Logical',
            'DroneSystem::Software -> DroneSystem::Physical'
        ]);
        expect(graph.edges.every(e => e.kind === 'import')).toBe(true);
    });

    it('draws the modules a package declares inside it', async () => {
        const graph = extractImplementationGraph(await parseExample());
        const physical = graph.nodes[0].children!.find(c => c.name === 'Physical')!;
        expect(physical.children!.map(c => c.name).sort()).toEqual([
            'CompanionComputer', 'FlightController', 'cc : CompanionComputer', 'fc : FlightController'
        ]);
        // A module box carries its name alone — no stereotype line.
        expect(physical.children!.every(c => c.stereotype === '')).toBe(true);
    });
});

describe('deployment diagram extraction', () => {
    it('nests software inside its host instead of drawing allocation edges', async () => {
        const graph = extractDeploymentGraph(await parseExample());
        expect(graph.edges).toHaveLength(0);

        const hosts = graph.nodes.map(n => n.name).sort();
        expect(hosts).toEqual(['cc : CompanionComputer', 'fc : FlightController']);
        expect(graph.nodes.every(n => n.shape === 'node3d')).toBe(true);

        const byHost = Object.fromEntries(
            graph.nodes.map(n => [n.name, n.children!.map(c => c.name).sort()])
        );
        expect(byHost['fc : FlightController']).toEqual(['attitudeControl', 'telemetry']);
        expect(byHost['cc : CompanionComputer']).toEqual(['navigation', 'perception']);
    });

    it('places every allocated software part exactly once', async () => {
        const graph = extractDeploymentGraph(await parseExample());
        const placed = graph.nodes.flatMap(n => n.children!.map(c => c.id));
        expect(new Set(placed).size).toBe(placed.length);
        expect(placed).toHaveLength(4);
    });

    it('renders both diagrams end-to-end', async () => {
        const model = await parseExample();

        const title = { heading: 'Implementation view — DroneSystem', source: 'fixtures/deployment.sysml' };
        const implementationSvg = renderSvg(await layoutGraph(extractImplementationGraph(model)), title);
        expect(implementationSvg).toContain('&#171;import&#187;');

        const deploymentSvg = renderSvg(
            await layoutGraph(extractDeploymentGraph(model)),
            { ...title, heading: 'Deployment view — DroneSystem' }
        );
        // Hosts are solids, and nothing on this view is an arrow.
        expect(deploymentSvg).toContain('<polygon');
        expect(deploymentSvg).toContain('fc : FlightController');
        expect(deploymentSvg).not.toContain('marker-end');
    });
});
