import { describe, expect, it } from 'vitest';
import { parseHelper } from 'langium/test';
import type { LangiumDocument } from 'langium';
import { createSysmlServices } from '../src/parser/sysml-module.js';
import {
    isActionDef, isAllocationUsage, isAttributeUsage, isConnectionUsage, isMessageUsage, isOtherDefinition, isOtherUsage,
    isPackageDecl, isPartDef, isPartUsage, isPortDef, isPortUsage, isSatisfyUsage
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
        expect(cell?.multiplicities[0]?.lower).toBe('4');
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
        expect(connection.source!.segments[0].ref?.name).toBe('battery');
        expect(connection.source!.segments[1].ref?.name).toBe('outlet');
        expect(connection.target!.segments[1].ref?.name).toBe('in_');
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
        // No view draws requirements, so what they name is kept as written.
        const pkg = document.parseResult.value.packages[0];
        expect(pkg.members.find(isOtherDefinition)?.kind).toBe('requirement');
        const usage = pkg.members.find(isOtherUsage)!;
        expect([usage.kind, usage.types]).toEqual(['requirement', ['MassLimit']]);
        const satisfy = pkg.members.find(isSatisfyUsage) as SatisfyUsage;
        expect([satisfy.requirement, satisfy.satisfier]).toEqual(['massReq', 'drone']);
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
        expect(connection.source!.segments[1].ref?.name).toBe('rate');
        expect(connection.target!.segments[1].ref?.name).toBe('input');
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
        expect(connection.source!.segments[0].ref?.name).toBe('ctrlIn');
        expect(connection.source!.segments[0].ref?.$container).toHaveProperty('name', 'Block');
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
        expect(connection.source!.segments[0].ref?.$container).toHaveProperty('name', 'Design');
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
        expect(allocation.source!.segments.map(s => s.ref?.name)).toEqual(['sw', 'video']);
        expect(allocation.target!.segments.map(s => s.ref?.name)).toEqual(['hw', 'cpu']);
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
        `)).toEqual([unresolved('Definition', 'MilkController')]);
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
        expect(allocation.source!.segments[0].ref?.$container).toBe(own);
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
        `)).toEqual([unresolved('Definition', 'Engine')]);
    });

    it('resolves nothing through a package that is not imported', async () => {
        expect(await errorsOf(`${LIBRARY}
            package ImpNone {
                allocate sw.video to ImpHardware::hw.cpu;
            }
        `)).toEqual([unresolved('Feature', 'sw')]);
    });
});

// Every test's document stays in the workspace, so these packages have names
// no other test uses.
describe('names and comments', () => {
    it('reads a quoted name as the name without its quotes, in declarations and references', async () => {
        const document = await parseOk(`
            package 'Name Hardware' {
                part def 'Brew unit' { port 'water in' : 'Water port'; }
                port def 'Water port';
                part def Machine {
                    part brew : 'Brew unit';
                    part tank : 'Name Hardware'::Tank;
                    connect brew.'water in' to tank.outlet;
                }
                part def Tank { port outlet : 'Water port'; }
            }
        `);
        const pkg = document.parseResult.value.packages[0];
        expect(pkg.name).toBe('Name Hardware');
        const machine = pkg.members.find(m => isPartDef(m) && m.name === 'Machine') as PartDef;
        const [brew, tank] = machine.members.filter(isPartUsage);
        expect([brew.type?.ref?.name, tank.type?.ref?.name]).toEqual(['Brew unit', 'Tank']);
        const connection = machine.members.find(isConnectionUsage)!;
        expect(connection.source!.segments.map(s => s.ref?.name)).toEqual(['brew', 'water in']);
    });

    it('finds an element by its short name or an alias', async () => {
        const document = await parseOk(`
            package <NS> 'Name Short' {
                part def <BU> 'Brew unit';
                alias Unit for BU;
                part a : BU;
                part b : Unit;
                part c : NS::BU;
            }
        `);
        const parts = document.parseResult.value.packages[0].members.filter(isPartUsage);
        expect(parts.map(p => p.type?.ref?.name)).toEqual(['Brew unit', 'Brew unit', 'Brew unit']);
    });

    it('keeps comments in the model and notes out of it', async () => {
        await parseOk(`
            /* A comment on the whole file. */
            standard library package NameLibrary {
                doc /* Documentation of the package. */
                //* A note, which no tool keeps. */
                // Another note.
                comment about Engine /* About the engine. */
                part def Engine {
                    doc <D1> locale "en_US" /* Documentation of the engine. */
                    rep language "text" /* An engine. */
                }
            }
            library package NameOther {
                comment Note /* A named comment. */
            }
        `);
    });

    it('imports everything nested in a package with ::**', async () => {
        await parseOk(`
            package NameDeep { package Inner { part def Pump; } }
            package NameUser {
                private import all NameDeep::**;
                part pump : Pump;
            }
        `);
    });
});

