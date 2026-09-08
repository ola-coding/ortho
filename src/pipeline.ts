import type { Model } from './generated/ast.js';
import type { DiagramGraph } from './model/graph.js';
import { extractUseCaseGraph } from './diagrams/use-case.js';
import { extractLogicalGraph } from './diagrams/logical.js';
import { extractImplementationGraph } from './diagrams/implementation.js';
import { extractPhysicalGraph } from './diagrams/physical.js';
import { extractDeploymentGraph } from './diagrams/deployment.js';
import { extractSequenceModel } from './diagrams/sequence.js';
import { layoutGraph } from './layout/elk-layout.js';
import { renderSvg } from './render/svg-renderer.js';
import type { DiagramTitle } from './render/svg-renderer.js';
import { renderSequenceSvg } from './render/sequence-renderer.js';

export interface RenderResult {
    svg: string;
    summary: string;
}

export type DiagramRenderer = (model: Model, title: DiagramTitle) => Promise<RenderResult>;

export interface DiagramType {
    /** Architectural view this diagram serves; leads the frame heading. */
    view: string;
    render: DiagramRenderer;
}

function graphDiagram(extract: (model: Model) => DiagramGraph, direction: 'DOWN' | 'RIGHT'): DiagramRenderer {
    return async (model, title) => {
        const graph = extract(model);
        const laidOut = await layoutGraph(graph, { direction });
        return {
            svg: renderSvg(laidOut, title),
            summary: `${laidOut.nodes.length} node(s), ${graph.edges.length} edge(s)`
        };
    };
}

/** The six views, in the order VIEWS.md introduces them. */
export const diagramTypes = {
    'use-case': { view: 'Use case view', render: graphDiagram(extractUseCaseGraph, 'RIGHT') },
    'logical': { view: 'Logical view', render: graphDiagram(extractLogicalGraph, 'DOWN') },
    'implementation': { view: 'Implementation view', render: graphDiagram(extractImplementationGraph, 'DOWN') },
    'physical': { view: 'Physical view', render: graphDiagram(extractPhysicalGraph, 'DOWN') },
    'deployment': { view: 'Deployment view', render: graphDiagram(extractDeploymentGraph, 'DOWN') },
    'process': {
        view: 'Process view',
        render: async (model, title) => {
            const sequence = extractSequenceModel(model);
            return {
                svg: renderSequenceSvg(sequence, title),
                summary: `${sequence.lifelines.length} lifeline(s), ${sequence.messages.length} message(s)`
            };
        }
    }
} satisfies Record<string, DiagramType>;

export type DiagramTypeName = keyof typeof diagramTypes;

export function getDiagramType(name: string): DiagramType | undefined {
    return name in diagramTypes ? diagramTypes[name as DiagramTypeName] : undefined;
}
