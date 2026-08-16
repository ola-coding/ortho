import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, expect, it } from 'vitest';
import { createSysmlServices } from '../src/parser/sysml-module.js';
import { parseSysmlFiles } from '../src/parser/parse.js';
import type { Model } from '../src/generated/ast.js';
import { extractPartDefinitionGraph } from '../src/diagrams/part-definition.js';
import { extractUseCaseGraph } from '../src/diagrams/use-case.js';
import { extractPackageGraph } from '../src/diagrams/package.js';
import { extractAllocationGraph } from '../src/diagrams/allocation.js';
import { extractSequenceModel } from '../src/diagrams/sequence.js';
import { diagramTypes } from '../src/pipeline.js';

const BASE = resolve(dirname(fileURLToPath(import.meta.url)), '../examples/auv-system');

const ALL = [
    'requirements.sysml', 'system.sysml', 'mechanics.sysml', 'electronics.sysml',
    'software.sysml', 'use-cases.sysml', 'scenario-follow-person.sysml'
];

/** The system level plus the two domains that realise it physically. */
const DOMAINS = [
    'system.sysml', 'mechanics.sysml', 'electronics.sysml', 'requirements.sysml'
];

/** What the development view renders: architectural packages, no scenarios. */
const ARCHITECTURE = ALL.filter(f => !f.startsWith('scenario-'));

async function parseSet(...files: string[]): Promise<Model> {
    const services = createSysmlServices();
    const { model } = await parseSysmlFiles(services.Sysml, files.map(f => resolve(BASE, f)));
    return model;
}

