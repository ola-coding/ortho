import { isPackageDecl, isPartDef, isPartUsage } from '../generated/ast.js';
import type { Model, PackageDecl, PartDef, PartUsage } from '../generated/ast.js';
import type { DiagramGraph, GraphEdge, GraphNode } from '../model/graph.js';
import { qualifiedName } from '../model/graph.js';
import { collectPackages } from '../model/packages.js';
import { measureText } from '../render/text-metrics.js';

function moduleNode(member: PartDef | PartUsage): GraphNode {
    const name = isPartUsage(member) && member.type
        ? `${member.name} : ${member.type.ref?.name ?? member.type.$refText}`
        : member.name;
    return {
        id: qualifiedName(member),
        shape: 'box',
        stereotype: '',
        name,
        compartments: [],
        ports: [],
        width: Math.max(120, measureText(name, 12, 'bold') + 28),
        height: 40
    };
}

/**
 * A package box, with its sub-packages and the modules it declares drawn
 * inside it. Only top-level part usages become modules — a part nested inside
 * another part is a detail of that module, not a module of its own.
 */
function packageNode(pkg: PackageDecl): GraphNode {
    const children: GraphNode[] = [];
    for (const member of pkg.members) {
        if (isPackageDecl(member)) {
            children.push(packageNode(member));
        } else if (isPartDef(member) || isPartUsage(member)) {
            children.push(moduleNode(member));
        }
    }
    return {
        id: qualifiedName(pkg),
        shape: 'package',
        stereotype: '',
        name: pkg.name,
        compartments: [],
        ports: [],
        children: children.length > 0 ? children : undefined,
        width: Math.max(130, measureText(pkg.name, 12, 'bold') + 50),
        height: 60
    };
}

/**
 * Resolves an import path the way references resolve: relative to the
 * importing package's ancestors first, then as an absolute qualified name.
 * `Pkg::Element` imports resolve to the containing package `Pkg`.
 */
function resolveImportTarget(importer: PackageDecl, path: string, packageIds: Set<string>): string | undefined {
    const prefixes: string[] = [];
    let scope: PackageDecl | undefined = importer;
    while (scope) {
        prefixes.push(qualifiedName(scope));
        scope = isPackageDecl(scope.$container) ? scope.$container : undefined;
    }
    prefixes.push('');

    for (const candidate of [path, path.split('::').slice(0, -1).join('::')]) {
        if (!candidate) {
            continue;
        }
        for (const prefix of prefixes) {
            const fqn = prefix ? `${prefix}::${candidate}` : candidate;
            if (packageIds.has(fqn)) {
                return fqn;
            }
        }
    }
    return undefined;
}

export function extractImplementationGraph(model: Model): DiagramGraph {
    const nodes = model.packages.map(packageNode);

    const allPackages = collectPackages(model);
    const packageIds = new Set(allPackages.map(qualifiedName));

    const edges: GraphEdge[] = [];
    const seen = new Set<string>();
    let edgeCounter = 0;
    for (const pkg of allPackages) {
        const sourceId = qualifiedName(pkg);
        for (const imp of pkg.imports) {
            const targetId = resolveImportTarget(pkg, imp.path, packageIds);
            if (targetId && targetId !== sourceId && !seen.has(`${sourceId}->${targetId}`)) {
                seen.add(`${sourceId}->${targetId}`);
                edges.push({
                    id: `e${edgeCounter++}`,
                    kind: 'import',
                    sourceId,
                    targetId,
                    label: 'import'
                });
            }
        }
    }

    return { nodes, edges };
}
