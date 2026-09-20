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
        // Requirements holds no module, so it is not software and not drawn;
        // the import into it goes with it.
        expect(root.children!.filter(c => c.shape === 'package').map(c => c.name).sort())
            .toEqual(['Logical', 'Physical', 'Software']);

        const imports = graph.edges.map(e => `${e.sourceId} -> ${e.targetId}`).sort();
        expect(imports).toEqual([
            'DroneSystem::Physical -> DroneSystem::Logical',
            'DroneSystem::Software -> DroneSystem::Logical',
            'DroneSystem::Software -> DroneSystem::Physical'
        ]);
        expect(graph.edges.every(e => e.kind === 'import')).toBe(true);
    });

    it('draws the modules a package declares inside it, and no instances', async () => {
        const graph = extractImplementationGraph(await parseExample());
        const physical = graph.nodes[0].children!.find(c => c.name === 'Physical')!;
        // `fc` and `cc` are package-level part usages — instances, not modules.
        expect(physical.children!.map(c => c.name).sort()).toEqual(['CompanionComputer', 'FlightController']);
        // A module box carries its name alone — no stereotype line.
        expect(physical.children!.every(c => c.stereotype === '')).toBe(true);
    });
});

describe('module APIs on the implementation view', () => {
    it('lists them in a ports compartment, conjugating the ones a module needs', async () => {
        const document = await parse(`
            package S {
                port def Api;
                part def Server { port api : Api; }
                part def Client { port api : ~Api; }
            }
        `, { validation: true });
        const graph = extractImplementationGraph(document.parseResult.value);
        const laidOut = await layoutGraph(graph, { direction: 'DOWN', gridLeaves: true });

        const server = laidOut.nodes.find(n => n.name === 'Server')!;
        const client = laidOut.nodes.find(n => n.name === 'Client')!;
        expect(server.compartments).toEqual([{ title: 'ports', lines: ['api : Api'] }]);
        expect(client.compartments).toEqual([{ title: 'ports', lines: ['api : ~Api'] }]);

        // A square on the border is for a port a wire lands on; this view
        // draws no wires between modules, and the modules stay gridded.
        expect(server.ports).toEqual([]);
        expect(server.y).toBe(client.y);
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