describe('AUV system example model', () => {
    it('parses and links all seven files as one workspace', async () => {
        const model = await parseSet(...ALL);
        expect(model.packages.map(p => p.name)).toEqual([
            'Requirements', 'System', 'Mechanics', 'Electronics',
            'Software', 'UseCases', 'FollowPersonScenario'
        ]);
    });

    it('logical view: system composition, realization and requirement traces', async () => {
        const graph = extractPartDefinitionGraph(await parseSet(...DOMAINS));

        const system = graph.nodes.find(n => n.name === 'AuvSystem')!;
        expect(system.compartments[0].lines).toEqual([
            'aircraft : Aircraft', 'controller : RadioController',
            'batteries : FlightBattery [1..3]', 'charger : BatteryCharger'
        ]);

        // The assembly composes architecture blocks, never a domain realization —
        // that is what keeps System from depending on Mechanics or Electronics.
        const aircraft = graph.nodes.find(n => n.name === 'Aircraft')!;
        expect(aircraft.compartments[1].lines).toContain('props : Propeller [4]');
        expect(aircraft.ports.map(p => p.label).sort()).toEqual(['batteryBay', 'rfLink']);

        // ... and the folding blade is the mechanical design that realizes it.
        expect(graph.edges).toContainEqual(expect.objectContaining({
            kind: 'specialization',
            sourceId: 'System::Propeller',
            targetId: 'Mechanics::FoldingPropeller'
        }));

        expect(graph.edges.filter(e => e.kind === 'composition')).toHaveLength(18);
        expect(graph.edges.filter(e => e.kind === 'connection')).toHaveLength(18);
        expect(graph.edges.filter(e => e.kind === 'satisfy')).toHaveLength(5);
        // Every architecture block is realized by exactly one domain design.
        expect(graph.edges.filter(e => e.kind === 'specialization')).toHaveLength(15);

        // The transceiver is shared: composed by both the aircraft and the controller
        const transceiverCompositions = graph.edges.filter(
            e => e.kind === 'composition' && e.targetId === 'System::RadioTransceiver'
        );
        expect(transceiverCompositions.map(e => e.sourceId).sort()).toEqual(
            ['System::Aircraft', 'System::RadioController']
        );
    });

    it('logical view: leaves no node unconnected', async () => {
        const graph = extractPartDefinitionGraph(await parseSet(...DOMAINS));
        const touched = new Set(graph.edges.flatMap(e => [e.sourceId, e.targetId]));
        expect(graph.nodes.filter(n => !touched.has(n.id)).map(n => n.name)).toEqual([]);
    });

    it('scenarios view: system boundary, two actors, follow-person includes', async () => {
        const graph = extractUseCaseGraph(await parseSet('use-cases.sysml', ...DOMAINS));
        const boundary = graph.nodes.find(n => n.shape === 'boundary')!;
        expect(boundary.name).toBe('AuvSystem');
        expect(boundary.children!.map(c => c.name).sort()).toEqual(
            ['avoidObstacles', 'followPerson', 'launchAircraft', 'selectTarget', 'viewLiveVideo']
        );
        expect(graph.nodes.filter(n => n.shape === 'actor').map(n => n.name).sort()).toEqual(['Pilot', 'TrackedPerson']);
        expect(graph.edges.filter(e => e.kind === 'association')).toHaveLength(5);
        const includes = graph.edges.filter(e => e.kind === 'include');
        expect(includes.map(e => e.targetId).sort()).toEqual(
            ['UseCases::avoidObstacles', 'UseCases::selectTarget']
        );
    });

    it('development view: system level over three domains, acyclic imports', async () => {
        const graph = extractPackageGraph(await parseSet(...ARCHITECTURE));
        expect(graph.nodes).toHaveLength(6);
        expect(graph.edges.map(e => `${e.sourceId} -> ${e.targetId}`).sort()).toEqual([
            'Electronics -> Requirements',
            'Electronics -> System',
            'Mechanics -> System',
            'Software -> Requirements',
            'Software -> System',
            'System -> Requirements',
            'UseCases -> System'
        ]);
    });

    it('development view: every domain depends on the system, never the reverse', async () => {
        const graph = extractPackageGraph(await parseSet(...ALL));
        const domains = ['Mechanics', 'Electronics', 'Software'];

        for (const domain of domains) {
            expect(graph.edges).toContainEqual(
                expect.objectContaining({ sourceId: domain, targetId: 'System' })
            );
        }
        expect(graph.edges.filter(e => e.sourceId === 'System' && domains.includes(e.targetId)))
            .toEqual([]);

        // System-level requirements are satisfied by the aircraft as a whole, so
        // unlike the coffee machine this architecture does trace to Requirements —
        // which stays the universal sink either way.
        expect(graph.edges.filter(e => e.sourceId === 'System').map(e => e.targetId))
            .toEqual(['Requirements']);
        expect(graph.edges.filter(e => e.sourceId === 'Requirements')).toEqual([]);
    });

    it('development view: the engineering domains do not depend on each other', async () => {
        const graph = extractPackageGraph(await parseSet(...ALL));
        const domains = ['Mechanics', 'Electronics', 'Software'];
        expect(graph.edges.filter(
            e => domains.includes(e.sourceId) && domains.includes(e.targetId)
        )).toEqual([]);
    });

    // system.sysml carries the aircraft-level satisfy traces, so requirements.sysml
    // travels with it — unlike the coffee machine, whose System is a pure sink.
    it('each domain resolves without the other domains loaded', async () => {
        const mechanics = extractPartDefinitionGraph(
            await parseSet('mechanics.sysml', 'system.sysml', 'requirements.sysml')
        );
        expect(mechanics.nodes.map(n => n.name)).toContain('FoldingPropeller');
        expect(mechanics.nodes.map(n => n.name)).not.toContain('SocFlightController');

        const electronics = extractPartDefinitionGraph(
            await parseSet('electronics.sysml', 'system.sysml', 'requirements.sysml')
        );
        expect(electronics.nodes.map(n => n.name)).toContain('SocFlightController');
        expect(electronics.nodes.map(n => n.name)).not.toContain('FoldingPropeller');
    });

    it('physical view: seven allocations onto aircraft modules and the controller', async () => {
        const graph = extractAllocationGraph(await parseSet('software.sysml', ...DOMAINS));
        expect(graph.edges).toHaveLength(7);
        const targets = new Set(graph.edges.map(e => e.targetId));
        expect([...targets].sort()).toEqual([
            'System::Aircraft::camera', 'System::Aircraft::fc', 'System::Aircraft::radio',
            'System::AuvSystem::controller'
        ]);
        expect(graph.nodes.map(n => n.name)).toContain('pilotApp : PilotApp');
        expect(graph.nodes.map(n => n.name)).toContain('controller : RadioController');
    });

    it('process view: follow-person scenario with obstacle avoidance', async () => {
        const sequence = extractSequenceModel(await parseSet(
            'scenario-follow-person.sysml', 'software.sysml', ...DOMAINS
        ));
        expect(sequence.lifelines.map(l => l.label)).toEqual([
            'pilot', 'rcApp : PilotApp', 'flightControl : FlightControlStack',
            'cameraCtrl : CameraController', 'targetTracking : TargetTrackingEngine',
            'perception : PerceptionPipeline'
        ]);
        expect(sequence.messages).toHaveLength(9);
        expect(sequence.messages[4].label).toBe('trackTarget : TargetBox');
        const self = sequence.messages[7];
        expect(self.label).toBe('avoidanceManeuver');
        expect(self.sourceId).toBe(self.targetId);
    });

    it('renders all five views end-to-end', async () => {
        const full = await parseSet(...ALL);
        for (const [name, diagramType] of Object.entries(diagramTypes)) {
            const result = await diagramType.render(full, { heading: `${diagramType.view} — test`, source: 'test' });
            expect(result.svg, name).toContain('</svg>');
        }
    });
});