describe('definitions and usages', () => {
    it('reads every kind, keeping what no view draws as written', async () => {
        await parseOk(`
            package Kinds {
                item def Fuel;
                enum def Color { red; green; enum blue; }
                abstract part def Vehicle {
                    attribute mass : Real;
                    ref item fuel : Fuel;
                    part engine : Engine;
                    port fuelIn : ~FuelPort;
                }
                part def Engine { event occurrence started; }
                port def FuelPort { in item fuel : Fuel; }
                part def Car :> Vehicle { part :>> engine : Engine; }
                connection def Hose { end part a : Engine; end part b : Engine; }
                allocation def Assign;
                state def Running;
                calc def Area;
                constraint def Limit;
                requirement def Speed { subject car : Car; }
                action def Drive { in fuel : Fuel; out distance : Real; }
                part car : Car {
                    bind engine.started = engine.started;
                    flow of Fuel from fuelIn to engine;
                    exhibit state running : Running;
                    #Safety part brake;
                }
                view def V;
                individual def Car1 :> Car;
                metadata def Safety;
                @Safety;
                dependency car to Car;
                occurrence def O { snapshot s; timeslice t; }
            }
        `);
    });

    it('reads values and expressions, which no view evaluates', async () => {
        await parseOk(`
            package ExprParts { metadata def Safety; #Safety part def Brake; }
            package ExprKinds {
                private import ScalarValues::*;
                private import ExprParts::*[@Safety];
                attribute def Mass;
                part def Car {
                    attribute mass : Mass = 1500 [kg];
                    attribute speed default := 3.6e1 * 2 ** 2;
                    attribute fast = if speed > 100 ? true else false;
                    attribute counts[*] = (1, 2, 3);
                    attribute heavy = counts->select { in c; c > 1 }->size() >= 2 and not fast;
                    constraint limit { mass <= 2000 [kg] }
                    requirement r { require constraint { speed < 200 } }
                }
                calc def Area { in w : Real; in h : Real; return a : Real = w * h; }
                view v : GeneralView { filter @Safety; }
            }
        `);
    });

    it('takes a message from and to event occurrences, on the parts that have them', async () => {
        const document = await parseOk(`
            package KindEvents {
                part def Driver { event occurrence turnOn; }
                part def Car { event occurrence started; }
                action def Start {
                    part driver : Driver;
                    part car : Car;
                    message go from driver.turnOn to car.started;
                }
            }
        `);
        const start = document.parseResult.value.packages[0].members.find(isActionDef)!;
        const message = start.members.find(isMessageUsage)!;
        expect(message.source!.segments.map(s => s.ref?.$type)).toEqual(['PartUsage', 'OtherUsage']);
    });

    it('names an unnamed redefinition and perform after what they refer to', async () => {
        const document = await parseOk(`
            package KindNames {
                action def Work { action step; }
                action work : Work;
                part def Base { part arm; }
                part def Robot :> Base {
                    part :>> arm;
                    perform work.step;
                }
                part robot : Robot;
                allocate robot.arm to robot.step;
            }
        `);
        const allocation = document.parseResult.value.packages[0].members.find(isAllocationUsage)!;
        expect(allocation.target!.segments.map(s => s.ref?.$type)).toEqual(['PartUsage', 'PerformUsage']);
    });
});

describe('the standard library', () => {
    const errorsOf = async (text: string): Promise<string[]> => {
        const document = await parse(text, { validation: true });
        return (document.diagnostics ?? []).filter(d => d.severity === 1)
            .map(d => typeof d.message === 'string' ? d.message : d.message.value);
    };

    it('lets a view name a library element without loading the library, and still catches a typo', async () => {
        // SpatialItem is declared in SpatialItems; MassValue in ISQBase, which ISQ imports publicly.
        expect(await errorsOf(`
            package LibraryNames {
                private import SpatialItems::*;
                private import ISQ::*;
                part def Car :> SpatialItem;
                part car : SpatialItems::SpatialItem;
                action def Weigh {
                    part scale : Car;
                    part car : Car;
                    message weight of MassValue from scale to car;
                }
            }
        `)).toEqual([]);
        expect(await errorsOf('package LibraryTypo { private import SpatialItems::*; part car : SpatialIten; }'))
            .toEqual(["Could not resolve reference to Definition named 'SpatialIten'."]);
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

    it('rejects a reserved word as a name, even one ortho does not parse, unless it is quoted', async () => {
        for (const word of ['frame', 'until']) {
            expect(await errorsOf(`package P { part def Drone { part ${word}; } }`)).not.toEqual([]);
            expect(await errorsOf(`package P { part def Drone { part '${word}'; } }`)).toEqual([]);
        }
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
