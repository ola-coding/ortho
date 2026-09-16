import { readFile } from 'node:fs/promises';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, expect, it } from 'vitest';
import { parseHelper } from 'langium/test';
import { createSysmlServices } from '../src/parser/sysml-module.js';
import { parseSysmlFiles } from '../src/parser/parse.js';
import { isActorUsage, isIncludeUsage, isUseCaseDef, isUseCaseUsage } from '../src/generated/ast.js';
import type { Model, UseCaseUsage } from '../src/generated/ast.js';
import { extractUseCaseGraph } from '../src/diagrams/use-case.js';
import { layoutGraph } from '../src/layout/elk-layout.js';
import { renderSvg } from '../src/render/svg-renderer.js';

const services = createSysmlServices();
const parse = parseHelper<Model>(services.Sysml);

async function parseOk(text: string) {
    const document = await parse(text, { validation: true });
    const errors = (document.diagnostics ?? []).filter(d => d.severity === 1);
    expect(errors, errors.map(e => e.message).join('; ')).toHaveLength(0);
    return document;
}

async function parseExample(): Promise<Model> {
    const text = await readFile(resolve(dirname(fileURLToPath(import.meta.url)), 'fixtures/use-case.sysml'), 'utf-8');
    return (await parseOk(text)).parseResult.value;
}

describe('use case grammar', () => {
    it('parses use case defs/usages with actors, subjects and includes', async () => {
        const document = await parseOk(`
            package P {
                part def Operator;
                part def System;
                use case def DoThing {
                    subject sys : System;
                    actor op : Operator;
                }
                use case doThing : DoThing {
                    include use case helpTask;
                }
                use case helpTask;
                part def Station {
                    perform doThing;
                }
            }
        `);
        const pkg = document.parseResult.value.packages[0];
        const def = pkg.members.find(isUseCaseDef)!;
        expect(def.members.some(isActorUsage)).toBe(true);
        const usage = pkg.members.find(m => isUseCaseUsage(m) && m.name === 'doThing') as UseCaseUsage;
        expect(usage.type?.ref?.name).toBe('DoThing');
        const include = usage.members.find(isIncludeUsage)!;
        expect(include.target.ref?.name).toBe('helpTask');
    });
});

describe('use-case diagram extraction', () => {
    it('builds boundary, ellipses, deduplicated actors and edges', async () => {
        const graph = extractUseCaseGraph(await parseExample());

        const boundary = graph.nodes.find(n => n.shape === 'boundary')!;
        expect(boundary.name).toBe('Drone');
        expect(boundary.children!.map(c => c.name).sort()).toEqual(
            ['avoidObstacles', 'capturePhotos', 'flyMission', 'returnToHome']
        );

        // FlyMission def is represented by its usage, not shown itself
        expect(graph.nodes.some(n => n.name === 'FlyMission')).toBe(false);

        const actors = graph.nodes.filter(n => n.shape === 'actor');
        expect(actors).toHaveLength(1);
        expect(actors[0].name).toBe('Pilot');

        const associations = graph.edges.filter(e => e.kind === 'association');
        expect(associations).toHaveLength(3); // flyMission (inherited), returnToHome, capturePhotos
        expect(associations.every(e => e.sourceId === actors[0].id)).toBe(true);

        const includes = graph.edges.filter(e => e.kind === 'include');
        expect(includes.map(e => e.targetId).sort()).toEqual(
            ['DroneUseCases::avoidObstacles', 'DroneUseCases::returnToHome']
        );
    });

    it('lays out hierarchically and renders actors, ellipses and «include»', async () => {
        const graph = extractUseCaseGraph(await parseExample());
        const laidOut = await layoutGraph(graph, { direction: 'RIGHT' });

        const boundary = laidOut.nodes.find(n => n.shape === 'boundary')!;
        const ellipses = laidOut.nodes.filter(n => n.shape === 'ellipse');
        expect(ellipses).toHaveLength(4);
        for (const e of ellipses) {
            expect(e.x).toBeGreaterThanOrEqual(boundary.x);
            expect(e.y).toBeGreaterThanOrEqual(boundary.y);
            expect(e.x + e.width).toBeLessThanOrEqual(boundary.x + boundary.width + 0.5);
            expect(e.y + e.height).toBeLessThanOrEqual(boundary.y + boundary.height + 0.5);
        }

        const svg = renderSvg(laidOut, { heading: 'Scenarios — DroneUseCases', source: 'examples/phase2-usecases.sysml' });
        expect(svg).toContain('<ellipse');
        expect(svg).toContain('<circle'); // actor head
        expect(svg).toContain('&#171;include&#187;');
    });

    it('draws every line straight, clear of every use case it does not join', async () => {
        const examples = resolve(dirname(fileURLToPath(import.meta.url)), '../examples');
        for (const example of ['survey-drone', 'coffee-machine']) {
            const { model } = await parseSysmlFiles(
                createSysmlServices().Sysml, [resolve(examples, example, 'use-case.sysml')]
            );
            const laidOut = await layoutGraph(extractUseCaseGraph(model), { direction: 'RIGHT', edgeRouting: 'STRAIGHT' });
            const ellipses = laidOut.nodes.filter(n => n.shape === 'ellipse');
            for (const edge of laidOut.edges) {
                const line = `${example}: ${edge.sourceId} -> ${edge.targetId}`;
                expect(edge.points, line).toHaveLength(2);
                const [p, q] = edge.points;
                for (const n of ellipses.filter(e => e.id !== edge.sourceId && e.id !== edge.targetId)) {
                    // Sampled along the line: how far inside the ellipse it
                    // reaches, where 1 is on the outline.
                    const rx = n.width / 2;
                    const ry = n.height / 2;
                    let nearest = Infinity;
                    for (let i = 0; i <= 200; i++) {
                        const x = p.x + (q.x - p.x) * i / 200;
                        const y = p.y + (q.y - p.y) * i / 200;
                        nearest = Math.min(nearest, ((x - n.x - rx) / rx) ** 2 + ((y - n.y - ry) / ry) ** 2);
                    }
                    expect(nearest, `${line} crosses ${n.name}`).toBeGreaterThan(1);
                }
            }
        }
    });
});
