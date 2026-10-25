import { DefaultDocumentValidator } from 'langium';
import type { AstNode } from 'langium';
import { isImportDecl } from '../generated/ast.js';

type LinkingErrorArgs = Parameters<DefaultDocumentValidator['processLinkingErrors']>;

/**
 * An import of a package ortho does not have, such as the standard library's
 * `ScalarValues`, imports nothing rather than failing: a name it would have
 * supplied fails where it is used instead.
 */
export class SysmlDocumentValidator extends DefaultDocumentValidator {
    protected override processLinkingErrors(...[document, diagnostics, options]: LinkingErrorArgs): void {
        const tolerated = new Set(document.references
            .filter(reference => reference.error && mayNameNothing(reference.error.info.container))
            .map(reference => reference.$refNode?.range));
        const start = diagnostics.length;
        super.processLinkingErrors(document, diagnostics, options);
        diagnostics.push(...diagnostics.splice(start).filter(d => !tolerated.has(d.range)));
    }
}

function mayNameNothing(container: AstNode): boolean {
    return isImportDecl(container);
}
