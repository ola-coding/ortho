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
                    include otherTask;
                }
                use case otherTask;
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
        // `include use case x` declares an included use case; `include x` names one.
        const [declared, named] = usage.members.filter(isIncludeUsage);
        expect([declared.useCase, declared.name]).toEqual([true, 'helpTask']);
        expect(named.target?.segments.at(-1)?.ref?.name).toBe('otherTask');
    });
});

describe('use-case diagram extraction', () => {
    it('builds use cases, one subject, deduplicated actors and edges', async () => {
        const graph = extractUseCaseGraph((await parseExample()).packages);

        // Each use case is a «use case» box, named as declared; the FlyMission
        // def is represented by its usage, not shown itself.
        const useCases = graph.nodes.filter(n => n.keyword === 'use case');
        expect(useCases.every(n => n.shape === 'usage')).toBe(true);
        expect(useCases.map(n => n.name).sort()).toEqual(
            ['avoidObstacles', 'capturePhotos', 'flyMission : FlyMission', 'returnToHome']
        );
        expect(graph.nodes.some(n => n.name === 'FlyMission')).toBe(false);

        // Four use cases declare the same subject: one «subject» box, joined to each.
        const subjects = graph.nodes.filter(n => n.keyword === 'subject');
        expect(subjects.map(n => n.name)).toEqual(['drone : Drone']);
        expect(graph.edges.filter(e => e.targetId === subjects[0].id).map(e => e.sourceId).sort())
            .toEqual(useCases.map(n => n.id).sort());

        const actors = graph.nodes.filter(n => n.shape === 'actor');
        expect(actors).toHaveLength(1);
        expect(actors[0].name).toBe('pilot : Pilot');

        // The pilot's lines: flyMission (inherited), returnToHome, capturePhotos.
        const lines = graph.edges.filter(e => e.kind === 'association' && e.sourceId === actors[0].id);
        expect(lines.map(e => e.targetId).sort()).toEqual([
            'DroneUseCases::capturePhotos', 'DroneUseCases::flyMission', 'DroneUseCases::returnToHome'
        ]);

        const includes = graph.edges.filter(e => e.kind === 'include');
        expect(includes.map(e => e.targetId).sort()).toEqual(
            ['DroneUseCases::avoidObstacles', 'DroneUseCases::returnToHome']
        );
    });

    it('lays out the actor, the use cases and the subject left to right, and renders them', async () => {
        const graph = extractUseCaseGraph((await parseExample()).packages);
        const laidOut = await layoutGraph(graph, { direction: 'RIGHT' });

        const actor = laidOut.nodes.find(n => n.shape === 'actor')!;
        const subject = laidOut.nodes.find(n => n.keyword === 'subject')!;
        const useCases = laidOut.nodes.filter(n => n.keyword === 'use case');
        expect(useCases).toHaveLength(4);
        for (const useCase of useCases) {
            expect(actor.x + actor.width).toBeLessThanOrEqual(useCase.x);
            expect(useCase.x + useCase.width).toBeLessThanOrEqual(subject.x);
        }

        const svg = renderSvg(laidOut, { heading: 'Scenarios — DroneUseCases', source: 'examples/phase2-usecases.sysml' });
        expect(svg).toContain('&#171;use case&#187;');
        expect(svg).toContain('&#171;subject&#187;');
        expect(svg).toContain('<circle'); // actor head
        // An «include» is a solid arrow (§8.2.3.25).
        expect(svg).toContain('&#171;include&#187;');
        expect(svg).toContain('marker-end="url(#openArrow)"');
        expect(svg).not.toContain('stroke-dasharray');
    });

    it('stands a primary actor left of the use cases and a secondary one right', async () => {
        // The customer comes with every order, so it is declared on the def:
        // primary. The courier joins one use case only: secondary.
        const document = await parseOk(`
            package Shop {
                part def Customer;
                part def Courier;
                use case def Order {
                    subject Webshop;
                    actor customer : Customer;
                }
                use case browse : Order;
                use case checkout : Order;
                use case deliver : Order {
                    subject Webshop;
                    actor courier : Courier;
                }
            }
        `);
        const laidOut = await layoutGraph(
            extractUseCaseGraph(document.parseResult.value.packages), { direction: 'RIGHT', edgeRouting: 'STRAIGHT' }
        );
        const byName = (name: string) => laidOut.nodes.find(n => n.name === name)!;
        const useCases = laidOut.nodes.filter(n => n.keyword === 'use case');
        const customer = byName('customer : Customer');
        const courier = byName('courier : Courier');
        expect(laidOut.nodes.filter(n => n.shape === 'actor')).toHaveLength(2);

        expect(customer.x + customer.width).toBeLessThanOrEqual(Math.min(...useCases.map(n => n.x)));
        expect(courier.x).toBeGreaterThanOrEqual(Math.max(...useCases.map(n => n.x + n.width)));

        // Each actor is joined to its own use cases and no others; the
        // customer's come through the def, deliver's included.
        const joined = (actorId: string) => laidOut.edges
            .filter(e => e.kind === 'association' && (e.sourceId === actorId || e.targetId === actorId))
            .map(e => (e.sourceId === actorId ? e.targetId : e.sourceId))
            .sort();
        expect(joined(customer.id)).toEqual(['Shop::browse', 'Shop::checkout', 'Shop::deliver']);
        expect(joined(courier.id)).toEqual(['Shop::deliver']);
    });

    it('draws every line straight, from outline to outline, clear of every box it does not join', async () => {
        const examples = resolve(dirname(fileURLToPath(import.meta.url)), '../examples');
        type Box = { x: number; y: number; width: number; height: number };
        const inside = (x: number, y: number, n: Box, by = 0) =>
            x > n.x - by && x < n.x + n.width + by && y > n.y - by && y < n.y + n.height + by;
        for (const example of ['survey-drone', 'coffee-machine']) {
            const { model } = await parseSysmlFiles(
                createSysmlServices().Sysml, [resolve(examples, example, 'use-case.sysml')]
            );
            const laidOut = await layoutGraph(extractUseCaseGraph(model.packages), { direction: 'RIGHT', edgeRouting: 'STRAIGHT' });
            const boxes = laidOut.nodes.filter(n => n.shape === 'usage' || n.shape === 'definition');
            for (const edge of laidOut.edges) {
                const line = `${example}: ${edge.sourceId} -> ${edge.targetId}`;
                expect(edge.points, line).toHaveLength(2);
                const [p, q] = edge.points;
                // An end on a box lies on its outline: within its bounds, and
                // no deeper than a rounded corner's arc.
                for (const [end, id] of [[p, edge.sourceId], [q, edge.targetId]] as const) {
                    const box = boxes.find(n => n.id === id);
                    if (box) {
                        expect(inside(end.x, end.y, box, 0.5) && !inside(end.x, end.y, box, -3.5), `${line} ends off ${box.name}`).toBe(true);
                    }
                }
                for (const n of boxes.filter(b => b.id !== edge.sourceId && b.id !== edge.targetId)) {
                    for (let i = 0; i <= 200; i++) {
                        const x = p.x + (q.x - p.x) * i / 200;
                        const y = p.y + (q.y - p.y) * i / 200;
                        expect(inside(x, y, n), `${line} crosses ${n.name}`).toBe(false);
                    }
                }
            }
        }
    });
});
