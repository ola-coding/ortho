import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, expect, it } from 'vitest';
import { createSysmlServices } from '../src/parser/sysml-module.js';
import { parseSysmlFiles } from '../src/parser/parse.js';
import type { Model } from '../src/generated/ast.js';
import { extractUseCaseGraph } from '../src/diagrams/use-case.js';
import { extractLogicalGraph } from '../src/diagrams/logical.js';
import { extractImplementationGraph } from '../src/diagrams/implementation.js';
import { extractPhysicalGraph } from '../src/diagrams/physical.js';
import { extractDeploymentGraph } from '../src/diagrams/deployment.js';
import { extractSequenceModel } from '../src/diagrams/sequence.js';
import { diagramTypes } from '../src/pipeline.js';

const BASE = resolve(dirname(fileURLToPath(import.meta.url)), '../examples/auv-system');

/** One file per view. */
const ALL = [
    'use-case.sysml', 'logical.sysml', 'implementation.sysml',
    'physical.sysml', 'deployment.sysml', 'process.sysml'
];

/** The four views whose file declares everything it draws. */
const SELF_CONTAINED = ['use-case.sysml', 'logical.sysml', 'implementation.sysml', 'physical.sysml'];

async function parseSet(...files: string[]): Promise<Model> {
    const services = createSysmlServices();
    const { model } = await parseSysmlFiles(services.Sysml, files.map(f => resolve(BASE, f)));
    return model;
}

