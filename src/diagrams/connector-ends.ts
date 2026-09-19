import { isPartUsage, isPerformUsage, isPortUsage } from '../generated/ast.js';
import type { ConnectorEnd, PartUsage, PerformUsage, UsageMember } from '../generated/ast.js';
import type { Compartment } from '../model/graph.js';
import { BOX_HEADER_HEIGHT, compartmentsHeight } from '../model/graph.js';
import { measureText } from '../render/text-metrics.js';

/**
 * Resolves a connector end to the part usage it names. A trailing port
 * segment falls back to the part that owns the port.
 */
export function endPart(end: ConnectorEnd): PartUsage | undefined {
    for (let i = end.segments.length - 1; i >= 0; i--) {
        const resolved = end.segments[i]?.ref;
        if (isPartUsage(resolved)) {
            return resolved;
        }
        if (isPortUsage(resolved) && isPartUsage(resolved.$container)) {
            return resolved.$container;
        }
    }
    return undefined;
}

export function partLabel(part: PartUsage): string {
    const type = part.type ? ` : ${part.type.ref?.name ?? part.type.$refText}` : '';
    return `${part.name}${type}`;
}

/**
 * The functions a part performs, as the spec's *perform actions* compartment
 * (§7.17.6): one line per `perform`, named by the function it reaches. This is
 * how realization is drawn — on the part that realizes, not as an edge.
 */
export function performCompartments(members: Array<UsageMember | PerformUsage>): Compartment[] {
    const lines = members.filter(isPerformUsage).map(performedName);
    return lines.length > 0 ? [{ title: 'perform actions', lines }] : [];
}

function performedName(perform: PerformUsage): string {
    const last = perform.target.segments[perform.target.segments.length - 1];
    return last?.ref?.name ?? last?.$refText ?? '';
}

/** A box's size once its compartments have to fit inside it. */
export function boxSize(
    name: string, compartments: Compartment[], minWidth: number, plainHeight: number
): { width: number; height: number } {
    const lineWidths = compartments.flatMap(c => [
        ...(c.title ? [measureText(c.title, 9) + 16] : []),
        ...c.lines.map(line => measureText(line, 11) + 16)
    ]);
    return {
        width: Math.max(minWidth, measureText(name, 12, 'bold') + 28, ...lineWidths),
        height: compartments.length > 0
            ? BOX_HEADER_HEIGHT + compartmentsHeight(compartments)
            : plainHeight
    };
}