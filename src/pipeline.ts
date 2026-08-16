import type { Model } from './generated/ast.js';
import type { DiagramGraph } from './model/graph.js';
import { extractPartDefinitionGraph } from './diagrams/part-definition.js';
import { extractUseCaseGraph } from './diagrams/use-case.js';
import { extractPackageGraph } from './diagrams/package.js';
import { extractAllocationGraph } from './diagrams/allocation.js';
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
    /** 4+1 architectural view this diagram serves; leads the frame heading. */
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

export const diagramTypes = {
    'part-definition': { view: 'Logical View', render: graphDiagram(extractPartDefinitionGraph, 'DOWN') },
    'use-case': { view: 'Scenarios', render: graphDiagram(extractUseCaseGraph, 'RIGHT') },
    'package': { view: 'Development View', render: graphDiagram(extractPackageGraph, 'DOWN') },
    'allocation': { view: 'Physical View', render: graphDiagram(extractAllocationGraph, 'DOWN') },
    'sequence': {
        view: 'Process View',
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
