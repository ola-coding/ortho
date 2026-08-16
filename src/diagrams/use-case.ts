import {
    isActorUsage, isIncludeUsage, isSubjectUsage, isUseCaseDef, isUseCaseUsage
} from '../generated/ast.js';
import type {
    ActorUsage, IncludeUsage, Model, SubjectUsage, UseCaseDef, UseCaseElement, UseCaseMember, UseCaseUsage
} from '../generated/ast.js';
import type { DiagramGraph, GraphEdge, GraphNode } from '../model/graph.js';
import { qualifiedName } from '../model/graph.js';
import { collectPackages } from '../model/packages.js';
import { measureText } from '../render/text-metrics.js';

type UseCase = UseCaseDef | UseCaseUsage;

/** Own members plus, for a typed usage, the members inherited from its def. */
function effectiveMembers(useCase: UseCase): UseCaseMember[] {
    const members: UseCaseMember[] = [...useCase.members];
    if (isUseCaseUsage(useCase) && useCase.type?.ref) {
        members.push(...useCase.type.ref.members);
    }
    return members;
}

function ellipseNode(useCase: UseCase): GraphNode {
    const name = useCase.name;
    return {
        id: qualifiedName(useCase),
        shape: 'ellipse',
        stereotype: '',
        name,
        compartments: [],
        ports: [],
        width: Math.max(100, measureText(name, 12) + 56),
        height: 50
    };
}

function actorNode(id: string, label: string): GraphNode {
    return {
        id,
        shape: 'actor',
        stereotype: '',
        name: label,
        compartments: [],
        ports: [],
        width: Math.max(48, measureText(label, 11) + 10),
        height: 76
    };
}

export function extractUseCaseGraph(model: Model): DiagramGraph {
    const useCases: UseCase[] = [];
    for (const pkg of collectPackages(model)) {
        for (const member of pkg.members) {
            if (isUseCaseDef(member) || isUseCaseUsage(member)) {
                useCases.push(member);
            }
        }
    }

    // A def represented by usages is not shown itself.
    const typedDefs = new Set(
        useCases.filter(isUseCaseUsage).map(u => u.type?.ref).filter((d): d is UseCaseDef => !!d)
    );
    const rendered = useCases.filter(uc => !(isUseCaseDef(uc) && typedDefs.has(uc)));
    const ellipseByUseCase = new Map<UseCase, GraphNode>(rendered.map(uc => [uc, ellipseNode(uc)]));

    // Group use cases into system boundaries by subject type.
    const rootNodes: GraphNode[] = [];
    const boundaries = new Map<string, GraphNode>();
    const actorNodes = new Map<string, GraphNode>();
    const edges: GraphEdge[] = [];
    const associationSeen = new Set<string>();
    let edgeCounter = 0;

    for (const [useCase, node] of ellipseByUseCase) {
        const members = effectiveMembers(useCase);
        const subject = members.find(isSubjectUsage);
        const subjectLabel = subject
            ? subject.type?.ref?.name ?? subject.type?.$refText ?? subject.name
            : undefined;
        if (subjectLabel) {
            let boundary = boundaries.get(subjectLabel);
            if (!boundary) {
                boundary = {
                    id: `boundary:${subjectLabel}`,
                    shape: 'boundary',
                    stereotype: '',
                    name: subjectLabel,
                    compartments: [],
                    ports: [],
                    children: [],
                    width: 0,
                    height: 0
                };
                boundaries.set(subjectLabel, boundary);
                rootNodes.push(boundary);
            }
            boundary.children!.push(node);
        } else {
            rootNodes.push(node);
        }

        for (const actor of members.filter(isActorUsage)) {
            const key = actor.type?.ref ? qualifiedName(actor.type.ref) : actor.name;
            const label = actor.type?.ref?.name ?? actor.type?.$refText ?? actor.name;
            let actorGraphNode = actorNodes.get(key);
            if (!actorGraphNode) {
                actorGraphNode = actorNode(`actor:${key}`, label);
                actorNodes.set(key, actorGraphNode);
                rootNodes.push(actorGraphNode);
            }
            const pairKey = `${actorGraphNode.id}->${node.id}`;
            if (!associationSeen.has(pairKey)) {
                associationSeen.add(pairKey);
                edges.push({
                    id: `e${edgeCounter++}`,
                    kind: 'association',
                    sourceId: actorGraphNode.id,
                    targetId: node.id
                });
            }
        }
    }

    // Include edges: a target def represented by usages maps to its first usage.
    const nodeForTarget = (target: UseCaseElement): GraphNode | undefined => {
        const direct = ellipseByUseCase.get(target);
        if (direct) {
            return direct;
        }
        if (isUseCaseDef(target)) {
            const usage = rendered.find(uc => isUseCaseUsage(uc) && uc.type?.ref === target);
            return usage ? ellipseByUseCase.get(usage) : undefined;
        }
        return undefined;
    };

    for (const [useCase, node] of ellipseByUseCase) {
        for (const include of effectiveMembers(useCase).filter(isIncludeUsage)) {
            const target = include.target.ref && nodeForTarget(include.target.ref);
            if (target && target !== node) {
                edges.push({
                    id: `e${edgeCounter++}`,
                    kind: 'include',
                    sourceId: node.id,
                    targetId: target.id,
                    label: 'include'
                });
            }
        }
    }

    return { nodes: rootNodes, edges };
}
