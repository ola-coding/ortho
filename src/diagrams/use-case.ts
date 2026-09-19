import {
    isActorUsage, isIncludeUsage, isSubjectUsage, isUseCaseDef, isUseCaseUsage
} from '../generated/ast.js';
import type {
    ActorUsage, IncludeUsage, Model, SubjectUsage, UseCaseDef, UseCaseMember, UseCaseUsage
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

    // An actor declared on a use case def, and so shared by every use case of
    // that kind, is a primary actor, drawn on the left. One added on a single
    // use case only — someone the system acts on, not a user of it — is a
    // secondary actor, drawn on the right. The distinction needs at least one
    // actor to come through a def; otherwise every actor is primary.
    const actorKey = (actor: ActorUsage): string => (actor.type?.ref ? qualifiedName(actor.type.ref) : actor.name);
    const onDef = new Set<string>();
    for (const useCase of useCases) {
        if (isUseCaseDef(useCase)) {
            for (const actor of useCase.members.filter(isActorUsage)) {
                onDef.add(actorKey(actor));
            }
        }
    }
    const isSecondary = (key: string): boolean => onDef.size > 0 && !onDef.has(key);

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
            const key = actorKey(actor);
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
                // An association has no direction and is drawn without an
                // arrowhead; the edge's direction only tells the layout which
                // side of the use cases the actor belongs on.
                const secondary = isSecondary(key);
                edges.push({
                    id: `e${edgeCounter++}`,
                    kind: 'association',
                    sourceId: secondary ? node.id : actorGraphNode.id,
                    targetId: secondary ? actorGraphNode.id : node.id
                });
            }
        }
    }

    // Include edges. The target is always a usage: SysML v2 includes a use
    // case, not a kind of use case.
    for (const [useCase, node] of ellipseByUseCase) {
        for (const include of effectiveMembers(useCase).filter(isIncludeUsage)) {
            const target = include.target.ref && ellipseByUseCase.get(include.target.ref);
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
