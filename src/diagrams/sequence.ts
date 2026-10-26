import { AstUtils } from 'langium';
import { isActorUsage, isMessageUsage } from '../generated/ast.js';
import type { Definition, Member, MessageUsage, Model, PartUsage } from '../generated/ast.js';
import { qualifiedName } from '../model/graph.js';
import { endPart, usageLabel } from './connector-ends.js';

export interface SequenceLifeline {
    id: string;
    label: string;
    /** A person rather than a software module: drawn as a stick figure. */
    actor: boolean;
}

export interface SequenceMessage {
    sourceId: string;
    targetId: string;
    label: string;
}

export interface SequenceModel {
    lifelines: SequenceLifeline[];
    messages: SequenceMessage[];
}

function messageLabel(message: MessageUsage): string {
    const name = message.name ?? '';
    if (message.payload) {
        // The payload as written, so a qualified name stays qualified.
        const payload = message.payload.$refText;
        return name ? `${name} : ${payload}` : payload;
    }
    return name || 'message';
}

/**
 * A lifeline stands for a person when its part is untyped — a scenario names
 * its human participant without giving it a software type — or when it is
 * typed by a part def that a use case in the loaded model casts as an actor.
 */
function isPerson(part: PartUsage, actorTypes: Set<Definition>): boolean {
    return !part.type || (part.type.ref !== undefined && actorTypes.has(part.type.ref));
}

/**
 * Lifelines are the parts referenced by message ends, in order of first
 * appearance; messages keep document order (`then` is explicit notation for
 * the same ordering).
 */
export function extractSequenceModel(exposed: Member[], model: Model): SequenceModel {
    const lifelines = new Map<string, SequenceLifeline>();
    const messages: SequenceMessage[] = [];
    // A part typed by a use case's actor, in any loaded file, is a person.
    const actorTypes = new Set(
        AstUtils.streamAllContents(model).filter(isActorUsage).map(a => a.type?.ref).filter((d): d is Definition => !!d)
    );

    for (const node of exposed.flatMap(e => [...AstUtils.streamAllContents(e)])) {
        if (!isMessageUsage(node)) {
            continue;
        }
        const source = endPart(node.source);
        const target = endPart(node.target);
        if (!source || !target) {
            continue;
        }
        for (const part of [source, target]) {
            const id = qualifiedName(part);
            if (!lifelines.has(id)) {
                lifelines.set(id, { id, label: usageLabel(part), actor: isPerson(part, actorTypes) });
            }
        }
        messages.push({
            sourceId: qualifiedName(source),
            targetId: qualifiedName(target),
            label: messageLabel(node)
        });
    }

    return { lifelines: [...lifelines.values()], messages };
}
