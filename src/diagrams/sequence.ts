import { AstUtils } from 'langium';
import { isActorUsage, isMessageUsage } from '../generated/ast.js';
import type { MessageUsage, Model, PartDef, PartUsage } from '../generated/ast.js';
import { qualifiedName } from '../model/graph.js';
import { endPart, partLabel } from './connector-ends.js';

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
        return name ? `${name} : ${message.payload}` : message.payload;
    }
    return name || 'message';
}

/**
 * A lifeline stands for a person when its part is untyped — a scenario names
 * its human participant without giving it a software type — or when it is
 * typed by a part def that a use case in the loaded model casts as an actor.
 */
function isPerson(part: PartUsage, actorTypes: Set<PartDef>): boolean {
    return !part.type || (part.type.ref !== undefined && actorTypes.has(part.type.ref));
}

/**
 * Lifelines are the parts referenced by message ends, in order of first
 * appearance; messages keep document order (`then` is explicit notation for
 * the same ordering).
 */
export function extractSequenceModel(model: Model): SequenceModel {
    const lifelines = new Map<string, SequenceLifeline>();
    const messages: SequenceMessage[] = [];
    const contents = [...AstUtils.streamAllContents(model)];
    const actorTypes = new Set(
        contents.filter(isActorUsage).map(a => a.type?.ref).filter((d): d is PartDef => !!d)
    );

    for (const node of contents) {
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
                lifelines.set(id, { id, label: partLabel(part), actor: isPerson(part, actorTypes) });
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
