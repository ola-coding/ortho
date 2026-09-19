import { AstUtils } from 'langium';
import { isConnectionUsage, isPartDef, isPartUsage, isPerformUsage, isPortUsage } from '../generated/ast.js';
import type {
    ConnectionUsage, ConnectorEnd, Model, Multiplicity, PartDef, PartUsage, PortUsage, UsageMember
} from '../generated/ast.js';
import type { Compartment, DiagramGraph, GraphEdge, GraphNode } from '../model/graph.js';
import { qualifiedName } from '../model/graph.js';
import { collectPackages } from '../model/packages.js';
import { boxSize, partLabel, performCompartments } from './connector-ends.js';
import { measureText } from '../render/text-metrics.js';

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

/** First of each name wins, so a part's own members shadow what its type declares. */
function uniqueByName<T extends { name?: string }>(items: T[]): T[] {
    const seen = new Set<string>();
    return items.filter(item => !item.name || (!seen.has(item.name) && seen.add(item.name)));
}

/**
 * The members a definition declares or inherits through `:>`. A local member
 * shadows an inherited one of the same name, as it does in name resolution.
 */
function effectiveMembers<T extends UsageMember & { name?: string }>(
    def: PartDef, pick: (member: UsageMember) => member is T, seen = new Set<PartDef>()
): T[] {
    if (seen.has(def)) {
        return [];
    }
    seen.add(def);
    const own = def.members.filter(pick);
    const inherited = def.supertypes.flatMap(s => (s.ref ? effectiveMembers(s.ref, pick, seen) : []));
    return uniqueByName([...own, ...inherited]);
}

function partBox(id: string, name: string, ports: PortUsage[], performs: Compartment[] = []): GraphNode {
    return {
        id,
        shape: 'box',
        stereotype: '',
        name,
        compartments: performs,
        ports: ports.map(p => ({ id: `${id}#${p.name}`, label: p.name })),
        ...boxSize(name, performs, MIN_WIDTH, HEADER_HEIGHT)
    };
}

function portsOf(type: PartDef | undefined, body: UsageMember[]): PortUsage[] {
    return uniqueByName([...body.filter(isPortUsage), ...(type ? effectiveMembers(type, isPortUsage) : [])]);
}

/** What a part performs: what its body says, plus what its type contributes. */
function performsOf(type: PartDef | undefined, body: UsageMember[]): Compartment[] {
    return performCompartments([...body, ...(type ? effectiveMembers(type, isPerformUsage) : [])]);
}

/**
 * Adds a nested box for every part the owner's type declares or inherits, and
 * every part in the usage's own body, recursively. `path` holds the types
 * already being expanded, so a definition that contains itself is drawn once.
 */
function expand(owner: Instance, type: PartDef | undefined, body: UsageMember[], path: Set<PartDef>, pending: Pending[]): void {
    const parts = uniqueByName([...body.filter(isPartUsage), ...(type ? effectiveMembers(type, isPartUsage) : [])]);
    for (const part of parts) {
        const partType = part.type?.ref;
        const id = `${owner.node!.id}.${part.name}`;
        const child: Instance = {
            node: partBox(
                id,
                `${partLabel(part)}${multiplicityText(part.multiplicity)}`,
                portsOf(partType, part.members),
                performsOf(partType, part.members)
            ),
            usage: part,
            children: new Map()
        };
        owner.children.set(part.name, child);
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
function resolveEnd(end: ConnectorEnd, context: Instance): { nodeId: string; portId?: string } | undefined {
    let current = context;
    for (const segment of end.segments) {
        const feature = segment.ref;
        if (isPartUsage(feature)) {
            const child = current.children.get(feature.name);
            if (!child || child.usage !== feature) {
                return undefined;
            }
            current = child;
        } else if (isPortUsage(feature) && current.node) {
            const portId = `${current.node.id}#${feature.name}`;
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
 * The top-level boxes are the definitions nothing else uses as a part type and
 * that no other definition specializes, plus any package-level part usages.
 */
export function extractPhysicalGraph(model: Model): DiagramGraph {
    const packages = collectPackages(model);
    const defs = packages.flatMap(pkg => pkg.members.filter(isPartDef));

    const usedAsType = new Set<PartDef>();
    for (const node of AstUtils.streamAllContents(model)) {
        if (isPartUsage(node) && node.type?.ref) {
            usedAsType.add(node.type.ref);
        }
    }
    const supertypes = new Set(defs.flatMap(d => d.supertypes.map(s => s.ref)).filter((d): d is PartDef => !!d));

    const nodes: GraphNode[] = [];
    const pending: Pending[] = [];

    for (const def of defs) {
        if (usedAsType.has(def) || supertypes.has(def)) {
            continue;
        }
        const root: Instance = {
            node: partBox(qualifiedName(def), def.name, portsOf(def, []), performsOf(def, [])),
            children: new Map()
        };
        expand(root, def, [], new Set([def]), pending);
        nodes.push(root.node!);
    }

    for (const pkg of packages) {
        const scope: Instance = { children: new Map() };
        for (const usage of pkg.members.filter(isPartUsage)) {
            const type = usage.type?.ref;
            const root: Instance = {
                node: partBox(
                    qualifiedName(usage),
                    `${partLabel(usage)}${multiplicityText(usage.multiplicity)}`,
                    portsOf(type, usage.members),
                    performsOf(type, usage.members)
                ),
                usage,
                children: new Map()
            };
            expand(root, type, usage.members, type ? new Set([type]) : new Set(), pending);
            scope.children.set(usage.name, root);
            nodes.push(root.node!);
        }
        for (const connection of pkg.members.filter(isConnectionUsage)) {
            pending.push({ connection, context: scope });
        }
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
