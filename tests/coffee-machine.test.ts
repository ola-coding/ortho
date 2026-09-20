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

const BASE = resolve(dirname(fileURLToPath(import.meta.url)), '../examples/coffee-machine');

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

describe('coffee machine example model', () => {
    it('parses and links all six files as one workspace', async () => {
        const model = await parseSet(...ALL);
        expect(model.packages.map(p => p.name)).toEqual([
            'UseCases', 'Functions', 'Software', 'Hardware', 'Deployment', 'MakeCappuccinoScenario'
        ]);
    });

    it('each self-contained view parses on its own', async () => {
        // The point of one file per view: four of the six need no companion.
        for (const file of SELF_CONTAINED) {
            await expect(parseSet(file)).resolves.toBeDefined();
        }
    });

    it('use case view: three beverages and a service task under one boundary', async () => {
        const graph = extractUseCaseGraph(await parseSet('use-case.sysml'));
        const boundaries = graph.nodes.filter(n => n.shape === 'boundary');
        expect(boundaries.map(b => b.name)).toEqual(['CoffeeMachine']);
        expect(boundaries[0].children!.map(c => c.name).sort())
            .toEqual(['descale', 'makeCappuccino', 'makeEspresso']);

        expect(graph.nodes.filter(n => n.shape === 'actor').map(n => n.name).sort())
            .toEqual(['User']);

        const includes = graph.edges.filter(e => e.kind === 'include');
        expect(includes).toHaveLength(1);
        expect(includes[0]).toMatchObject({
            sourceId: 'UseCases::makeCappuccino',
            targetId: 'UseCases::makeEspresso'
        });
    });

    it('logical view: a strict capability tree naming no realization', async () => {
        const graph = extractLogicalGraph(await parseSet('logical.sysml'));
        // Two roots, one per use case definition: making a drink, and looking
        // after the machine. A forest of n trees has n fewer edges than nodes.
        expect(graph.edges).toHaveLength(graph.nodes.length - 2);
        expect(graph.edges.every(e => e.kind === 'decomposition')).toBe(true);

        const names = graph.nodes.map(n => n.name);
        expect(names).toContain('heatWater');
        expect(names).toContain('regulateTemperature');
        // This view says what, not what by: no realization name reaches it.
        // (`pumpWater` is a function; `WaterPump` is the thing that does it.)
        for (const realization of ['Thermoblock', 'WaterPump', 'ControlBoard', 'RecipeEngine']) {
            expect(names).not.toContain(realization);
        }

        const parents = new Set(graph.edges.map(e => e.targetId));
        expect(graph.nodes.filter(n => !parents.has(n.id)).map(n => n.name))
            .toEqual(['MakeBeverage', 'MaintainMachine']);
    });

    it('implementation view: the application layer depends on control, never the reverse', async () => {
        const graph = extractImplementationGraph(await parseSet('implementation.sysml', 'logical.sysml'));
        const imports = graph.edges.map(e => `${e.sourceId} -> ${e.targetId}`);
        expect(imports).toEqual(['Software::Application -> Software::Control']);

        // `sw`, the tree of deployable parts, is an instance, not a module.
        const names: string[] = [];
        const walk = (nodes: typeof graph.nodes): void => nodes.forEach(n => {
            names.push(n.name);
            walk(n.children ?? []);
        });
        walk(graph.nodes);
        expect(names).toContain('RecipeEngine');
        expect(names).not.toContain('sw');
    });

    it('implementation view: the recipe engine offers one API and needs three', async () => {
        const graph = extractImplementationGraph(await parseSet('implementation.sysml', 'logical.sysml'));
        const modules = new Map(allNodes(graph.nodes).map(n => [n.name, n]));
        const ports = (name: string) =>
            modules.get(name)!.compartments.find(c => c.title === 'ports')?.lines ?? [];

        // One offered, three needed: the whole of the machine's control is
        // this module calling those.
        expect(ports('RecipeEngine')).toEqual([
            'recipe : RecipeApi', 'brew : ~BrewApi', 'temperature : ~TemperatureApi', 'milk : ~MilkApi'
        ]);
        // The panel asks the engine; the engine drives the controllers.
        expect(ports('UiApp')).toEqual(['recipe : ~RecipeApi']);
        expect(ports('BrewController')).toEqual(['brew : BrewApi']);
    });

    it('physical view: the machine as nested parts, looms named by their interface', async () => {
        const graph = extractPhysicalGraph(await parseSet('physical.sysml', 'logical.sysml'));
        expect(graph.nodes.map(n => n.name)).toEqual(['machine : CoffeeMachine']);
        const names = graph.nodes[0].children!.map(n => n.name);
        expect(names).toHaveLength(11);
        expect(names).toContain('heater : Thermoblock');
        expect(names).toContain('controller : ControlBoard');
        expect(names).toContain('milkTube : MilkTube');
        // Interface defs label their connection rather than becoming a box.
        expect(names.some(n => n.includes('ControlLink'))).toBe(false);
        expect(graph.edges).toHaveLength(14);
        expect(graph.edges.filter(e => e.kind === 'connection' && e.label === 'ControlLink')).toHaveLength(6);
    });

    it('realization: every function is performed by something, and some parts by nothing', async () => {
        const logical = extractLogicalGraph(await parseSet('logical.sysml'));
        const parents = new Set(logical.edges.map(e => e.sourceId));
        const leaves = logical.nodes.filter(n => !parents.has(n.id)).map(n => n.name);

        const software = extractImplementationGraph(await parseSet('implementation.sysml', 'logical.sysml'));
        const hardware = extractPhysicalGraph(await parseSet('physical.sysml', 'logical.sysml'));
        const performed = new Set(
            [...allNodes(software.nodes), ...allNodes(hardware.nodes)]
                .flatMap(n => n.compartments.flatMap(c => c.lines))
        );

        // Every function has something that does it; a function nothing
        // performs would show here, as a hole in the design.
        expect(leaves.filter(leaf => !performed.has(leaf))).toEqual([]);
        // Frothing is realized twice over, by the wand and by the program that
        // drives the wand: realization is many to many.
        const frothers = [...allNodes(software.nodes), ...allNodes(hardware.nodes)]
            .filter(n => n.compartments.some(c => c.lines.includes('frothMilk')))
            .map(n => n.name);
        expect(frothers.sort()).toEqual(['MilkController', 'wand : SteamWand']);
        // The other direction is a part that realizes nothing: the panel and
        // the supply are there for the parts that do.
        const idle = allNodes(hardware.nodes).filter(n => n.compartments.length === 0).map(n => n.name);
        expect(idle).toContain('display : TouchDisplay');
        expect(idle).toContain('psu : MainsSupply');
    });

    it('deployment view: five programs on two hosts, inside the machine', async () => {
        const graph = extractDeploymentGraph(
            await parseSet('deployment.sysml', 'implementation.sysml', 'physical.sysml', 'logical.sysml')
        );
        expect(graph.edges).toHaveLength(0);
        // Both hosts are fitted in the machine, so they share its frame.
        expect(graph.nodes.map(n => n.name)).toEqual(['machine : CoffeeMachine']);
        const hosts = graph.nodes[0].children!;
        expect(hosts.map(n => n.id).sort()).toEqual([
            'Hardware::CoffeeMachine::controller',
            'Hardware::CoffeeMachine::display'
        ]);
        expect(hosts.every(h => h.shape === 'node3d')).toBe(true);
        const placed = hosts.flatMap(n => n.children!.map(c => c.name));
        expect(placed).toHaveLength(5);
        expect(new Set(placed).size).toBe(5);
    });

    it('process view: one press produces a cappuccino', async () => {
        const sequence = extractSequenceModel(await parseSet('process.sysml', 'implementation.sysml', 'logical.sysml'));
        expect(sequence.lifelines.map(l => l.label)).toEqual([
            'user',
            'ui : UiApp',
            'recipes : RecipeEngine',
            'temperature : TemperatureController',
            'brew : BrewController',
            'milk : MilkController'
        ]);
        // The user is untyped in the model: a person, drawn as a stick figure.
        expect(sequence.lifelines.filter(l => l.actor).map(l => l.label)).toEqual(['user']);
        expect(sequence.messages[0].label).toBe('selectCappuccino');
        expect(sequence.messages.at(-1)!.label).toBe('beverageReady');
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
                heading: `${type.view} — Coffee Machine`,
                source: files[0]
            });
            expect(svg.startsWith('<svg'), diagram).toBe(true);
            expect(svg, diagram).toContain(type.view);
        }
    });
});
