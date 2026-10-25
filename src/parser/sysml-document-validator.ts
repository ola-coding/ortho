import { DefaultDocumentValidator } from 'langium';
import type { AstNode } from 'langium';
import { isConnectorEnd, isImportDecl } from '../generated/ast.js';
import type { ImportDecl } from '../generated/ast.js';
import { LIBRARY } from '../model/library.js';

type LinkingErrorArgs = Parameters<DefaultDocumentValidator['processLinkingErrors']>;
type Reference = LinkingErrorArgs[0]['references'][number];

/**
 * ortho does not load the standard library, so some references name nothing
 * it has, and are not errors:
 *
 * - an import of a library package, such as `ScalarValues`, which imports
 *   nothing: a name it would have supplied fails where it is used instead;
 * - a name of the library itself, `SpatialItems::SpatialItem`, or one an
 *   import of the library brings in, which a view shows as written.
 *
 * A typo is still an error: the library has to have the name. And a chain's
 * later segments, after one that failed, are not reported again.
 */
export class SysmlDocumentValidator extends DefaultDocumentValidator {
    protected override processLinkingErrors(...[document, diagnostics, options]: LinkingErrorArgs): void {
        const tolerated = new Set(document.references.filter(mayNameNothing).map(reference => reference.$refNode?.range));
        const start = diagnostics.length;
        super.processLinkingErrors(document, diagnostics, options);
        diagnostics.push(...diagnostics.splice(start).filter(d => !tolerated.has(d.range)));
    }
}

function mayNameNothing(reference: Reference): boolean {
    if (!reference.error) {
        return false;
    }
    const { container, index } = reference.error.info;
    if (isImportDecl(container)) {
        return true;
    }
    if (isConnectorEnd(container) && (index ?? 0) > 0 && !container.segments[(index ?? 0) - 1]?.ref) {
        return true;
    }
    return namesLibraryElement(container, reference.$refText);
}

function namesLibraryElement(container: AstNode, text: string): boolean {
    const segments = text.split('::');
    const name = segments.pop()!;
    if (segments.length > 0) {
        return libraryNames(segments.join('::')).has(name);
    }
    for (let namespace: AstNode | undefined = container; namespace; namespace = namespace.$container) {
        for (const imp of (namespace as { imports?: ImportDecl[] }).imports ?? []) {
            if (imp.target?.ref) {
                continue;
            }
            const path = imp.target.$refText;
            const brought = imp.wildcard || imp.recursive
                ? libraryNames(path, imp.recursive).has(name)
                : path.split('::').at(-1) === name && libraryNames(path.split('::').slice(0, -1).join('::')).has(name);
            if (brought) {
                return true;
            }
        }
    }
    return false;
}

const namesByPackage = new Map<string, Set<string>>();

/** The names a library package shows: its own and its public imports'; with `nested`, its packages' too. */
function libraryNames(pkg: string, nested = false): Set<string> {
    const key = `${pkg}${nested ? '::**' : ''}`;
    let names = namesByPackage.get(key);
    if (!names) {
        const found = new Set<string>();
        const seen = new Set<string>();
        const visit = (name: string) => {
            const entry = LIBRARY[name];
            if (!entry || seen.has(name)) {
                return;
            }
            seen.add(name);
            entry.names.forEach(n => found.add(n));
            entry.imports.forEach(visit);
        };
        visit(pkg);
        if (nested) {
            Object.keys(LIBRARY).filter(name => name.startsWith(`${pkg}::`)).forEach(visit);
        }
        names = found;
        namesByPackage.set(key, names);
    }
    return names;
}
