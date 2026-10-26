import {
    isActorUsage, isIncludeUsage, isSubjectUsage, isUseCaseDef, isUseCaseUsage
} from '../generated/ast.js';
import type {
    ActorUsage, Member, IncludeUsage, SubjectUsage, UseCaseDef, UseCaseUsage
} from '../generated/ast.js';
import type { DiagramGraph, GraphEdge, GraphNode } from '../model/graph.js';
import { boxOf, qualifiedName } from '../model/graph.js';
import { exposedMembers } from '../model/packages.js';
import { measureText } from '../render/text-metrics.js';
import { boxSize, usageLabel } from './connector-ends.js';

type UseCase = UseCaseDef | UseCaseUsage;

/** Own members plus, for a typed usage, the members inherited from its def. */
function effectiveMembers(useCase: UseCase): Member[] {
    const members: Member[] = [...useCase.members];
    if (isUseCaseUsage(useCase) && useCase.type?.ref) {
        members.push(...useCase.type.ref.members);
    }
    return members;
}

/** A «use case» box, named as declared, or a «use case def» one. */
function useCaseNode(useCase: UseCase): GraphNode {
    const name = isUseCaseUsage(useCase) ? usageLabel(useCase) : useCase.name ?? '';
    const { shape, keyword } = boxOf(useCase);
    return {
        id: qualifiedName(useCase),
        shape,
        keyword,
        name,
        compartments: [],
        ports: [],
        ...boxSize(name, [], 110, 40, keyword)
    };
}

/** The «subject» box, drawn once however many use cases share it. */
function subjectNode(subject: SubjectUsage, label: string): GraphNode {
    const { shape, keyword } = boxOf(subject);
    return {
        id: `subject:${label}`,
        shape,
        keyword,
        name: label,
        compartments: [],
        ports: [],
        ...boxSize(label, [], 110, 40, keyword)
    };
}

function actorNode(id: string, label: string): GraphNode {
    return {
        id,
        shape: 'actor',
        keyword: '',
        name: label,
        compartments: [],
        ports: [],
        width: Math.max(48, measureText(label, 11) + 10),
        height: 76
    };
}

/**
 * The use case view (§8.2.3.25): «use case» boxes, the «subject» and the
 * actors each joined to their use cases by a plain line (§8.2.3.21), and an
 * «include» arrow to each included use case.
 */
export function extractUseCaseGraph(exposed: Member[]): DiagramGraph {
    const useCases = exposedMembers(exposed).filter((m): m is UseCase => isUseCaseDef(m) || isUseCaseUsage(m));

    // A def represented by usages is not shown itself.
    const typedDefs = new Set(
        useCases.filter(isUseCaseUsage).map(u => u.type?.ref).filter((d): d is UseCaseDef => !!d)
    );
    const rendered = useCases.filter(uc => !(isUseCaseDef(uc) && typedDefs.has(uc)));
    const boxByUseCase = new Map<UseCase, GraphNode>(rendered.map(uc => [uc, useCaseNode(uc)]));

    const nodes: GraphNode[] = [...boxByUseCase.values()];
    const subjects = new Map<string, GraphNode>();
    const actorNodes = new Map<string, GraphNode>();
    const edges: GraphEdge[] = [];
    const joined = new Set<string>();
    let edgeCounter = 0;

    // A line has no direction and is drawn without an arrowhead; the edge's
    // direction only tells the layout which side of the use cases its other
    // end belongs on.
    const join = (sourceId: string, targetId: string): void => {
        if (!joined.has(`${sourceId}->${targetId}`)) {
            joined.add(`${sourceId}->${targetId}`);
            edges.push({ id: `e${edgeCounter++}`, kind: 'association', sourceId, targetId });
        }
    };

    // An actor declared on a use case def, and so shared by every use case of
    // that kind, is a primary actor, drawn on the left. One added on a single
    // use case only — someone the system acts on, not a user of it — is a
    // secondary actor, drawn on the right. The distinction needs at least one
    // actor to come through a def; otherwise every actor is primary.
    const actorKey = (actor: ActorUsage): string => (actor.type?.ref ? qualifiedName(actor.type.ref) : actor.name ?? '');
    const onDef = new Set<string>();
    for (const useCase of useCases) {
        if (isUseCaseDef(useCase)) {
            for (const actor of useCase.members.filter(isActorUsage)) {
                onDef.add(actorKey(actor));
            }
        }
    }
    const isSecondary = (key: string): boolean => onDef.size > 0 && !onDef.has(key);

    for (const [useCase, node] of boxByUseCase) {
        const members = effectiveMembers(useCase);
        // The subject stands on the right, across the use cases from the
        // primary actors.
        const subject = members.find(isSubjectUsage);
        if (subject) {
            const label = usageLabel(subject);
            let subjectBox = subjects.get(label);
            if (!subjectBox) {
                subjectBox = subjectNode(subject, label);
                subjects.set(label, subjectBox);
                nodes.push(subjectBox);
            }
            join(node.id, subjectBox.id);
        }

        for (const actor of members.filter(isActorUsage)) {
            const key = actorKey(actor);
            let actorGraphNode = actorNodes.get(key);
            if (!actorGraphNode) {
                actorGraphNode = actorNode(`actor:${key}`, usageLabel(actor));
                actorNodes.set(key, actorGraphNode);
                nodes.push(actorGraphNode);
            }
            if (isSecondary(key)) {
                join(node.id, actorGraphNode.id);
            } else {
                join(actorGraphNode.id, node.id);
            }
        }
    }

    // Include edges. The target is always a usage: SysML v2 includes a use
    // case, not a kind of use case.
    for (const [useCase, node] of boxByUseCase) {
        for (const include of effectiveMembers(useCase).filter(isIncludeUsage)) {
            const target = includedBox(include, boxByUseCase);
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

    return { nodes, edges };
}

/**
 * The box an include points at: `include x` names the use case, and
 * `include use case x` declares one, drawn as the use case of that name.
 */
function includedBox(include: IncludeUsage, boxByUseCase: Map<UseCase, GraphNode>): GraphNode | undefined {
    const included = include.target?.segments.at(-1)?.ref
        ?? [...boxByUseCase.keys()].find(u => isUseCaseUsage(u) && include.useCase && u.name === include.name);
    return isUseCaseUsage(included) ? boxByUseCase.get(included) : undefined;
}
