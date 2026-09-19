import { isPackageDecl, isPartDef, isPartUsage } from '../generated/ast.js';
import type { Model, PackageDecl, PartDef } from '../generated/ast.js';
import type { DiagramGraph, GraphEdge, GraphNode } from '../model/graph.js';
import { qualifiedName } from '../model/graph.js';
import { boxSize, performCompartments } from './connector-ends.js';
import { collectPackages } from '../model/packages.js';
import { measureText } from '../render/text-metrics.js';

function moduleNode(def: PartDef): GraphNode {
    const compartments = performCompartments(def.members);
    return {
        id: qualifiedName(def),
        shape: 'box',
        stereotype: '',
        name: def.name,
        compartments,
        ports: [],
        ...boxSize(def.name, compartments, 120, 40)
    };
}

/**
 * A package this view draws: one holding something part-like, a module or a
 * deployable part, at any depth. The view is rendered with `logical.sysml`
 * alongside, because the modules name the functions they perform, and a
 * package of functions — or of requirements — is not software.
 */
function holdsParts(pkg: PackageDecl): boolean {
    return pkg.members.some(member =>
        isPartDef(member) || isPartUsage(member) || (isPackageDecl(member) && holdsParts(member)));
}

/**
 * A package box, with its sub-packages and the modules it declares drawn
 * inside it. A module is a part def. A package-level part usage is an
 * instance — the tree of deployable parts the deployment view allocates
 * from — not a module, so it is left off this view.
 */
function packageNode(pkg: PackageDecl): GraphNode {
    const children: GraphNode[] = [];
    for (const member of pkg.members) {
        if (isPackageDecl(member) && holdsParts(member)) {
            children.push(packageNode(member));
        } else if (isPartDef(member)) {
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
    const drawn = model.packages.filter(holdsParts);
    const nodes = drawn.map(packageNode);

    // Imports are drawn between the packages on this view; one reaching
    // outside it, such as a module's import of the function tree, or a
    // package of requirements, is not.
    const allPackages = collectPackages(model).filter(holdsParts);
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
