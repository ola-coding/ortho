import { AstUtils } from 'langium';
import { isMessageUsage } from '../generated/ast.js';
import type { Member, MessageUsage } from '../generated/ast.js';
import { qualifiedName } from '../model/graph.js';
import { endPart, usageLabel } from './connector-ends.js';

/** A lifeline, headed by its part as declared: `uiApp : UiApp`, or a person's `user`. */
export interface SequenceLifeline {
    id: string;
    label: string;
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

/**
 * A message as declared (§8.2.3.16): `name of Payload`, its name or its
 * payload alone, or «message» for one with neither. The payload is as
 * written, so a qualified name stays qualified.
 */
function messageLabel(message: MessageUsage): string {
    const name = message.name ?? '';
    const payload = message.payload?.$refText;
    return payload ? (name ? `${name} of ${payload}` : payload) : name || '«message»';
}

/**
 * Lifelines are the parts referenced by message ends, in order of first
 * appearance, a person's as any other (§8.2.3.9); messages keep document
 * order (`then` is explicit notation for the same ordering).
 */
export function extractSequenceModel(exposed: Member[]): SequenceModel {
    const lifelines = new Map<string, SequenceLifeline>();
    const messages: SequenceMessage[] = [];

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
                lifelines.set(id, { id, label: usageLabel(part) });
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
