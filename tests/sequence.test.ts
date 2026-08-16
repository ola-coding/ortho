import { readFile } from 'node:fs/promises';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, expect, it } from 'vitest';
import { parseHelper } from 'langium/test';
import { createSysmlServices } from '../src/parser/sysml-module.js';
import { isActionDef, isMessageUsage } from '../src/generated/ast.js';
import type { Model } from '../src/generated/ast.js';
import { extractSequenceModel } from '../src/diagrams/sequence.js';
import { renderSequenceSvg } from '../src/render/sequence-renderer.js';

const services = createSysmlServices();
const parse = parseHelper<Model>(services.Sysml);

async function parseOk(text: string) {
    const document = await parse(text, { validation: true });
    const errors = (document.diagnostics ?? []).filter(d => d.severity === 1);
    expect(errors, errors.map(e => e.message).join('; ')).toHaveLength(0);
    return document;
}

async function parseExample(): Promise<Model> {
    const text = await readFile(resolve(dirname(fileURLToPath(import.meta.url)), 'fixtures/phase4-sequence.sysml'), 'utf-8');
    return (await parseOk(text)).parseResult.value;
}

describe('Phase 4 grammar', () => {
    it('parses action defs with ordered messages', async () => {
        const model = await parseExample();
        const actionDef = model.packages[0].members.find(isActionDef)!;
        const messages = actionDef.members.filter(isMessageUsage);
        expect(messages).toHaveLength(6);
        expect(messages[0].then).toBeFalsy();
        expect(messages[1].then).toBe(true);
        expect(messages[0].name).toBe('cmdTakeOff');
        expect(messages[0].payload).toBe('TakeOffCommand');
        expect(messages[0].source.segments[0].ref?.name).toBe('pilot');
        expect(messages[0].target.segments[0].ref?.name).toBe('fc');
    });

    it('parses nested actions and lifeline parts local to an action def', async () => {
        const document = await parseOk(`
            package P {
                action def Handshake {
                    part client;
                    part server;
                    message syn from client to server;
                    then message ack from server to client;
                    action retry;
                }
            }
        `);
        const actionDef = document.parseResult.value.packages[0].members.find(isActionDef)!;
        const messages = actionDef.members.filter(isMessageUsage);
        expect(messages[0].source.segments[0].ref?.name).toBe('client');
    });
});

describe('sequence extraction and rendering', () => {
    it('orders lifelines by first appearance and keeps message order', async () => {
        const sequence = extractSequenceModel(await parseExample());
        expect(sequence.lifelines.map(l => l.label)).toEqual([
            'pilot : Pilot', 'fc : FlightControllerSw', 'nav : NavigationSw', 'motors : MotorControl'
        ]);
        expect(sequence.messages.map(m => m.label)).toEqual([
            'cmdTakeOff : TakeOffCommand', 'planRoute', 'computePath', 'waypoints', 'motorSetpoints', 'telemetry'
        ]);
        const self = sequence.messages[2];
        expect(self.sourceId).toBe(self.targetId);
    });

    it('renders lifelines, messages and a self-loop to SVG', async () => {
        const sequence = extractSequenceModel(await parseExample());
        const svg = renderSequenceSvg(sequence, { heading: 'Process View — DroneProcess', source: 'examples/phase4-sequence.sysml' });
        expect(svg).toContain('stroke-dasharray="4 4"'); // lifelines
        expect(svg).toContain('marker-end="url(#msgArrow)"');
        expect(svg).toContain('cmdTakeOff : TakeOffCommand');
        expect(svg).toContain('<polyline'); // self-message loop
        // messages appear top-to-bottom in document order
        expect(svg.indexOf('planRoute')).toBeLessThan(svg.indexOf('telemetry'));
    });
});
