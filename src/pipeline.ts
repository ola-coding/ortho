import type { Member, Model, ViewUsage } from './generated/ast.js';
import type { DiagramGraph } from './model/graph.js';
import { diagramKind, exposedElements } from './model/views.js';
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

function graphDiagram(extract: Extractor, layout: LayoutOptions): Renderer {
    return async (exposed, model, title, theme) => {
        const graph = extract(exposed, model);
        const laidOut = await layoutGraph(graph, layout);
        return {
            svg: renderSvg(laidOut, title, theme),
            summary: `${laidOut.nodes.length} node(s), ${graph.edges.length} edge(s)`
        };
    };
}

/** Laid out by ortho itself rather than ELK: a tree's shape is fully determined. */
function treeDiagram(extract: Extractor): Renderer {
    return async (exposed, model, title, theme) => {
        const graph = extract(exposed, model);
        const laidOut = layoutTree(graph);
        return {
            svg: renderSvg(laidOut, title, theme),
            summary: `${laidOut.nodes.length} node(s), ${graph.edges.length} edge(s)`
        };
    };
}

/** The six diagrams, in the order VIEWS.md introduces them. */
const renderers: Record<DiagramKind, Renderer> = {
    'use-case': graphDiagram(extractUseCaseGraph, { direction: 'RIGHT', edgeRouting: 'STRAIGHT' }),
    'logical': treeDiagram(extractLogicalGraph),
    'implementation': graphDiagram(extractImplementationGraph, { direction: 'DOWN', gridLeaves: true }),
    'physical': graphDiagram(extractPhysicalGraph, { direction: 'DOWN' }),
    'deployment': graphDiagram(extractDeploymentGraph, { algorithm: 'rectpacking' }),
    'process': async (exposed, model, title, theme) => {
        const sequence = extractSequenceModel(exposed, model);
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
