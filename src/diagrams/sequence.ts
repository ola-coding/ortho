import { AstUtils } from 'langium';
import { isMessageUsage } from '../generated/ast.js';
import type { MessageUsage, Model } from '../generated/ast.js';
import { qualifiedName } from '../model/graph.js';
import { endPart, partLabel } from './connector-ends.js';

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

function messageLabel(message: MessageUsage): string {
    const name = message.name ?? '';
    if (message.payload) {
        return name ? `${name} : ${message.payload}` : message.payload;
    }
    return name || 'message';
}

/**
 * Lifelines are the parts referenced by message ends, in order of first
 * appearance; messages keep document order (`then` is explicit notation for
 * the same ordering).
 */
export function extractSequenceModel(model: Model): SequenceModel {
    const lifelines = new Map<string, SequenceLifeline>();
    const messages: SequenceMessage[] = [];

    for (const node of AstUtils.streamAllContents(model)) {
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
                lifelines.set(id, { id, label: partLabel(part) });
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
