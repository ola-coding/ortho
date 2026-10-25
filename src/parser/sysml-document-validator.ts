import { DefaultDocumentValidator } from 'langium';

type LinkingErrorArgs = Parameters<DefaultDocumentValidator['processLinkingErrors']>;

/**
 * An import of a package ortho does not have, such as the standard library's
 * `ScalarValues`, imports nothing rather than failing: a name it would have
 * supplied fails where it is used instead.
 */
export class SysmlDocumentValidator extends DefaultDocumentValidator {
    protected override processLinkingErrors(...[document, diagnostics, options]: LinkingErrorArgs): void {
        const start = diagnostics.length;
        super.processLinkingErrors(document, diagnostics, options);
        const kept = diagnostics.splice(start)
            .filter(d => (d.data as { containerType?: string } | undefined)?.containerType !== 'ImportDecl');
        diagnostics.push(...kept);
    }
}
