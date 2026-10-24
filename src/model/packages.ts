import type { AstNode } from 'langium';
import { isPackageDecl } from '../generated/ast.js';
import type { Exposable, PackageDecl } from '../generated/ast.js';
import { qualifiedName } from './graph.js';

/** The given packages and every package nested in them. */
export function collectPackages(roots: PackageDecl[]): PackageDecl[] {
    const packages: PackageDecl[] = [];
    const visit = (pkg: PackageDecl) => {
        packages.push(pkg);
        for (const member of pkg.members) {
            if (isPackageDecl(member)) {
                visit(member);
            }
        }
    };
    roots.forEach(visit);
    return packages;
}

/**
 * What a view presents of the elements it exposes: an exposed package stands
 * for its members, nested packages' included, and anything else for itself.
 */
export function exposedMembers(exposed: Exposable[]): AstNode[] {
    return [
        ...exposed.filter(e => !isPackageDecl(e)),
        ...collectPackages(exposed.filter(isPackageDecl)).flatMap(pkg => pkg.members)
    ];
}

/**
 * The qualified names an import's path may stand for, nearest first: within
 * the importing package, within each package around it, then from the top.
 */
export function importCandidates(importer: PackageDecl, path: string): string[] {
    const candidates: string[] = [];
    let scope: PackageDecl | undefined = importer;
    while (scope) {
        candidates.push(`${qualifiedName(scope)}::${path}`);
        scope = isPackageDecl(scope.$container) ? scope.$container : undefined;
    }
    candidates.push(path);
    return candidates;
}
