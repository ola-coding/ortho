import { readFile } from 'node:fs/promises';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, expect, it } from 'vitest';
import { parseHelper } from 'langium/test';
import { createSysmlServices } from '../src/parser/sysml-module.js';
import type { Model } from '../src/generated/ast.js';
import { extractPartDefinitionGraph } from '../src/diagrams/part-definition.js';
import { layoutGraph } from '../src/layout/elk-layout.js';
import { renderSvg } from '../src/render/svg-renderer.js';

const services = createSysmlServices();
const parse = parseHelper<Model>(services.Sysml);

async function parseExample(): Promise<Model> {
    const text = await readFile(resolve(dirname(fileURLToPath(import.meta.url)), 'fixtures/phase1-logical.sysml'), 'utf-8');
    const document = await parse(text, { validation: true });
    const errors = (document.diagnostics ?? []).filter(d => d.severity === 1);
    expect(errors, errors.map(e => e.message).join('; ')).toHaveLength(0);
    return document.parseResult.value;
}

describe('part-definition diagram extraction', () => {
    it('produces one node per definition/usage with compartments and ports', async () => {
        const graph = extractPartDefinitionGraph(await parseExample());
        const ids = graph.nodes.map(n => n.id);
        expect(ids).toContain('DroneLogical::Drone');
        expect(ids).toContain('DroneLogical::FlightController');
        expect(ids).toContain('DroneLogical::MassLimit');
        expect(ids).toContain('DroneLogical::massReq');
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

    it('produces specialization, composition, connection and satisfy edges', async () => {
        const graph = extractPartDefinitionGraph(await parseExample());
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

        expect(byKind('satisfy')).toEqual([
            expect.objectContaining({ sourceId: 'DroneLogical::Drone', targetId: 'DroneLogical::massReq' })
        ]);

        // `satisfy` reaches the usage; this edge carries on to the def holding
        // the value, completing part → requirement → specification.
        expect(byKind('typing')).toEqual([
            expect.objectContaining({
                sourceId: 'DroneLogical::massReq',
                targetId: 'DroneLogical::MassLimit'
            })
        ]);
    });

    it('leaves no node unconnected', async () => {
        const graph = extractPartDefinitionGraph(await parseExample());
        const touched = new Set(graph.edges.flatMap(e => [e.sourceId, e.targetId]));
        expect(graph.nodes.filter(n => !touched.has(n.id)).map(n => n.name)).toEqual([]);
    });

    it('lays out and renders to SVG end-to-end', async () => {
        const graph = extractPartDefinitionGraph(await parseExample());
        const laidOut = await layoutGraph(graph);
        expect(laidOut.nodes).toHaveLength(graph.nodes.length);
        for (const node of laidOut.nodes.filter(n => n.ports.length > 0)) {
            expect(node.ports.every(p => Number.isFinite(p.x) && Number.isFinite(p.y))).toBe(true);
        }

        const svg = renderSvg(laidOut, { heading: 'Logical View — DroneLogical', source: 'examples/phase1-logical.sysml' });
        expect(svg).toContain('<svg');
        expect(svg).toContain('&#171;part def&#187;');
        expect(svg).toContain('marker-start="url(#triangle)"');
        expect(svg).toContain('marker-start="url(#diamond)"');
        expect(svg).toContain('marker-end="url(#openArrow)"');
    });
});
