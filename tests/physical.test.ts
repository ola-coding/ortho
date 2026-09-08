import { readFile } from 'node:fs/promises';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, expect, it } from 'vitest';
import { parseHelper } from 'langium/test';
import { createSysmlServices } from '../src/parser/sysml-module.js';
import type { Model } from '../src/generated/ast.js';
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

describe('physical diagram extraction', () => {
    it('produces one node per part definition/usage with compartments and ports', async () => {
        const graph = extractPhysicalGraph(await parseExample());
        const ids = graph.nodes.map(n => n.id);
        expect(ids).toContain('DroneLogical::Drone');
        expect(ids).toContain('DroneLogical::FlightController');
        // Port defs are not rendered: no edge kind can terminate on one, so they
        // could only ever appear as disconnected boxes. Ports show as markers.
        expect(ids).not.toContain('DroneLogical::PowerPort');

        const drone = graph.nodes.find(n => n.id === 'DroneLogical::Drone')!;
        expect(drone.stereotype).toBe('part def');
        expect(drone.compartments[0].lines).toContain('mass : Real = 249');
        expect(drone.compartments[1].lines).toContain('motors : Motor [4]');

        const fc = graph.nodes.find(n => n.id === 'DroneLogical::FlightController')!;
        expect(fc.ports.map(p => p.label).sort()).toEqual(['motorCtrl', 'powerIn']);
    });

    it('carries no requirements: the view is the product, not its specification', async () => {
        const graph = extractPhysicalGraph(await parseExample());
        const ids = graph.nodes.map(n => n.id);
        // The fixture declares MassLimit, massReq and a satisfy; none of them
        // belong to any of the six views, so none of them reach this graph.
        expect(ids).not.toContain('DroneLogical::MassLimit');
        expect(ids).not.toContain('DroneLogical::massReq');
        expect(graph.edges.some(e => e.kind === 'satisfy' || e.kind === 'typing')).toBe(false);
    });

    it('produces specialization, composition and connection edges', async () => {
        const graph = extractPhysicalGraph(await parseExample());
        const byKind = (kind: string) => graph.edges.filter(e => e.kind === kind);

        // super → sub, so generalization triangles render at the super end
        expect(byKind('specialization')).toEqual([
            expect.objectContaining({ sourceId: 'DroneLogical::Drone', targetId: 'DroneLogical::Quadcopter' })
        ]);

        expect(byKind('composition')).toHaveLength(3);
        expect(byKind('composition').map(e => e.label)).toContain('motors [4]');

        const connections = byKind('connection');
        expect(connections).toHaveLength(2);
        expect(connections[0]).toMatchObject({
            sourceId: 'DroneLogical::Battery',
            sourcePortId: 'DroneLogical::Battery.powerOut',
            targetId: 'DroneLogical::FlightController',
            targetPortId: 'DroneLogical::FlightController.powerIn'
        });
    });

    it('leaves no node unconnected', async () => {
        const graph = extractPhysicalGraph(await parseExample());
        const touched = new Set(graph.edges.flatMap(e => [e.sourceId, e.targetId]));
        expect(graph.nodes.filter(n => !touched.has(n.id)).map(n => n.name)).toEqual([]);
    });

    it('lays out and renders to SVG end-to-end', async () => {
        const graph = extractPhysicalGraph(await parseExample());
        const laidOut = await layoutGraph(graph);
        expect(laidOut.nodes).toHaveLength(graph.nodes.length);
        for (const node of laidOut.nodes.filter(n => n.ports.length > 0)) {
            expect(node.ports.every(p => Number.isFinite(p.x) && Number.isFinite(p.y))).toBe(true);
        }

        const svg = renderSvg(laidOut, { heading: 'Physical view — DroneLogical', source: 'fixtures/physical.sysml' });
        expect(svg).toContain('<svg');
        expect(svg).toContain('&#171;part def&#187;');
        expect(svg).toContain('marker-start="url(#triangle)"');
        expect(svg).toContain('marker-start="url(#diamond)"');
    });
});
