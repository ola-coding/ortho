import { describe, expect, it } from 'vitest';
import { parseHelper } from 'langium/test';
import type { LangiumDocument } from 'langium';
import { createSysmlServices } from '../src/parser/sysml-module.js';
import {
    isAttributeUsage, isConnectionUsage, isPartDef, isPartUsage, isPortDef, isPortUsage,
    isRequirementDef, isRequirementUsage, isSatisfyUsage
} from '../src/generated/ast.js';
import type { ConnectionUsage, Model, PartDef, SatisfyUsage } from '../src/generated/ast.js';

const services = createSysmlServices();
const parse = parseHelper<Model>(services.Sysml);

async function parseOk(text: string): Promise<LangiumDocument<Model>> {
    const document = await parse(text, { validation: true });
    const errors = (document.diagnostics ?? []).filter(d => d.severity === 1);
    expect(errors, errors.map(e => e.message).join('; ')).toHaveLength(0);
    return document;
}

describe('Phase 1 grammar', () => {
    it('parses part defs with attributes, ports and nested parts', async () => {
        const document = await parseOk(`
            package P {
                port def PowerPort { attribute voltage : Real; }
                part def Battery {
                    attribute capacity : Real = 2590;
                    port powerOut : PowerPort;
                    part cell : Cell [4];
                }
                part def Cell;
            }
        `);
        const pkg = document.parseResult.value.packages[0];
        const battery = pkg.members.find(m => isPartDef(m) && m.name === 'Battery') as PartDef;
        expect(battery.members.filter(isAttributeUsage)).toHaveLength(1);
        expect(battery.members.filter(isPortUsage)).toHaveLength(1);
        const cell = battery.members.find(isPartUsage);
        expect(cell?.type?.ref?.name).toBe('Cell');
        expect(cell?.multiplicity?.lower).toBe('4');
    });

    it('resolves specialization and cross-package qualified names', async () => {
        const document = await parseOk(`
            package A { part def Base; }
            package B { part def Derived :> A::Base; }
        `);
        const b = document.parseResult.value.packages[1];
        const derived = b.members.find(isPartDef)!;
        expect(derived.supertypes[0].ref?.name).toBe('Base');
    });

    it('resolves connect feature chains through part types', async () => {
        const document = await parseOk(`
            package P {
                port def Pw;
                part def Battery { port out : Pw; }
                part def Fc { port in_ : Pw; }
                part def Drone {
                    part battery : Battery;
                    part fc : Fc;
                    connect battery.out to fc.in_;
                }
            }
        `);
        const pkg = document.parseResult.value.packages[0];
        const drone = pkg.members.find(m => isPartDef(m) && m.name === 'Drone') as PartDef;
        const connection = drone.members.find(isConnectionUsage) as ConnectionUsage;
        expect(connection.source.segments[0].ref?.name).toBe('battery');
        expect(connection.source.segments[1].ref?.name).toBe('out');
        expect(connection.target.segments[1].ref?.name).toBe('in_');
    });

    it('parses requirements and satisfy relations', async () => {
        const document = await parseOk(`
            package P {
                part def Drone;
                requirement def MassLimit {
                    attribute maxMass : Real = 249;
                    subject vehicle : Drone;
                }
                requirement massReq : MassLimit;
                satisfy massReq by Drone;
            }
        `);
        const pkg = document.parseResult.value.packages[0];
        expect(pkg.members.some(isRequirementDef)).toBe(true);
        const usage = pkg.members.find(isRequirementUsage)!;
        expect(usage.type?.ref?.name).toBe('MassLimit');
        const satisfy = pkg.members.find(isSatisfyUsage) as SatisfyUsage;
        expect(satisfy.requirement.ref?.name).toBe('massReq');
        expect(satisfy.satisfier.ref?.name).toBe('Drone');
    });

    it('resolves connector chains ending in an attribute', async () => {
        const document = await parseOk(`
            package P {
                part def Sensor { attribute rate : Real; }
                part def Logger { attribute input : Real; }
                part def Board {
                    part sensor : Sensor;
                    part logger : Logger;
                    connect sensor.rate to logger.input;
                }
            }
        `);
        const pkg = document.parseResult.value.packages[0];
        const board = pkg.members.find(m => isPartDef(m) && m.name === 'Board') as PartDef;
        const connection = board.members.find(isConnectionUsage) as ConnectionUsage;
        expect(connection.source.segments[1].ref?.name).toBe('rate');
        expect(connection.target.segments[1].ref?.name).toBe('input');
    });

    it('parses interface defs and imports', async () => {
        await parseOk(`
            package Ifaces {
                port def Pw;
                interface def PowerIface {
                    end supply : Pw;
                    end demand : Pw;
                }
            }
            package User {
                import Ifaces::*;
                part def X;
            }
        `);
    });

    it('reports unresolved references as errors', async () => {
        const document = await parse('package P { part def A :> Missing; }', { validation: true });
        const errors = (document.diagnostics ?? []).filter(d => d.severity === 1);
        expect(errors.length).toBeGreaterThan(0);
    });

    // A realization connects its internals to the ports it inherits from the
    // block it specializes, so those names must resolve without redeclaring them.
    it('resolves connector ends against ports inherited through :>', async () => {
        const document = await parseOk(`
            package P {
                port def DataPort;
                part def Block { port ctrlIn : DataPort; }
                part def Motor { port ctrlIn : DataPort; }
                part def Design :> Block {
                    part motor : Motor;
                    connect ctrlIn to motor.ctrlIn;
                }
            }
        `);
        const design = document.parseResult.value.packages[0].members
            .find(m => isPartDef(m) && m.name === 'Design') as PartDef;
        const connection = design.members.find(isConnectionUsage) as ConnectionUsage;
        expect(connection.source.segments[0].ref?.name).toBe('ctrlIn');
        expect(connection.source.segments[0].ref?.$container).toHaveProperty('name', 'Block');
    });

    it('a locally declared feature shadows an inherited one of the same name', async () => {
        const document = await parseOk(`
            package P {
                port def DataPort;
                part def Block { port ctrlIn : DataPort; }
                part def Motor { port ctrlIn : DataPort; }
                part def Design :> Block {
                    port ctrlIn : DataPort;
                    part motor : Motor;
                    connect ctrlIn to motor.ctrlIn;
                }
            }
        `);
        const design = document.parseResult.value.packages[0].members
            .find(m => isPartDef(m) && m.name === 'Design') as PartDef;
        const connection = design.members.find(isConnectionUsage) as ConnectionUsage;
        expect(connection.source.segments[0].ref?.$container).toHaveProperty('name', 'Design');
    });
});
