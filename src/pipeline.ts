import { relative } from 'node:path';
import { AstUtils } from 'langium';
import type { Member, Model, ViewUsage } from './generated/ast.js';
import type { DiagramGraph } from './model/graph.js';
import type { ParsedModel } from './parser/parse.js';
import { declaredViews, diagramKind, exposedElements, viewFileName, viewHeading, whyUndrawable } from './model/views.js';
import type { DiagramKind } from './model/views.js';
import { extractUseCaseGraph } from './diagrams/use-case.js';
import { extractLogicalGraph } from './diagrams/logical.js';
import { extractImplementationGraph } from './diagrams/implementation.js';
import { extractPhysicalGraph } from './diagrams/physical.js';
import { extractDeploymentGraph } from './diagrams/deployment.js';
import { extractSequenceModel } from './diagrams/sequence.js';
import { layoutGraph } from './layout/elk-layout.js';
import type { LayoutOptions } from './layout/elk-layout.js';
import { layoutTree } from './layout/tree-layout.js';
import { layoutUseCases } from './layout/use-case-layout.js';
import type { LaidOutDiagram } from './layout/elk-layout.js';
import { renderSvg } from './render/svg-renderer.js';
import type { DiagramTitle } from './render/svg-renderer.js';
import { renderSequenceSvg } from './render/sequence-renderer.js';
import type { Theme } from './render/theme.js';
import { lightTheme } from './render/theme.js';

export interface RenderResult {
    svg: string;
    summary: string;
}

type Extractor = (exposed: Member[], model: Model) => DiagramGraph;
type Renderer = (exposed: Member[], model: Model, title: DiagramTitle, theme: Theme) => Promise<RenderResult>;

function graphDiagram(extract: Extractor, layout: LayoutOptions = {}): Renderer {
    return async (exposed, model, title, theme) => {
        const graph = extract(exposed, model);
        const laidOut = await layoutGraph(graph, layout);
        return {
            svg: renderSvg(laidOut, title, theme),
            summary: `${laidOut.nodes.length} node(s), ${graph.edges.length} edge(s)`
        };
    };
}

/** Laid out by ortho itself rather than ELK, where the view's shape is fully determined. */
function ownLayout(extract: Extractor, layout: (graph: DiagramGraph) => LaidOutDiagram): Renderer {
    return async (exposed, model, title, theme) => {
        const graph = extract(exposed, model);
        const laidOut = layout(graph);
        return {
            svg: renderSvg(laidOut, title, theme),
            summary: `${laidOut.nodes.length} node(s), ${graph.edges.length} edge(s)`
        };
    };
}

/** The six diagrams, in the order VIEWS.md introduces them. */
const renderers: Record<DiagramKind, Renderer> = {
    'use-case': ownLayout(extractUseCaseGraph, layoutUseCases),
    'logical': ownLayout(extractLogicalGraph, layoutTree),
    'implementation': graphDiagram(extractImplementationGraph, { gridLeaves: true }),
    'physical': graphDiagram(extractPhysicalGraph),
    'deployment': graphDiagram(extractDeploymentGraph, { algorithm: 'rectpacking' }),
    'process': async (exposed, _model, title, theme) => {
        const sequence = extractSequenceModel(exposed);
        return {
            svg: renderSequenceSvg(sequence, title, theme),
            summary: `${sequence.lifelines.length} lifeline(s), ${sequence.messages.length} message(s)`
        };
    }
};

/** Renders a view as the diagram its kind calls for, of what it exposes. */
export function renderView(view: ViewUsage, model: Model, title: DiagramTitle, theme: Theme = lightTheme): Promise<RenderResult> {
    return renderers[diagramKind(view)](exposedElements(view), model, title, theme);
}

export interface RenderedView {
    view: ViewUsage;
    /** The file it is written to, named after the view. */
    file: string;
    result: RenderResult;
}

export interface ModelRendering {
    rendered: RenderedView[];
    /** The views ortho does not draw, and why. */
    skipped: Array<{ view: ViewUsage; reason: string }>;
}

/**
 * Renders every view a model declares, each to a file named after it, headed
 * by its name and naming the file that declares it. A view ortho does not
 * draw is passed over with the reason; two views that would be written to the
 * same file are an error.
 */
export async function renderViews(parsed: ParsedModel, theme: Theme = lightTheme): Promise<ModelRendering> {
    const skipped: ModelRendering['skipped'] = [];
    const byFile = new Map<string, ViewUsage>();
    for (const view of parsed.documents.flatMap(document => declaredViews(document.parseResult.value))) {
        const reason = whyUndrawable(view);
        const file = viewFileName(view);
        const other = byFile.get(file);
        if (reason) {
            skipped.push({ view, reason });
        } else if (other) {
            throw new Error(`'${viewHeading(other)}' and '${viewHeading(view)}' would both be written to ${file}; `
                + 'give one a short name, view <name> \'...\'.');
        } else {
            byFile.set(file, view);
        }
    }
    const rendered: RenderedView[] = [];
    for (const [file, view] of byFile) {
        const source = relative(process.cwd(), AstUtils.getDocument(view).uri.fsPath).replace(/\\/g, '/');
        const title = { heading: view.name ?? '', definition: view.definition, source };
        rendered.push({ view, file, result: await renderView(view, parsed.model, title, theme) });
    }
    return { rendered, skipped };
}
