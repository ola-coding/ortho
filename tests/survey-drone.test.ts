import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, expect, it } from 'vitest';
import { createSysmlServices } from '../src/parser/sysml-module.js';
import { parseSysmlFiles } from '../src/parser/parse.js';
import type { Model } from '../src/generated/ast.js';
import type { GraphNode } from '../src/model/graph.js';
import { extractUseCaseGraph } from '../src/diagrams/use-case.js';
import { extractLogicalGraph } from '../src/diagrams/logical.js';
import { extractImplementationGraph } from '../src/diagrams/implementation.js';
import { extractPhysicalGraph } from '../src/diagrams/physical.js';
import { extractDeploymentGraph } from '../src/diagrams/deployment.js';
import { extractSequenceModel } from '../src/diagrams/sequence.js';
import { diagramTypes } from '../src/pipeline.js';

const BASE = resolve(dirname(fileURLToPath(import.meta.url)), '../examples/survey-drone');

/** One file per view. */
const ALL = [
    'use-case.sysml', 'logical.sysml', 'implementation.sysml',
    'physical.sysml', 'deployment.sysml', 'process.sysml'
];

/** The two views whose file declares everything it draws. */
const SELF_CONTAINED = ['use-case.sysml', 'logical.sysml'];

/** Every box, nested ones included. */
function allNodes(nodes: GraphNode[]): GraphNode[] {
    return nodes.flatMap(n => [n, ...allNodes(n.children ?? [])]);
}

async function parseSet(...files: string[]): Promise<Model> {
    const services = createSysmlServices();
    const { model } = await parseSysmlFiles(services.Sysml, files.map(f => resolve(BASE, f)));
    return model;
}

