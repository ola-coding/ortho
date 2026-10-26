import { readFile } from 'node:fs/promises';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, expect, it } from 'vitest';
import { parseHelper } from 'langium/test';
import { createSysmlServices } from '../src/parser/sysml-module.js';
import { isAllocationUsage } from '../src/generated/ast.js';
import type { Member, Model, PackageDecl } from '../src/generated/ast.js';
import { extractImplementationGraph } from '../src/diagrams/implementation.js';
import { extractDeploymentGraph } from '../src/diagrams/deployment.js';
import { layoutGraph } from '../src/layout/elk-layout.js';
import { renderSvg } from '../src/render/svg-renderer.js';
import { measureText } from '../src/render/text-metrics.js';

/** Exposes the whole test model, as a view of all of it would. */
const everything = (model: Model): [Member[], Model] => [model.packages, model];

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
        expect(allocations[0].source!.segments.map(s => s.ref?.name)).toEqual(['flightStack', 'attitudeControl']);
        expect(allocations[0].target!.segments[0].ref?.name).toBe('fc');
    });
});

describe('implementation diagram extraction', () => {
    it('nests packages and resolves import edges', async () => {
        const graph = extractImplementationGraph((await parseExample()).packages);
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
        // Each imports a package's members, `Pkg::*` (§8.2.3.5).
        expect(graph.edges.every(e => e.label === '«import»*')).toBe(true);
    });

    it('draws the modules a package declares inside it, and no instances', async () => {
        const graph = extractImplementationGraph((await parseExample()).packages);
        const physical = graph.nodes[0].children!.find(c => c.name === 'Physical')!;
        // `fc` and `cc` are package-level part usages — instances, not modules.
        expect(physical.children!.map(c => c.name).sort()).toEqual(['CompanionComputer', 'FlightController']);
        // A module is a definition: a square box headed «part def».
        expect(physical.children!.every(c => c.shape === 'definition' && c.keyword === 'part def')).toBe(true);
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
        const graph = extractImplementationGraph(document.parseResult.value.packages);
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

    it('keeps a package name wider than its grid inside the package', async () => {
        const document = await parse(`
            package S {
                package FirmwareModulesForTheFlightControllerBoard {
                    part def A;
                }
            }
        `, { validation: true });
        const graph = extractImplementationGraph(document.parseResult.value.packages);
        const laidOut = await layoutGraph(graph, { direction: 'DOWN', gridLeaves: true });

        const pkg = laidOut.nodes.find(n => n.name === 'FirmwareModulesForTheFlightControllerBoard')!;
        const module = laidOut.nodes.find(n => n.name === 'A')!;
        expect(pkg.width).toBeGreaterThan(measureText(pkg.name, 12, 'bold'));
        // The grid centres under the name.
        expect(module.x + module.width / 2).toBeCloseTo(pkg.x + pkg.width / 2);
    });
});

describe('deployment diagram extraction', () => {
    it('lists what each host runs in its relationships compartment, and draws no edges', async () => {
        const graph = extractDeploymentGraph(...everything(await parseExample()));
        expect(graph.edges).toHaveLength(0);

        expect(graph.nodes.map(n => n.name).sort()).toEqual(['cc : CompanionComputer', 'fc : FlightController']);
        expect(graph.nodes.every(n => n.shape === 'usage' && n.keyword === 'part')).toBe(true);
        // Nothing is drawn inside a host: software is not its member.
        expect(graph.nodes.every(n => !n.children)).toBe(true);

        const runs = Object.fromEntries(graph.nodes.map(n => [n.name, n.compartments]));
        expect(runs['fc : FlightController']).toEqual([{
            title: 'relationships',
            lines: ['allocated flightStack.attitudeControl', 'allocated flightStack.telemetry']
        }]);
        expect(runs['cc : CompanionComputer']).toEqual([{
            title: 'relationships',
            lines: ['allocated flightStack.navigation', 'allocated flightStack.perception']
        }]);
    });

    it('lists a program allocated to two hosts on both, once each', async () => {
        const document = await parse(`
            package S {
                part def Computer;
                part a : Computer;
                part b : Computer;
                part sw { part logger; }
                allocate sw.logger to a;
                allocate sw.logger to b;
                allocate sw.logger to a;
            }
        `, { validation: true });
        const graph = extractDeploymentGraph(...everything(document.parseResult.value));
        expect(graph.nodes.map(n => [n.name, n.compartments[0].lines])).toEqual([
            ['a : Computer', ['allocated sw.logger']],
            ['b : Computer', ['allocated sw.logger']]
        ]);
    });

    it('keeps a name wider than the contents inside its box', async () => {
        const document = await parse(`
            package S {
                part def ElectronicsBayWithAVeryLongName {
                    part flightControllerComputerWithALongName : Computer;
                }
                part def Computer;
                part bay : ElectronicsBayWithAVeryLongName;
                part sw { part a; }
                allocate sw.a to bay.flightControllerComputerWithALongName;
            }
        `, { validation: true });
        const laidOut = await layoutGraph(extractDeploymentGraph(...everything(document.parseResult.value)), { algorithm: 'rectpacking' });

        const host = laidOut.nodes.find(n => n.name === 'flightControllerComputerWithALongName : Computer')!;
        const frame = laidOut.nodes.find(n => n.name === 'bay : ElectronicsBayWithAVeryLongName')!;
        for (const box of [host, frame]) {
            expect(box.width).toBeGreaterThan(measureText(box.name, 12, 'bold'));
        }
        // The host sits inside its device, as the part it is.
        expect(host.x).toBeGreaterThan(frame.x);
        expect(host.x + host.width).toBeLessThan(frame.x + frame.width);
        expect(host.compartments[0].lines).toEqual(['allocated sw.a']);
    });

    it('renders both diagrams end-to-end', async () => {
        const model = await parseExample();

        const title = { heading: 'Implementation view — DroneSystem', source: 'fixtures/deployment.sysml' };
        const implementationSvg = renderSvg(await layoutGraph(extractImplementationGraph(model.packages)), title);
        expect(implementationSvg).toContain('&#171;import&#187;*');

        const deploymentSvg = renderSvg(
            await layoutGraph(extractDeploymentGraph(...everything(model))),
            { ...title, heading: 'Deployment view — DroneSystem' }
        );
        // Hosts list what they run, and nothing on this view is an arrow.
        expect(deploymentSvg).toContain('fc : FlightController');
        expect(deploymentSvg).toContain('>relationships</text>');
        expect(deploymentSvg).toContain('>allocated flightStack.attitudeControl</text>');
        expect(deploymentSvg).not.toContain('marker-end');
    });
});
