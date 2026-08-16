import { isPackageDecl } from '../generated/ast.js';
import type { Model, PackageDecl } from '../generated/ast.js';

export function collectPackages(model: Model): PackageDecl[] {
    const packages: PackageDecl[] = [];
    const visit = (pkg: PackageDecl) => {
        packages.push(pkg);
        for (const member of pkg.members) {
            if (isPackageDecl(member)) {
                visit(member);
            }
        }
    };
    model.packages.forEach(visit);
    return packages;
}
