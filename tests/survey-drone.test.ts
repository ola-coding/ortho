import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, expect, it } from 'vitest';
import { createSysmlServices } from '../src/parser/sysml-module.js';
import { parseSysmlFiles } from '../src/parser/parse.js';
import type { Member, Model } from '../src/generated/ast.js';
import type { GraphNode } from '../src/model/graph.js';
import { extractUseCaseGraph } from '../src/diagrams/use-case.js';
import { extractLogicalGraph } from '../src/diagrams/logical.js';
import { extractImplementationGraph } from '../src/diagrams/implementation.js';
import { extractPhysicalGraph } from '../src/diagrams/physical.js';
import { extractDeploymentGraph } from '../src/diagrams/deployment.js';
import { extractSequenceModel } from '../src/diagrams/sequence.js';
import { declaredViews, diagramKind, exposedElements, viewHeading } from '../src/model/views.js';
import { renderViews } from '../src/pipeline.js';

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

/** What the first file's view exposes, and the workspace it was parsed in. */
async function viewSet(...files: string[]): Promise<[Member[], Model]> {
    const model = await parseSet(...files);
    return [exposedElements(declaredViews(model.packages[0])[0]), model];
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
        const graph = extractUseCaseGraph((await viewSet('use-case.sysml'))[0]);
        expect(graph.nodes.filter(n => n.keyword === 'use case').map(n => n.name).sort()).toEqual([
            'followProgress : SurveyTask', 'planSurvey : SurveyTask', 'surveyArea : SurveyTask'
        ]);
        const subjects = graph.nodes.filter(n => n.keyword === 'subject');
        expect(subjects.map(n => n.name)).toEqual(['SurveyDroneSystem']);
        expect(graph.edges.filter(e => e.targetId === subjects[0].id)).toHaveLength(3);

        expect(graph.nodes.filter(n => n.shape === 'actor').map(n => n.name)).toEqual(['operator : Operator']);

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
        const graph = extractLogicalGraph((await viewSet('logical.sysml'))[0]);
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
        const graph = extractImplementationGraph((await viewSet('implementation.sysml', 'logical.sysml'))[0]);
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
        const graph = extractImplementationGraph((await viewSet('implementation.sysml', 'logical.sysml'))[0]);
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
        const graph = extractPhysicalGraph((await viewSet('physical.sysml', 'logical.sysml'))[0]);
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
        expect(graph.edges.filter(e => e.label === ': RadioLink')).toEqual([
            expect.objectContaining({
                sourcePortId: 'Hardware::system.drone#rfLink',
                targetPortId: 'Hardware::system.ground#rfLink'
            })
        ]);
    });

    it('physical view: a part inherits what its definition performs', async () => {
        const graph = extractPhysicalGraph((await viewSet('physical.sysml', 'logical.sysml'))[0]);
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
        const graph = extractDeploymentGraph(...await viewSet('deployment.sysml', 'implementation.sysml', 'physical.sysml', 'logical.sysml'));
        expect(graph.edges).toHaveLength(0);

        expect(graph.nodes.map(n => n.name)).toEqual(['system : SurveyDroneSystem']);
        const system = graph.nodes[0];
        const drone = system.children!.find(c => c.name === 'drone : SurveyDrone')!;
        expect(drone.children!.map(c => c.id).sort()).toEqual([
            'Hardware::SurveyDrone::companion', 'Hardware::SurveyDrone::fc'
        ]);
        expect(system.children!.map(c => c.id)).toContain('device:Hardware::GroundStation');

        // A host is a «part» listing what is allocated to it.
        const hosts: typeof graph.nodes = [];
        const walk = (nodes: typeof graph.nodes): void => nodes.forEach(n => {
            if (n.compartments.some(c => c.title === 'relationships')) {
                hosts.push(n);
            }
            walk(n.children ?? []);
        });
        walk(graph.nodes);
        expect(hosts).toHaveLength(3);
        expect(hosts.every(h => h.keyword === 'part')).toBe(true);

        // The flight computer runs the firmware and nothing else — no
        // container, no router, not a line of the Linux stack.
        const fc = hosts.find(n => n.id === 'Hardware::SurveyDrone::fc')!;
        expect(fc.compartments[0].lines).toEqual([
            'allocated sw.mission',
            'allocated sw.estimator',
            'allocated sw.control',
            'allocated sw.sensors',
            'allocated sw.mavlink',
            'allocated sw.hal',
            'allocated sw.rtos'
        ]);

        // A module missing from this view would be a program with nowhere to
        // run, so the count is the whole of implementation.sysml's `sw`.
        const placed = hosts.flatMap(n => n.compartments.flatMap(c => c.lines));
        expect(placed).toHaveLength(24);
        expect(new Set(placed).size).toBe(24);
    });

    it('process view: a survey mission, uploaded over the MAVLink mission protocol', async () => {
        const sequence = extractSequenceModel(...await viewSet('process.sysml', 'implementation.sysml', 'logical.sysml'));
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

    it('renders all six views, each to a file named after it', async () => {
        const { rendered, skipped } = await renderViews(await parseSysmlFiles(createSysmlServices().Sysml, ALL.map(f => resolve(BASE, f))));
        expect(skipped).toEqual([]);
        expect(Object.fromEntries(rendered.map(r => [diagramKind(r.view), r.file]))).toEqual({
            'use-case': 'use-case-view-survey-drone.svg',
            logical: 'logical-view-survey-drone.svg',
            implementation: 'implementation-view-survey-drone.svg',
            physical: 'physical-view-survey-drone.svg',
            deployment: 'deployment-view-survey-drone.svg',
            process: 'process-view-survey-drone.svg'
        });
        for (const { view, result } of rendered) {
            expect(result.svg.startsWith('<svg'), view.name).toBe(true);
            expect(result.svg, view.name).toContain(viewHeading(view));
        }
        // Each diagram names the file that declares its view.
        expect(rendered.find(r => diagramKind(r.view) === 'physical')!.result.svg).toContain('examples/survey-drone/physical.sysml');
    }, 30_000);
});