describe('AUV system example model', () => {
    it('parses and links all six files as one workspace', async () => {
        const model = await parseSet(...ALL);
        expect(model.packages.map(p => p.name)).toEqual([
            'UseCases', 'Functions', 'Software', 'Hardware', 'Deployment', 'FollowPersonScenario'
        ]);
    });

    it('each self-contained view parses on its own', async () => {
        for (const file of SELF_CONTAINED) {
            await expect(parseSet(file)).resolves.toBeDefined();
        }
    });

    it('use case view: five use cases, two actors, follow-person includes', async () => {
        const graph = extractUseCaseGraph(await parseSet('use-case.sysml'));
        const boundaries = graph.nodes.filter(n => n.shape === 'boundary');
        expect(boundaries.map(b => b.name)).toEqual(['AuvSystem']);
        expect(boundaries[0].children!.map(c => c.name).sort()).toEqual([
            'avoidObstacles', 'followPerson', 'launchAircraft', 'selectTarget', 'viewLiveVideo'
        ]);

        expect(graph.nodes.filter(n => n.shape === 'actor').map(n => n.name).sort())
            .toEqual(['Pilot', 'TrackedPerson']);

        const includes = graph.edges.filter(e => e.kind === 'include');
        expect(includes.map(e => e.targetId).sort())
            .toEqual(['UseCases::avoidObstacles', 'UseCases::selectTarget']);
    });

    it('logical view: a strict capability tree, larger than the coffee machine', async () => {
        const graph = extractLogicalGraph(await parseSet('logical.sysml'));
        expect(graph.edges).toHaveLength(graph.nodes.length - 1);
        expect(graph.nodes.length).toBeGreaterThan(20);

        const names = graph.nodes.map(n => n.name);
        expect(names).toContain('stabiliseAttitude');
        expect(names).toContain('trackTarget');
        // Nothing that performs a function appears here. (`rechargeBattery` is
        // a function; `LiPoFlightBattery` is the thing that does it.)
        for (const realization of [
            'ThreeAxisGimbal', 'DualBandTransceiver', 'LiPoFlightBattery',
            'FlightControlStack', 'NavigationEngine'
        ]) {
            expect(names).not.toContain(realization);
        }

        const parents = new Set(graph.edges.map(e => e.targetId));
        expect(graph.nodes.filter(n => !parents.has(n.id)).map(n => n.name)).toEqual(['OperateAircraft']);
    });

    it('implementation view: the ground app depends on the airborne packages, never the reverse', async () => {
        const graph = extractImplementationGraph(await parseSet('implementation.sysml'));
        const imports = graph.edges.map(e => `${e.sourceId} -> ${e.targetId}`).sort();
        expect(imports).toEqual([
            'Software::Ground -> Software::Flight',
            'Software::Ground -> Software::Link',
            'Software::Ground -> Software::Perception'
        ]);
        expect(imports.some(i => i.includes('-> Software::Ground'))).toBe(false);
    });

    it('physical view: the product as nested parts, each radio wired on its own', async () => {
        const graph = extractPhysicalGraph(await parseSet('physical.sysml'));
        // One top-level assembly; the Propeller supertype is part of nothing.
        expect(graph.nodes.map(n => n.name)).toEqual(['AuvSystem']);
        const system = graph.nodes[0];
        expect(system.children!.map(c => c.name)).toEqual([
            'aircraft : Aircraft', 'controller : RadioController',
            'batteries : LiPoFlightBattery [1..3]', 'charger : MultiBayCharger'
        ]);
        expect(system.children![0].children).toHaveLength(11);
        expect(graph.edges).toHaveLength(18);
        expect(graph.edges.every(e => e.kind === 'connection')).toBe(true);

        // The same transceiver type is fitted at both ends: two boxes, and the
        // camera's video reaches the aircraft's radio, not the controller's.
        expect(graph.edges).toContainEqual(expect.objectContaining({
            sourcePortId: 'Hardware::AuvSystem.aircraft.camera#videoOut',
            targetPortId: 'Hardware::AuvSystem.aircraft.radio#video'
        }));
        expect(graph.edges).toContainEqual(expect.objectContaining({
            sourcePortId: 'Hardware::AuvSystem.controller.radio#video',
            targetPortId: 'Hardware::AuvSystem.controller.display#videoIn'
        }));

        // The air-to-ground link is the one connection typed by an interface.
        expect(graph.edges.filter(e => e.label === 'RadioLink')).toEqual([
            expect.objectContaining({
                sourcePortId: 'Hardware::AuvSystem.aircraft#rfLink',
                targetPortId: 'Hardware::AuvSystem.controller#rfLink'
            })
        ]);
    });

    it('deployment view: seven programs across three hosts, drawn as containment', async () => {
        const graph = extractDeploymentGraph(
            await parseSet('deployment.sysml', 'implementation.sysml', 'physical.sysml')
        );
        expect(graph.edges).toHaveLength(0);
        expect(graph.nodes.map(n => n.id).sort()).toEqual([
            'Hardware::Aircraft::camera',
            'Hardware::Aircraft::fc',
            'Hardware::Aircraft::radio',
            'Hardware::AuvSystem::controller'
        ]);

        const fc = graph.nodes.find(n => n.id === 'Hardware::Aircraft::fc')!;
        expect(fc.children!.map(c => c.name).sort()).toEqual([
            'flightControl : FlightControlStack',
            'navigation : NavigationEngine',
            'perception : PerceptionPipeline',
            'targetTracking : TargetTrackingEngine'
        ]);

        const placed = graph.nodes.flatMap(n => n.children!.map(c => c.name));
        expect(placed).toHaveLength(7);
        expect(new Set(placed).size).toBe(7);
    });

    it('process view: follow-person scenario with obstacle avoidance', async () => {
        const sequence = extractSequenceModel(await parseSet('process.sysml', 'implementation.sysml'));
        expect(sequence.lifelines).toHaveLength(6);
        expect(sequence.messages[0].label).toBe('launch');

        // A self-message: the flight stack manoeuvres without asking anyone.
        const selfMessages = sequence.messages.filter(m => m.sourceId === m.targetId);
        expect(selfMessages.map(m => m.label)).toEqual(['avoidanceManeuver']);
    });

    it('renders all six views end-to-end', async () => {
        const jobs: Array<[keyof typeof diagramTypes, string[]]> = [
            ['use-case', ['use-case.sysml']],
            ['logical', ['logical.sysml']],
            ['implementation', ['implementation.sysml']],
            ['physical', ['physical.sysml']],
            ['deployment', ['deployment.sysml', 'implementation.sysml', 'physical.sysml']],
            ['process', ['process.sysml', 'implementation.sysml']]
        ];
        for (const [diagram, files] of jobs) {
            const type = diagramTypes[diagram];
            const { svg } = await type.render(await parseSet(...files), {
                heading: `${type.view} — AUV System`,
                source: files[0]
            });
            expect(svg.startsWith('<svg'), diagram).toBe(true);
            expect(svg, diagram).toContain(type.view);
        }
    });
});
