import type { AstNode } from 'langium';
import { isPackageDecl } from '../generated/ast.js';
import type { Member, PackageDecl } from '../generated/ast.js';

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
export function exposedMembers(exposed: Member[]): AstNode[] {
    return [
        ...exposed.filter(e => !isPackageDecl(e)),
        ...collectPackages(exposed.filter(isPackageDecl)).flatMap(pkg => pkg.members)
    ];
}
