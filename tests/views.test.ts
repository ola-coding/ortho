import { describe, expect, it } from 'vitest';
import { parseHelper } from 'langium/test';
import { createSysmlServices } from '../src/parser/sysml-module.js';
import type { Model } from '../src/generated/ast.js';
import { declaredViews, diagramKind, exposedElements, viewFileName, viewHeading, whyUndrawable } from '../src/model/views.js';
import { renderViews } from '../src/pipeline.js';
import { extractLogicalGraph } from '../src/diagrams/logical.js';
import { extractSequenceModel } from '../src/diagrams/sequence.js';

const parse = parseHelper<Model>(createSysmlServices().Sysml);

async function errorsOf(text: string): Promise<string[]> {
    const document = await parse(text, { validation: true });
    return (document.diagnostics ?? []).filter(d => d.severity === 1)
        .map(d => typeof d.message === 'string' ? d.message : d.message.value);
}

// A function tree and a scenario side by side: each nests actions, and each
// view draws only what it exposes.
const TWO_VIEWS = `
    package M {
        part def Engine;
        action def Operate {
            action start;
            action stop;
        }
        action def Scenario {
            part a : Engine;
            part b : Engine;
            message ping from a to b;
        }
        view 'Functions — M' : GeneralView { expose Operate; }
        view sequence : SequenceView { expose Scenario; }
    }
`;

describe('views', () => {
    it('heads a diagram with its view name, without the quotes', async () => {
        const [functions, sequence] = declaredViews((await parse(TWO_VIEWS)).parseResult.value);
        expect(viewHeading(functions)).toBe('Functions — M');
        expect(viewHeading(sequence)).toBe('sequence');
        expect([diagramKind(functions), diagramKind(sequence)]).toEqual(['logical', 'process']);
    });

    it('draws only what a view exposes', async () => {
        const model = (await parse(TWO_VIEWS)).parseResult.value;
        const [functions, sequence] = declaredViews(model);
        // The scenario nests actions too, but the function view does not expose it.
        expect(extractLogicalGraph(exposedElements(functions), model).nodes.map(n => n.name))
            .toEqual(['Operate', 'start', 'stop']);
        expect(extractSequenceModel(exposedElements(sequence)).messages.map(m => m.label))
            .toEqual(['ping']);
    });

    it('draws a SequenceView of an occurrence def, as OMG writes a scenario', async () => {
        const [view] = declaredViews((await parse(`
            package M {
                private import StandardViewDefinitions::*;
                part def Engine;
                item def Go;
                occurrence def Scenario {
                    part a : Engine;
                    part b : Engine;
                    message ping of Go from a to b;
                }
                view sequence : SequenceView { expose Scenario; }
            }
        `)).parseResult.value);
        expect(whyUndrawable(view)).toBeUndefined();
        expect(diagramKind(view)).toBe('process');
        expect(extractSequenceModel(exposedElements(view)).messages.map(m => m.label)).toEqual(['ping of Go']);
    });

    it('loads any view, and says why it cannot draw one other than the three it knows', async () => {
        const why = async (text: string) => {
            expect(await errorsOf(text)).toEqual([]);
            return declaredViews((await parse(text)).parseResult.value).map(whyUndrawable);
        };
        expect(await why(TWO_VIEWS)).toEqual([undefined, undefined]);
        expect(await why('package M { action def A; view v : StateTransitionView { expose A; } }'))
            .toEqual(["ortho draws a GeneralView, an InterconnectionView or a SequenceView, not 'StateTransitionView'."]);
        expect(await why('package M { action def A; view v : InterconnectionView { expose A; } }'))
            .toEqual(['An InterconnectionView exposes parts.']);
        expect(await why('package M { part p; view v : GeneralView { expose p; } }'))
            .toEqual(['A GeneralView exposes packages, action definitions or use cases.']);
    });

    it('takes a view that exposes only a use case for the use case view', async () => {
        const [view] = declaredViews((await parse(`
            package M {
                use case def Order;
                view v : GeneralView { expose Order; }
            }
        `)).parseResult.value);
        expect(whyUndrawable(view)).toBeUndefined();
        expect(diagramKind(view)).toBe('use-case');
    });
    it('writes each view to a file named after its short name, or else its name', async () => {
        const views = declaredViews((await parse(`
            package Files {
                action def A;
                view 'Logical view — Coffee Machine' : GeneralView { expose A; }
                view <logical> 'Logical view' : GeneralView { expose A; }
                view 'Översikt: Kaffe & Te' : GeneralView { expose A; }
            }
        `)).parseResult.value);
        expect(views.map(viewFileName)).toEqual(['logical-view-coffee-machine.svg', 'logical.svg', 'oversikt-kaffe-te.svg']);
    });

    it('renders every view it can draw, passes over the rest, and refuses two views for one file', async () => {
        const rendering = async (text: string) => {
            const document = await parse(text);
            return renderViews({ model: document.parseResult.value, documents: [document] });
        };
        const { rendered, skipped } = await rendering(`
            package Every {
                action def A { action b; }
                view one : GeneralView { expose A; }
                view states : StateTransitionView { expose A; }
                view two : GeneralView { expose A; }
            }
        `);
        expect(rendered.map(r => r.file)).toEqual(['one.svg', 'two.svg']);
        expect(skipped.map(s => [s.view.name, s.reason]))
            .toEqual([['states', "ortho draws a GeneralView, an InterconnectionView or a SequenceView, not 'StateTransitionView'."]]);
        await expect(rendering(`
            package Twice {
                action def A;
                view 'Functions' : GeneralView { expose A; }
                view functions : GeneralView { expose A; }
            }
        `)).rejects.toThrow("'Functions' and 'functions' would both be written to functions.svg");
    });
});
