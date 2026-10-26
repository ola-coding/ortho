import type { AstNode } from 'langium';
import { isConnectionUsage, isPartDef, isPartUsage, isPerformUsage, isPortUsage } from '../generated/ast.js';
import type {
    ConnectionUsage, ConnectorEnd, Definition, Member, Multiplicity, PartUsage, PortUsage
} from '../generated/ast.js';
import type { Compartment, DiagramGraph, GraphEdge, GraphNode } from '../model/graph.js';
import { boxOf, effectiveName, qualifiedName } from '../model/graph.js';
import { boxSize, usageLabel, performCompartments } from './connector-ends.js';

const HEADER_HEIGHT = 40;
const MIN_WIDTH = 110;

/** One part of the product: a box on the diagram, and the parts nested in it. */
interface Instance {
    /** Undefined only for a package, which owns top-level parts but draws no box. */
    node?: GraphNode;
    /** The usage this box stands for; undefined for a top-level definition. */
    usage?: PartUsage;
    children: Map<string, Instance>;
}

/** A connection, together with the part whose body declares it. */
interface Pending {
    connection: ConnectionUsage;
    context: Instance;
}

function multiplicityText(multiplicity: Multiplicity | undefined): string {
    if (!multiplicity) {
        return '';
    }
    return multiplicity.upper !== undefined
        ? ` [${multiplicity.lower}..${multiplicity.upper}]`
        : ` [${multiplicity.lower}]`;
}

/**
 * First of each name wins, so a part's own members shadow what its type
 * declares; a redefinition, `part :>> engine`, takes the name it redefines.
 */
function uniqueByName<T extends AstNode>(items: T[]): T[] {
    const seen = new Set<string>();
    return items.filter(item => {
        const name = effectiveName(item);
        return !name || (!seen.has(name) && seen.add(name));
    });
}

/**
 * The members a definition declares or inherits through `:>`. A local member
 * shadows an inherited one of the same name, as it does in name resolution.
 */
function effectiveMembers<T extends Member>(
    def: Definition, pick: (member: Member) => member is T, seen = new Set<Definition>()
): T[] {
    if (seen.has(def)) {
        return [];
    }
    seen.add(def);
    const own = def.members.filter(pick);
    // Only a part def's supertypes are resolved; the rest are kept as written.
    const supertypes = isPartDef(def) ? def.supertypes.map(s => s.ref) : [];
    const inherited = supertypes.flatMap(s => (s ? effectiveMembers(s, pick, seen) : []));
    return uniqueByName([...own, ...inherited]);
}

/** A «part» box: `name : Type [n]`, its ports, and what it performs. */
function partBox(id: string, part: PartUsage, type: Definition | undefined): GraphNode {
    const name = `${usageLabel(part)}${multiplicityText(part.multiplicities[0])}`;
    const performs = performsOf(type, part.members);
    const { shape, keyword } = boxOf(part);
    return {
        id,
        shape,
        keyword,
        name,
        compartments: performs,
        ports: portsOf(type, part.members).map(p => ({ id: `${id}#${effectiveName(p) ?? ''}`, label: effectiveName(p) ?? '' })),
        ...boxSize(name, performs, MIN_WIDTH, HEADER_HEIGHT, keyword)
    };
}

function portsOf(type: Definition | undefined, body: Member[]): PortUsage[] {
    return uniqueByName([...body.filter(isPortUsage), ...(type ? effectiveMembers(type, isPortUsage) : [])]);
}

/** What a part performs: what its body says, plus what its type contributes. */
function performsOf(type: Definition | undefined, body: Member[]): Compartment[] {
    return performCompartments([...body, ...(type ? effectiveMembers(type, isPerformUsage) : [])]);
}

/**
 * Adds a nested box for every part the owner's type declares or inherits, and
 * every part in the usage's own body, recursively. `path` holds the types
 * already being expanded, so a definition that contains itself is drawn once.
 */
function expand(owner: Instance, type: Definition | undefined, body: Member[], path: Set<Definition>, pending: Pending[]): void {
    const parts = uniqueByName([...body.filter(isPartUsage), ...(type ? effectiveMembers(type, isPartUsage) : [])]);
    for (const part of parts) {
        const partType = part.type?.ref;
        const id = `${owner.node!.id}.${effectiveName(part) ?? ''}`;
        const child: Instance = { node: partBox(id, part, partType), usage: part, children: new Map() };
        owner.children.set(effectiveName(part) ?? '', child);
        (owner.node!.children ??= []).push(child.node!);
        if (partType && path.has(partType)) {
            continue;
        }
        expand(child, partType, part.members, partType ? new Set([...path, partType]) : path, pending);
    }
    const connections = [...body.filter(isConnectionUsage), ...(type ? effectiveMembers(type, isConnectionUsage) : [])];
    for (const connection of connections) {
        pending.push({ connection, context: owner });
    }
}

/**
 * Walks a connector end (`drone.rfLink`, `radio.rf`) from the part whose body
 * declares the connection down to the box it names, and the port on it.
 * Resolving per instance is the point: the drone's radio and the ground
 * station's radio share a type but are two boxes, each wired on its own.
 */
function resolveEnd(end: ConnectorEnd | undefined, context: Instance): { nodeId: string; portId?: string } | undefined {
    let current = context;
    for (const segment of end?.segments ?? []) {
        const feature = segment.ref;
        if (isPartUsage(feature)) {
            const child = current.children.get(effectiveName(feature) ?? '');
            if (!child || child.usage !== feature) {
                return undefined;
            }
            current = child;
        } else if (isPortUsage(feature) && current.node) {
            const portId = `${current.node.id}#${effectiveName(feature) ?? ''}`;
            return current.node.ports.some(p => p.id === portId)
                ? { nodeId: current.node.id, portId }
                : { nodeId: current.node.id };
        } else {
            break;
        }
    }
    return current.node ? { nodeId: current.node.id } : undefined;
}

/**
 * The Physical view is the product's topology, drawn as an internal block
 * diagram: one box per part *in the product*, nested inside the part that
 * contains it, with ports on the box edges and a line only where something is
 * connected, cabled or piped. Containment is shown by nesting, so there are no
 * composition edges, and no attribute compartments — the view is about what
 * connects to what, not about specification values.
 *
 * Drawing instances rather than definitions matters: a type used twice (the
 * same IP radio in the drone and in the ground station) becomes two boxes, and
 * each connection lands on the part it actually names.
 *
 * The top-level boxes are the exposed parts.
 */
export function extractPhysicalGraph(exposed: Member[]): DiagramGraph {
    const nodes: GraphNode[] = [];
    const pending: Pending[] = [];
    for (const usage of exposed.filter(isPartUsage)) {
        const type = usage.type?.ref;
        const root: Instance = { node: partBox(qualifiedName(usage), usage, type), usage, children: new Map() };
        expand(root, type, usage.members, type ? new Set([type]) : new Set(), pending);
        nodes.push(root.node!);
    }

    const edges: GraphEdge[] = [];
    for (const { connection, context } of pending) {
        const source = resolveEnd(connection.source, context);
        const target = resolveEnd(connection.target, context);
        if (!source || !target || (source.nodeId === target.nodeId && source.portId === target.portId)) {
            continue;
        }
        edges.push({
            id: `e${edges.length}`,
            kind: 'connection',
            sourceId: source.nodeId,
            targetId: target.nodeId,
            sourcePortId: source.portId,
            targetPortId: target.portId,
            // A typed connection carries its interface name on the line.
            label: connection.type?.ref?.name ?? connection.name
        });
    }

    return { nodes, edges };
}