describe('survey drone example model', () => {
    it('parses and links all six files as one workspace', async () => {
        const model = await parseSet(...ALL);
        expect(model.packages.map(p => p.name)).toEqual([
            'UseCases', 'Functions', 'Software', 'Hardware', 'Deployment', 'SurveyMissionScenario'
        ]);
    });

    it('each self-contained view parses on its own', async () => {
        for (const file of SELF_CONTAINED) {
            await expect(parseSet(file)).resolves.toBeDefined();
        }
    });

    it('use case view: one operator, three use cases, surveying includes both', async () => {
        const graph = extractUseCaseGraph(await parseSet('use-case.sysml'));
        const boundaries = graph.nodes.filter(n => n.shape === 'boundary');
        expect(boundaries.map(b => b.name)).toEqual(['SurveyDroneSystem']);
        expect(boundaries[0].children!.map(c => c.name).sort()).toEqual([
            'followProgress', 'planSurvey', 'surveyArea'
        ]);

        expect(graph.nodes.filter(n => n.shape === 'actor').map(n => n.name)).toEqual(['Operator']);

        const includes = graph.edges.filter(e => e.kind === 'include');
        expect(includes.map(e => e.targetId).sort())
            .toEqual(['UseCases::followProgress', 'UseCases::planSurvey']);

        // The operator comes through the shared use case def, so they are a
        // primary actor of all three: planning and watching stand on their own
        // as well as inside a survey.
        const associations = graph.edges.filter(e => e.kind === 'association');
        expect(associations.filter(e => e.sourceId === 'actor:UseCases::Operator')).toHaveLength(3);
    });

    it('logical view: a strict capability tree naming nothing that realizes a function', async () => {
        const graph = extractLogicalGraph(await parseSet('logical.sysml'));
        expect(graph.edges).toHaveLength(graph.nodes.length - 1);
        expect(graph.nodes.length).toBeGreaterThan(20);

        const names = graph.nodes.map(n => n.name);
        expect(names).toContain('avoidObstacles');
        expect(names).toContain('maintainWorldModel');
        // Every onboard module answers to a function here — health monitoring
        // and flight recording included.
        expect(names).toContain('monitorHealth');
        expect(names).toContain('recordFlightData');
        // Nothing that performs a function appears on this view. (`supplyPower`
        // is a function; `LiPoBattery` is the thing that does it.)
        for (const realization of [
            'LiPoBattery', 'GpuComputer', 'ControlComputer',
            'RoutePlanner', 'ZenohRouter', 'MissionExecutor'
        ]) {
            expect(names).not.toContain(realization);
        }

        const parents = new Set(graph.edges.map(e => e.targetId));
        expect(graph.nodes.filter(n => !parents.has(n.id)).map(n => n.name))
            .toEqual(['PerformAerialSurvey']);

        // `action survey : PerformAerialSurvey` is what the realizing parts
        // point into. It is one use of the tree, not a second tree.
        expect(names).not.toContain('survey');
    });

    it('implementation view: each module lists the functions it realizes', async () => {
        const graph = extractImplementationGraph(await parseSet('implementation.sysml', 'logical.sysml'));
        const modules = new Map(allNodes(graph.nodes).map(n => [n.name, n]));
        const compartment = (name: string, title: string) =>
            modules.get(name)!.compartments.find(c => c.title === title)?.lines ?? [];

        expect(compartment('VideoStreamer', 'perform actions')).toEqual(['streamVideo']);
        expect(compartment('SurveyPlanner', 'perform actions'))
            .toEqual(['planMission', 'presentInformation', 'acceptOperatorInput']);

        // A platform module realizes nothing of the function tree: it is what
        // the modules above it stand on. It still has an API of its own.
        for (const platform of ['RealTimeKernel', 'MinimalLinux', 'ZenohRouter', 'SensorDrivers']) {
            expect(compartment(platform, 'perform actions'), platform).toEqual([]);
        }
        expect(compartment('RealTimeKernel', 'ports')).toEqual(['scheduler : SchedulerApi']);

        // The containers publish and subscribe: one topic, two ends.
        expect(compartment('VideoStreamer', 'ports')).toEqual(['video : VideoTopic']);
        expect(compartment('DataLogger', 'ports')).toEqual(['health : ~HealthTopic', 'video : ~VideoTopic']);

        // The function tree is a companion file, not part of this view.
        expect(graph.nodes.map(n => n.name)).not.toContain('Functions');
    });

    it('implementation view: the firmware stack and the Linux stack never import each other', async () => {
        const graph = extractImplementationGraph(await parseSet('implementation.sysml', 'logical.sysml'));
        const imports = graph.edges.map(e => `${e.sourceId} -> ${e.targetId}`).sort();
        expect(imports).toEqual([
            'Software::Flight -> Software::FlightPlatform',
            'Software::Ground -> Software::Middleware',
            'Software::Middleware -> Software::LinuxPlatform',
            'Software::Onboard -> Software::Middleware',
            'Software::Onboard::Autonomy -> Software::Onboard::Perception'
        ]);

        // The two stacks meet over MAVLink at runtime, never through a
        // compile-time dependency: the firmware knows nothing of Zenoh or
        // Linux, and nothing in the Linux stack builds against the firmware.
        const firmware = ['Software::Flight', 'Software::FlightPlatform'];
        for (const edge of graph.edges) {
            expect(firmware.includes(edge.sourceId)).toBe(firmware.includes(edge.targetId));
        }
    });

    it('physical view: the product as nested parts, each radio wired on its own', async () => {
        const graph = extractPhysicalGraph(await parseSet('physical.sysml', 'logical.sysml'));
        expect(graph.nodes.map(n => n.name)).toEqual(['system : SurveyDroneSystem']);
        const system = graph.nodes[0];
        expect(system.children!.map(c => c.name)).toEqual([
            'drone : SurveyDrone', 'ground : GroundStation'
        ]);
        expect(system.children![0].children).toHaveLength(12);
        expect(graph.edges).toHaveLength(18);
        expect(graph.edges.every(e => e.kind === 'connection')).toBe(true);

        // The same radio type is fitted at both ends: two boxes, and the
        // camera's video reaches the companion computer, not the tablet.
        expect(graph.edges).toContainEqual(expect.objectContaining({
            sourcePortId: 'Hardware::system.drone.camera#videoOut',
            targetPortId: 'Hardware::system.drone.companion#cameraIn'
        }));
        expect(graph.edges).toContainEqual(expect.objectContaining({
            sourcePortId: 'Hardware::system.ground.tablet#network',
            targetPortId: 'Hardware::system.ground.radio#ethernet'
        }));

        // The air-to-ground link is the one connection typed by an interface.
        expect(graph.edges.filter(e => e.label === 'RadioLink')).toEqual([
            expect.objectContaining({
                sourcePortId: 'Hardware::system.drone#rfLink',
                targetPortId: 'Hardware::system.ground#rfLink'
            })
        ]);
    });

    it('physical view: a part inherits what its definition performs', async () => {
        const graph = extractPhysicalGraph(await parseSet('physical.sysml', 'logical.sysml'));
        const boxes = new Map(allNodes(graph.nodes).map(n => [n.name, n]));

        // The perform sits on BrushlessMotor; the box is the usage.
        expect(boxes.get('motors : BrushlessMotor [4]')!.compartments)
            .toEqual([{ title: 'perform actions', lines: ['produceThrust'] }]);
        // Both computers provide computing: realization is many to many.
        expect(boxes.get('fc : ControlComputer')!.compartments[0].lines).toEqual(['provideComputing']);
        expect(boxes.get('companion : GpuComputer')!.compartments[0].lines).toEqual(['provideComputing']);
        // The radio carries what the software says; it performs nothing.
        expect(boxes.get('radio : IpRadio')!.compartments).toEqual([]);
    });

    it('deployment view: every module hosted, on three computers in two devices', async () => {
        const graph = extractDeploymentGraph(
            await parseSet('deployment.sysml', 'implementation.sysml', 'physical.sysml', 'logical.sysml')
        );
        expect(graph.edges).toHaveLength(0);

        expect(graph.nodes.map(n => n.name)).toEqual(['system : SurveyDroneSystem']);
        const system = graph.nodes[0];
        const drone = system.children!.find(c => c.name === 'drone : SurveyDrone')!;
        expect(drone.children!.map(c => c.id).sort()).toEqual([
            'Hardware::SurveyDrone::companion', 'Hardware::SurveyDrone::fc'
        ]);
        expect(system.children!.map(c => c.id)).toContain('device:Hardware::GroundStation');

        const hosts: typeof graph.nodes = [];
        const walk = (nodes: typeof graph.nodes): void => nodes.forEach(n => {
            if (n.shape === 'node3d') {
                hosts.push(n);
            }
            walk(n.children ?? []);
        });
        walk(graph.nodes);
        expect(hosts).toHaveLength(3);
        // Every box inside a host is a deployed program, so none carries a tag.
        expect(hosts.flatMap(h => h.children!).every(c => c.stereotype === '')).toBe(true);

        // The flight computer runs the firmware and nothing else — no
        // container, no router, not a line of the Linux stack.
        const fc = hosts.find(n => n.id === 'Hardware::SurveyDrone::fc')!;
        expect(fc.children!.map(c => c.name).sort()).toEqual([
            'control : FlightControl',
            'estimator : StateEstimator',
            'hal : HardwareAbstraction',
            'mavlink : MavlinkInterface',
            'mission : MissionExecutor',
            'rtos : RealTimeKernel',
            'sensors : SensorDrivers'
        ]);

        // A module missing from this view would be a program with nowhere to
        // run, so the count is the whole of implementation.sysml's `sw`.
        const placed = hosts.flatMap(n => n.children!.map(c => c.name));
        expect(placed).toHaveLength(24);
        expect(new Set(placed).size).toBe(24);
    });

    it('process view: a survey mission, uploaded over the MAVLink mission protocol', async () => {
        const sequence = extractSequenceModel(await parseSet('process.sysml', 'implementation.sysml', 'logical.sysml'));
        expect(sequence.lifelines).toHaveLength(7);
        // The operator is untyped in the model: a person, drawn as a stick figure.
        expect(sequence.lifelines.filter(l => l.actor).map(l => l.label)).toEqual(['operator']);
        expect(sequence.messages[0].label).toBe('markArea');

        // The upload handshake runs in MAVLink's own order, vehicle-driven.
        const labels = sequence.messages.map(m => m.label);
        const handshake = [
            'missionCount : MissionCount', 'requestItem : MissionRequestInt',
            'missionItem : MissionItemInt', 'missionAck : MissionAck'
        ];
        const positions = handshake.map(l => labels.indexOf(l));
        expect(positions.every(i => i >= 0)).toBe(true);
        expect([...positions].sort((a, b) => a - b)).toEqual(positions);

        // Self-messages: the work nobody has to be asked for.
        const selfMessages = sequence.messages.filter(m => m.sourceId === m.targetId);
        expect(selfMessages.map(m => m.label))
            .toEqual(['computeSurveyLegs', 'flySurveyLegs', 'returnToLaunch']);
    });

    it('renders all six views end-to-end', async () => {
        const jobs: Array<[keyof typeof diagramTypes, string[]]> = [
            ['use-case', ['use-case.sysml']],
            ['logical', ['logical.sysml']],
            ['implementation', ['implementation.sysml', 'logical.sysml']],
            ['physical', ['physical.sysml', 'logical.sysml']],
            ['deployment', ['deployment.sysml', 'implementation.sysml', 'physical.sysml', 'logical.sysml']],
            ['process', ['process.sysml', 'implementation.sysml', 'logical.sysml']]
        ];
        for (const [diagram, files] of jobs) {
            const type = diagramTypes[diagram];
            const { svg } = await type.render(await parseSet(...files), {
                heading: `${type.view} — Survey Drone`,
                source: files[0]
            });
            expect(svg.startsWith('<svg'), diagram).toBe(true);
            expect(svg, diagram).toContain(type.view);
        }
    });
});
