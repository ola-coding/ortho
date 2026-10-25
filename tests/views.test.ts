import { describe, expect, it } from 'vitest';
import { parseHelper } from 'langium/test';
import { createSysmlServices } from '../src/parser/sysml-module.js';
import type { Model } from '../src/generated/ast.js';
import { declaredViews, diagramKind, exposedElements, viewHeading, whyUndrawable } from '../src/model/views.js';
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
        expect(extractLogicalGraph(exposedElements(functions)).nodes.map(n => n.name))
            .toEqual(['Operate', 'start', 'stop']);
        expect(extractSequenceModel(exposedElements(sequence), model).messages.map(m => m.label))
            .toEqual(['ping']);
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
            .toEqual(['A GeneralView exposes packages or action definitions.']);
    });
});
