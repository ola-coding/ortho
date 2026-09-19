import { readFile } from 'node:fs/promises';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, expect, it } from 'vitest';
import { parseHelper } from 'langium/test';
import { createSysmlServices } from '../src/parser/sysml-module.js';
import type { Model } from '../src/generated/ast.js';
import type { GraphNode } from '../src/model/graph.js';
import { extractPhysicalGraph } from '../src/diagrams/physical.js';
import { layoutGraph } from '../src/layout/elk-layout.js';
import { renderSvg } from '../src/render/svg-renderer.js';

const services = createSysmlServices();
const parse = parseHelper<Model>(services.Sysml);

async function parseExample(): Promise<Model> {
    const text = await readFile(resolve(dirname(fileURLToPath(import.meta.url)), 'fixtures/physical.sysml'), 'utf-8');
    const document = await parse(text, { validation: true });
    const errors = (document.diagnostics ?? []).filter(d => d.severity === 1);
    expect(errors, errors.map(e => e.message).join('; ')).toHaveLength(0);
    return document.parseResult.value;
}

function allNodes(nodes: GraphNode[]): GraphNode[] {
    return nodes.flatMap(n => [n, ...allNodes(n.children ?? [])]);
}

describe('physical diagram extraction', () => {
    it('draws the product, not its types: one box per part, nested in its owner', async () => {
        const graph = extractPhysicalGraph(await parseExample());
        // Drone is only a supertype and nothing uses Quadcopter as a part type,
        // so the quadcopter is the one top-level box, carrying what it inherits.
        expect(graph.nodes.map(n => n.name)).toEqual(['Quadcopter']);
        const quadcopter = graph.nodes[0];
        expect(quadcopter.children!.map(c => c.name)).toEqual([
            'fc : FlightController', 'battery : Battery', 'motors : Motor [4]'
        ]);
        // Port defs are not boxes; ports show as markers on the parts.
        expect(allNodes(graph.nodes).map(n => n.name)).not.toContain('PowerPort');
        expect(quadcopter.children![0].ports.map(p => p.label).sort()).toEqual(['motorCtrl', 'powerIn']);
    });

    it('shows no attributes, stereotypes or type-level edges', async () => {
        const graph = extractPhysicalGraph(await parseExample());
        for (const node of allNodes(graph.nodes)) {
            expect(node.compartments, node.name).toEqual([]);
            expect(node.stereotype, node.name).toBe('');
        }
        expect(graph.edges.every(e => e.kind === 'connection')).toBe(true);
    });

    it('carries no requirements: the view is the product, not its specification', async () => {
        const graph = extractPhysicalGraph(await parseExample());
        const names = allNodes(graph.nodes).map(n => n.name);
        // The fixture declares PowerBudget, powerReq and a satisfy; none of them
        // belong to any of the six views, so none of them reach this graph.
        expect(names.some(n => n.includes('PowerBudget') || n.includes('powerReq'))).toBe(false);
        expect(graph.edges.some(e => e.kind === 'satisfy' || e.kind === 'typing')).toBe(false);
    });

    it('wires each connection between the ports of the parts it names', async () => {
        const graph = extractPhysicalGraph(await parseExample());
        expect(graph.edges.map(e => [e.sourcePortId, e.targetPortId])).toEqual([
            ['DroneLogical::Quadcopter.battery#powerOut', 'DroneLogical::Quadcopter.fc#powerIn'],
            ['DroneLogical::Quadcopter.fc#motorCtrl', 'DroneLogical::Quadcopter.motors#ctrlIn']
        ]);
    });

    it('lays each part out inside its owner and renders without class-diagram markers', async () => {
        const graph = extractPhysicalGraph(await parseExample());
        const laidOut = await layoutGraph(graph);
        const byId = new Map(laidOut.nodes.map(n => [n.id, n]));
        const owner = byId.get('DroneLogical::Quadcopter')!;
        for (const child of graph.nodes[0].children!) {
            const box = byId.get(child.id)!;
            expect(box.x).toBeGreaterThanOrEqual(owner.x);
            expect(box.y).toBeGreaterThanOrEqual(owner.y);
            expect(box.x + box.width).toBeLessThanOrEqual(owner.x + owner.width + 0.5);
            expect(box.y + box.height).toBeLessThanOrEqual(owner.y + owner.height + 0.5);
        }
        // ELK placed every port label, so none is left to the fallback.
        for (const node of laidOut.nodes.filter(n => n.ports.length > 0)) {
            expect(node.ports.every(p => p.labelX !== undefined && p.labelY !== undefined), node.name).toBe(true);
        }

        const svg = renderSvg(laidOut, { heading: 'Physical view — DroneLogical', source: 'fixtures/physical.sysml' });
        expect(svg).toContain('<svg');
        expect(svg).toContain('>powerIn</text>');
        expect(svg).not.toContain('part def');
        expect(svg).not.toContain('url(#diamond)');
        expect(svg).not.toContain('url(#triangle)');
    });
});
