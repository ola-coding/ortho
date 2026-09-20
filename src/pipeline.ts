import type { Model } from './generated/ast.js';
import type { DiagramGraph } from './model/graph.js';
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

export type DiagramRenderer = (model: Model, title: DiagramTitle, theme?: Theme) => Promise<RenderResult>;

export interface DiagramType {
    /** Architectural view this diagram serves; leads the frame heading. */
    view: string;
    render: DiagramRenderer;
}

function graphDiagram(extract: (model: Model) => DiagramGraph, layout: LayoutOptions): DiagramRenderer {
    return async (model, title, theme = lightTheme) => {
        const graph = extract(model);
        const laidOut = await layoutGraph(graph, layout);
        return {
            svg: renderSvg(laidOut, title, theme),
            summary: `${laidOut.nodes.length} node(s), ${graph.edges.length} edge(s)`
        };
    };
}

/** Laid out by ortho itself rather than ELK: a tree's shape is fully determined. */
function treeDiagram(extract: (model: Model) => DiagramGraph): DiagramRenderer {
    return async (model, title, theme = lightTheme) => {
        const graph = extract(model);
        const laidOut = layoutTree(graph);
        return {
            svg: renderSvg(laidOut, title, theme),
            summary: `${laidOut.nodes.length} node(s), ${graph.edges.length} edge(s)`
        };
    };
}

/** The six views, in the order VIEWS.md introduces them. */
export const diagramTypes = {
    'use-case': {
        view: 'Use case view',
        render: graphDiagram(extractUseCaseGraph, { direction: 'RIGHT', edgeRouting: 'STRAIGHT' })
    },
    'logical': { view: 'Logical view', render: treeDiagram(extractLogicalGraph) },
    'implementation': {
        view: 'Implementation view',
        render: graphDiagram(extractImplementationGraph, { direction: 'DOWN', gridLeaves: true })
    },
    'physical': { view: 'Physical view', render: graphDiagram(extractPhysicalGraph, { direction: 'DOWN' }) },
    'deployment': { view: 'Deployment view', render: graphDiagram(extractDeploymentGraph, { algorithm: 'rectpacking' }) },
    'process': {
        view: 'Process view',
        render: async (model, title, theme = lightTheme) => {
            const sequence = extractSequenceModel(model);
            return {
                svg: renderSequenceSvg(sequence, title, theme),
                summary: `${sequence.lifelines.length} lifeline(s), ${sequence.messages.length} message(s)`
            };
        }
    }
} satisfies Record<string, DiagramType>;

export type DiagramTypeName = keyof typeof diagramTypes;

export function getDiagramType(name: string): DiagramType | undefined {
    return name in diagramTypes ? diagramTypes[name as DiagramTypeName] : undefined;
}
