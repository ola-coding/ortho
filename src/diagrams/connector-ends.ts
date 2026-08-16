import { isPartUsage, isPortUsage } from '../generated/ast.js';
import type { ConnectorEnd, PartUsage } from '../generated/ast.js';

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
