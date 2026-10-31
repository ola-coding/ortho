import {
    isActorUsage, isIncludeUsage, isSubjectUsage, isUseCaseDef, isUseCaseUsage
} from '../generated/ast.js';
import type {
    ActorUsage, Member, IncludeUsage, SubjectUsage, UseCaseDef, UseCaseUsage
} from '../generated/ast.js';
import type { DiagramGraph, GraphEdge, GraphNode } from '../model/graph.js';
import { boxOf, effectiveName, qualifiedName } from '../model/graph.js';
import { exposedMembers } from '../model/packages.js';
import { measureText } from '../render/text-metrics.js';
import { boxSize, declaration, typeName, usageLabel } from './connector-ends.js';

/** A use case drawn as a box. `include use case x` declares one (§7.25); `include x` only names one. */
type UseCase = UseCaseDef | UseCaseUsage | IncludeUsage;

/** Own members plus, for a typed usage, the members inherited from its def. */
function effectiveMembers(useCase: UseCase): Member[] {
    const members: Member[] = [...useCase.members];
    if (!isUseCaseDef(useCase) && useCase.type?.ref) {
        members.push(...useCase.type.ref.members);
    }
    return members;
}

/** A «use case» box, named as declared, or a «use case def» one. */
export function useCaseNode(useCase: UseCase): GraphNode {
    const name = isUseCaseDef(useCase) ? useCase.name ?? '' : usageLabel(useCase);
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
    const found = new Set(exposedMembers(exposed).filter((m): m is UseCase => isUseCaseDef(m) || isUseCaseUsage(m)));
    // A set visits what is added while it is walked, so this reaches what those
    // declare in turn.
    for (const useCase of found) {
        for (const member of effectiveMembers(useCase)) {
            if (isIncludeUsage(member) && member.useCase) {
                found.add(member);
            }
        }
    }
    const useCases = [...found];

    // A def represented by usages is not shown itself.
    const typedDefs = new Set(useCases.flatMap(u => (!isUseCaseDef(u) && u.type?.ref ? [u.type.ref] : [])));
    const rendered = useCases.filter(uc => !(isUseCaseDef(uc) && typedDefs.has(uc)));
    const boxByUseCase = new Map<UseCase, GraphNode>(rendered.map(uc => [uc, useCaseNode(uc)]));

    const nodes: GraphNode[] = [...boxByUseCase.values()];
    const subjects = new Map<string, GraphNode>();
    const actorNodes = new Map<string, GraphNode>();
    const edges: GraphEdge[] = [];
    const joined = new Set<string>();
    let edgeCounter = 0;

    // A line has no direction and is drawn without an arrowhead; the edge's
    // direction only tells the layout a secondary actor from a primary one.
    const join = (sourceId: string, targetId: string): void => {
        if (!joined.has(`${sourceId}->${targetId}`)) {
            joined.add(`${sourceId}->${targetId}`);
            edges.push({ id: `e${edgeCounter++}`, kind: 'association', sourceId, targetId });
        }
    };

    // An actor declared on a use case def, and so shared by every use case of
    // that kind, is a primary actor. One added on a single use case only —
    // someone the system acts on, not a user of it — is a secondary actor,
    // drawn after the primary ones. The distinction needs at least one actor
    // to come through a def; otherwise every actor is primary.
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
        // A use case restating its def's subject, `subject machine;`,
        // redefines it and keeps its type: both are `machine : CoffeeMachine`,
        // one box.
        const declared = members.filter(isSubjectUsage);
        if (declared.length > 0) {
            const label = declaration(
                declared.map(s => effectiveName(s)).find(name => name) ?? '',
                declared.map(typeName).find(type => type)
            );
            let subjectBox = subjects.get(label);
            if (!subjectBox) {
                subjectBox = subjectNode(declared[0], label);
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
                    label: '«include»'
                });
            }
        }
    }

    return { nodes, edges };
}

/** The box an include points at: its own, or that of the use case it names. */
function includedBox(include: IncludeUsage, boxByUseCase: ReadonlyMap<unknown, GraphNode>): GraphNode | undefined {
    return boxByUseCase.get(include.useCase ? include : include.target?.segments.at(-1)?.ref);
}
