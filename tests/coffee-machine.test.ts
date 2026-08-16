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

const BASE = resolve(dirname(fileURLToPath(import.meta.url)), '../examples/coffee-machine');

const ALL = [
    'requirements.sysml', 'system.sysml', 'mechanics.sysml', 'electronics.sysml',
    'software.sysml', 'use-cases.sysml', 'scenario-make-cappuccino.sysml'
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

describe('coffee machine example model', () => {
    it('parses and links all seven files as one workspace', async () => {
        const model = await parseSet(...ALL);
        expect(model.packages.map(p => p.name)).toEqual([
            'Requirements', 'System', 'Mechanics', 'Electronics',
            'Software', 'UseCases', 'MakeCappuccinoScenario'
        ]);
    });

    it('logical view: subsystem decomposition and requirement traces', async () => {
        const graph = extractPartDefinitionGraph(await parseSet(...DOMAINS));

        const machine = graph.nodes.find(n => n.name === 'CoffeeMachine')!;
        expect(machine.compartments[1].lines).toEqual([
            'water : WaterSubsystem', 'coffee : CoffeeSubsystem', 'frother : MilkFrother',
            'display : DisplayModule', 'controller : ControlBoard', 'psu : PowerSupply'
        ]);

        // The architecture block carries the interfaces and no internals ...
        const water = graph.nodes.find(n => n.name === 'WaterSubsystem')!;
        expect(water.compartments.flatMap(c => c.lines)).toEqual([]);
        expect(water.ports.map(p => p.label).sort()).toEqual(
            ['ctrlIn', 'hotWaterOut', 'steamOut']
        );

        // ... and the mechanical realization supplies them.
        const realization = graph.nodes.find(n => n.name === 'ThermoblockWaterSubsystem')!;
        expect(realization.compartments.flatMap(c => c.lines)).toContain('pump : WaterPump');

        // Every architecture block is realized by exactly one domain design.
        expect(graph.edges.filter(e => e.kind === 'specialization')).toHaveLength(6);
        expect(graph.edges.filter(e => e.kind === 'satisfy')).toHaveLength(4);
    });

    it('scenarios view: the three beverages under one boundary', async () => {
        const graph = extractUseCaseGraph(await parseSet('use-cases.sysml', ...DOMAINS));

        const boundary = graph.nodes.find(n => n.shape === 'boundary')!;
        expect(boundary.name).toBe('CoffeeMachine');
        expect(boundary.children!.map(c => c.name).sort()).toEqual([
            'makeCappuccino', 'makeCoffee', 'makeEspresso'
        ]);

        // one actor, associated with all three use cases through the shared def
        expect(graph.nodes.filter(n => n.shape === 'actor').map(n => n.name)).toEqual(['User']);
        expect(graph.edges.filter(e => e.kind === 'association')).toHaveLength(3);

        // a cappuccino brews a shot first
        expect(graph.edges.filter(e => e.kind === 'include')).toEqual([
            expect.objectContaining({
                sourceId: 'UseCases::makeCappuccino',
                targetId: 'UseCases::makeEspresso'
            })
        ]);
    });

    it('development view: system level over three domains, acyclic imports', async () => {
        const graph = extractPackageGraph(await parseSet(...ARCHITECTURE));
        expect(graph.nodes).toHaveLength(6);
        expect(graph.edges.map(e => `${e.sourceId} -> ${e.targetId}`).sort()).toEqual([
            'Electronics -> System',
            'Mechanics -> Requirements',
            'Mechanics -> System',
            'Software -> Requirements',
            'Software -> System',
            'UseCases -> System'
        ]);
    });

    // Scenarios are kept off the development view: a scenario is an instance of
    // behaviour, not an architectural unit, and one leaf per scenario would
    // swamp the package diagram. The dependency itself still has to be sound.
    it('the scenario depends only on the package it exercises', async () => {
        const graph = extractPackageGraph(await parseSet(...ALL));
        expect(graph.edges.filter(e => e.sourceId === 'MakeCappuccinoScenario')
            .map(e => e.targetId)).toEqual(['Software']);
        expect(graph.edges.filter(e => e.targetId === 'MakeCappuccinoScenario')).toEqual([]);
    });

    it('development view: every domain depends on the system, never the reverse', async () => {
        const graph = extractPackageGraph(await parseSet(...ALL));
        const domains = ['Mechanics', 'Electronics', 'Software'];

        // The architecture is the stable hub: domains conform to it.
        for (const domain of domains) {
            expect(graph.edges).toContainEqual(
                expect.objectContaining({ sourceId: domain, targetId: 'System' })
            );
        }
        // Stronger: the architecture depends on nothing whatsoever.
        expect(graph.edges.filter(e => e.sourceId === 'System')).toEqual([]);
    });

    it('development view: the engineering domains do not depend on each other', async () => {
        const graph = extractPackageGraph(await parseSet(...ALL));
        const domains = ['Mechanics', 'Electronics', 'Software'];
        const crossDomain = graph.edges.filter(
            e => domains.includes(e.sourceId) && domains.includes(e.targetId)
        );
        expect(crossDomain).toEqual([]);
    });

    // No per-domain view is committed — it would be a strict subset of the full
    // logical view. But each domain must still resolve and render on its own
    // against the architecture, or the domains are not really independent.
    it('each domain resolves without the other domains loaded', async () => {
        const mechanics = extractPartDefinitionGraph(
            await parseSet('mechanics.sysml', 'system.sysml', 'requirements.sysml')
        );
        expect(mechanics.nodes.map(n => n.name)).toContain('Thermoblock');
        expect(mechanics.nodes.map(n => n.name)).toContain('WaterSubsystem');
        expect(mechanics.nodes.map(n => n.name)).not.toContain('MainControlBoard');

        const electronics = extractPartDefinitionGraph(
            await parseSet('electronics.sysml', 'system.sysml')
        );
        expect(electronics.nodes.map(n => n.name)).toContain('MainControlBoard');
        expect(electronics.nodes.map(n => n.name)).not.toContain('Thermoblock');
    });

    it('physical view: five allocations onto two electronic nodes', async () => {
        const graph = extractAllocationGraph(await parseSet('software.sysml', ...DOMAINS));
        expect(graph.edges).toHaveLength(5);
        expect([...new Set(graph.edges.map(e => e.targetId))].sort()).toEqual([
            'System::CoffeeMachine::controller',
            'System::CoffeeMachine::display'
        ]);
        expect(graph.nodes.map(n => n.name)).toContain('uiApp : UiApp');
    });

    it('process view: one press produces a cappuccino', async () => {
        const sequence = extractSequenceModel(await parseSet(
            'scenario-make-cappuccino.sysml', 'software.sysml', ...DOMAINS
        ));
        expect(sequence.lifelines.map(l => l.label)).toEqual([
            'user', 'ui : UiApp', 'recipes : RecipeEngine',
            'temperature : TemperatureController', 'brew : BrewController', 'milk : MilkController'
        ]);
        expect(sequence.messages).toHaveLength(8);
        expect(sequence.messages[1].label).toBe('startBeverage : BeverageRecipe');

        const extract = sequence.messages.find(m => m.label === 'extractShot')!;
        expect(extract.sourceId).toBe(extract.targetId);
    });

    it('renders all five views end-to-end', async () => {
        const full = await parseSet(...ALL);
        for (const [name, diagramType] of Object.entries(diagramTypes)) {
            const result = await diagramType.render(full, { heading: `${diagramType.view} — test`, source: 'test' });
            expect(result.svg, name).toContain('</svg>');
        }
    });
});
