import { describe, expect, it } from 'vitest';
import { parseHelper } from 'langium/test';
import type { LangiumDocument } from 'langium';
import { createSysmlServices } from '../src/parser/sysml-module.js';
import {
    isAllocationUsage, isAttributeUsage, isConnectionUsage, isPackageDecl, isPartDef, isPartUsage, isPortDef, isPortUsage,
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

describe('core grammar', () => {
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
                part def Battery { port outlet : Pw; }
                part def Fc { port in_ : Pw; }
                part def Drone {
                    part battery : Battery;
                    part fc : Fc;
                    connect battery.outlet to fc.in_;
                }
            }
        `);
        const pkg = document.parseResult.value.packages[0];
        const drone = pkg.members.find(m => isPartDef(m) && m.name === 'Drone') as PartDef;
        const connection = drone.members.find(isConnectionUsage) as ConnectionUsage;
        expect(connection.source.segments[0].ref?.name).toBe('battery');
        expect(connection.source.segments[1].ref?.name).toBe('outlet');
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
                part drone : Drone;
                satisfy massReq by drone;
            }
        `);
        const pkg = document.parseResult.value.packages[0];
        expect(pkg.members.some(isRequirementDef)).toBe(true);
        const usage = pkg.members.find(isRequirementUsage)!;
        expect(usage.type?.ref?.name).toBe('MassLimit');
        const satisfy = pkg.members.find(isSatisfyUsage) as SatisfyUsage;
        expect(satisfy.requirement.ref?.name).toBe('massReq');
        expect(satisfy.satisfier.ref?.name).toBe('drone');
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
                private import Ifaces::*;
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

// Every test's document stays in the workspace, so these packages have names
// no other test uses.
describe('imports', () => {
    const errorsOf = async (text: string): Promise<string[]> => {
        const document = await parse(text, { validation: true });
        return (document.diagnostics ?? []).filter(d => d.severity === 1)
            .map(d => typeof d.message === 'string' ? d.message : d.message.value);
    };
    const unresolved = (type: string, name: string) => `Could not resolve reference to ${type} named '${name}'.`;
    const LIBRARY = `
        package ImpSoftware {
            package Control { part def BrewController; part def MilkController; }
            part sw { part video; part other; }
        }
        package ImpHardware {
            part hw { part cpu; }
        }
    `;

    it('makes a package\'s members visible by their own names', async () => {
        const document = await parseOk(`${LIBRARY}
            package ImpDeployment {
                private import ImpSoftware::*;
                private import ImpHardware::*;
                private import ScalarValues::*;
                allocate sw.video to hw.cpu;
                part brew : Control::BrewController;
            }
        `);
        const deployment = document.parseResult.value.packages[2];
        const allocation = deployment.members.find(isAllocationUsage)!;
        expect(allocation.source.segments.map(s => s.ref?.name)).toEqual(['sw', 'video']);
        expect(allocation.target.segments.map(s => s.ref?.name)).toEqual(['hw', 'cpu']);
    });

    it('imports one element; a package\'s own members outrank imports, which reach nested packages', async () => {
        expect(await errorsOf(`${LIBRARY}
            package ImpOne {
                private import ImpSoftware::sw;
                allocate sw.video to ImpHardware::hw.cpu;
            }
        `)).toEqual([]);
        expect(await errorsOf(`${LIBRARY}
            package ImpOnly {
                private import ImpSoftware::Control::BrewController;
                part def P { part b : BrewController; part m : MilkController; }
            }
        `)).toEqual([unresolved('PartDef', 'MilkController')]);
        const document = await parseOk(`${LIBRARY}
            package ImpOwn {
                private import ImpSoftware::*;
                part sw { part video; }
                package Inner {
                    allocate sw.video to ImpHardware::hw.cpu;
                    part def Brew { part brew : Control::BrewController; }
                }
            }
        `);
        const own = document.parseResult.value.packages[2];
        const inner = own.members.find(isPackageDecl)!;
        const allocation = inner.members.find(isAllocationUsage)!;
        expect(allocation.source.segments[0].ref?.$container).toBe(own);
    });

    it('passes on public imports only', async () => {
        const units = `
            package ImpParts { part def Engine; part def Motor; }
            package ImpUnits {
                private import ImpParts::Engine;
                public import ImpParts::Motor;
            }
        `;
        expect(await errorsOf(`${units}
            package ImpUser {
                private import ImpUnits::*;
                part def M { part m : Motor; }
                part def Q { part q : ImpUnits::Motor; }
            }
        `)).toEqual([]);
        expect(await errorsOf(`${units}
            package ImpPrivate {
                private import ImpUnits::*;
                part def E { part e : Engine; }
            }
        `)).toEqual([unresolved('PartDef', 'Engine')]);
    });

    it('resolves nothing through a package that is not imported', async () => {
        expect(await errorsOf(`${LIBRARY}
            package ImpNone {
                allocate sw.video to ImpHardware::hw.cpu;
            }
        `)).toEqual([unresolved('Feature', 'sw'), unresolved('Feature', 'video')]);
    });
});

// A model ortho accepts should also be valid SysML v2, so the rules the OMG
// pilot implementation enforces and ortho can check are checked here too.
describe('SysML v2 conformance', () => {
    const errorsOf = async (text: string): Promise<string[]> => {
        const document = await parse(text, { validation: true });
        return (document.diagnostics ?? []).filter(d => d.severity === 1)
            .map(d => typeof d.message === 'string' ? d.message : d.message.value);
    };

    it('requires an import to state its visibility', async () => {
        expect(await errorsOf('package A { part def X; } package B { import A::*; }')).not.toHaveLength(0);
        for (const visibility of ['private', 'protected', 'public']) {
            expect(await errorsOf(`package A { part def X; } package B { ${visibility} import A::*; }`)).toEqual([]);
        }
    });

    it('rejects a reserved word as a name, even one ortho does not parse', async () => {
        const errors = await errorsOf('package P { part def Drone { part frame; } }');
        expect(errors).toEqual(["'frame' is a reserved word in SysML v2 and cannot be a name."]);
    });

    it('rejects a qualified name into a definition, and accepts the chain from a usage', async () => {
        const model = (target: string) => `
            package H { part def M { part c; } part m : M; }
            package S { part sw { part a; } }
            package D { allocate S::sw.a to ${target}; }`;
        expect(await errorsOf(model('H::M::c'))).toEqual(
            ["'H::M::c' names a feature through its owner. Start at a usage and continue with dots."]
        );
        expect(await errorsOf(model('H::m.c'))).toEqual([]);
    });

    it('requires a use case to declare its subject before its actors', async () => {
        const model = (body: string) => `
            package U {
                part def Operator;
                use case def Operate { subject Machine; actor operator : Operator; }
                use case service : Operate { ${body} }
            }`;
        expect(await errorsOf(model('actor technician : Operator;'))).toEqual([
            "Declare the subject of 'service' before its actors: the subject is a use case's first parameter."
        ]);
        expect(await errorsOf(model('subject Machine; actor technician : Operator;'))).toEqual([]);
    });

    it('performs an action through a chain that starts at a usage', async () => {
        const model = (target: string) => `
            package Functions {
                action def Survey { action communicate { action streamVideo; } }
                action survey : Survey;
            }
            package Software {
                part def Streamer { port data; perform ${target}; }
            }`;
        expect(await errorsOf(model('Functions::survey.communicate.streamVideo'))).toEqual([]);
        // The spec's own rule, and the pilot implementation's: a path into a
        // definition's features is not a feature chain.
        expect(await errorsOf(model('Functions::Survey::communicate::streamVideo'))).toEqual([
            "'Functions::Survey::communicate::streamVideo' names a feature through its owner."
                + ' Start at a usage and continue with dots.'
        ]);
    });

    it('refuses to perform something that is not an action or a use case', async () => {
        const errors = await errorsOf(`
            package P {
                part def Streamer { port data; perform data; }
            }`);
        expect(errors).toEqual(["'data' is not an action or a use case, so it cannot be performed."]);
    });

    it('gives a port directed features, and conjugates the port that needs them', async () => {
        expect(await errorsOf(`
            package P {
                port def Api {
                    in attribute command : String;
                    out attribute status : String;
                    inout attribute parameter : Real;
                }
                interface def Link {
                    end offered : Api;
                    end needed : ~Api;
                }
                part def Server { port api : Api; }
                part def Client { port api : ~Api; }
            }`)).toEqual([]);
    });

    it('resolves a message payload to an attribute def', async () => {
        const text = (payload: string) => `
            package P {
                attribute def Command;
                part def Sender; part def Receiver;
                action def Talk {
                    part s : Sender; part r : Receiver;
                    message go of ${payload} from s to r;
                }
            }`;
        expect(await errorsOf(text('Command'))).toEqual([]);
        expect(await errorsOf(text('Undeclared'))).not.toHaveLength(0);
    });
});
