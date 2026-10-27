import type { AstNode } from 'langium';
import { isPartUsage, isPerformUsage, isPortUsage } from '../generated/ast.js';
import type { ConnectorEnd, Member, PartUsage, PerformUsage } from '../generated/ast.js';
import type { Compartment } from '../model/graph.js';
import { BOX_HEADER_HEIGHT, compartmentsHeight, effectiveName } from '../model/graph.js';
import { measureText } from '../render/text-metrics.js';

/**
 * Resolves a connector end to the part usage it names. A trailing port
 * segment falls back to the part that owns the port.
 */
export function endPart(end: ConnectorEnd | undefined): PartUsage | undefined {
    for (let i = (end?.segments.length ?? 0) - 1; end && i >= 0; i--) {
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

type Typed = { type?: { ref?: { name?: string }; $refText: string } };

/** The type a usage names, as written. */
export function typeName(usage: Typed): string | undefined {
    return usage.type && (usage.type.ref?.name ?? usage.type.$refText);
}

/** A declaration as written (§8.2.3.6): `name : Type`, or the name or `: Type` alone. */
export function declaration(name: string, type: string | undefined): string {
    return type === undefined ? name : name ? `${name} : ${type}` : `: ${type}`;
}

/** A usage as declared: `name : Type`, or its name or `: Type` alone. */
export function usageLabel(usage: AstNode & Typed): string {
    return declaration(effectiveName(usage) ?? '', typeName(usage));
}

/**
 * The functions a part performs, as the spec's *perform actions* compartment
 * (§7.17.6): one line per `perform`, named by the function it reaches. This is
 * how realization is drawn — on the part that realizes, not as an edge.
 */
export function performCompartments(members: Member[]): Compartment[] {
    const lines = members.filter(isPerformUsage).map(performedName);
    return lines.length > 0 ? [{ title: 'perform actions', lines }] : [];
}

function performedName(perform: PerformUsage): string {
    const last = perform.target?.segments.at(-1);
    return last?.ref?.name ?? last?.$refText ?? effectiveName(perform) ?? '';
}

/** A box's size once its keyword, name and compartments have to fit inside it. */
export function boxSize(
    name: string, compartments: Compartment[], minWidth: number, plainHeight: number, keyword = ''
): { width: number; height: number } {
    const lineWidths = compartments.flatMap(c => [
        ...(c.title ? [measureText(c.title, 9) + 16] : []),
        ...c.lines.map(line => measureText(line, 11) + 16)
    ]);
    return {
        width: Math.max(minWidth, measureText(name, 12, 'bold') + 28, measureText(`«${keyword}»`, 9) + 28, ...lineWidths),
        height: compartments.length > 0
            ? BOX_HEADER_HEIGHT + compartmentsHeight(compartments)
            : plainHeight
    };
}