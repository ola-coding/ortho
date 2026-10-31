import { resolve } from 'node:path';
import { URI } from 'langium';
import type { LangiumDocument } from 'langium';
import type { SysmlServices } from './sysml-module.js';
import type { Model } from '../generated/ast.js';

export interface ParsedModel {
    /** Synthetic root merging the packages of every parsed file. */
    model: Model;
    /** One document per input file, in input order. */
    documents: Array<LangiumDocument<Model>>;
    /** What the validator warns of, `file line n: message`. None of it stops a model rendering. */
    warnings: string[];
}

/**
 * Parses and links one or more .sysml files as a single workspace, so
 * names resolve across files. The returned model merges all root packages in
 * file order.
 */
export async function parseSysmlFiles(services: SysmlServices, filePaths: string[]): Promise<ParsedModel> {
    const documents: Array<LangiumDocument<Model>> = [];
    for (const filePath of filePaths) {
        const uri = URI.file(resolve(filePath));
        documents.push(await services.shared.workspace.LangiumDocuments.getOrCreateDocument(uri) as LangiumDocument<Model>);
    }
    await services.shared.workspace.DocumentBuilder.build(documents, { validation: true });

    // LSP's severities: 1 is an error, 2 a warning.
    const reported = (severity: 1 | 2): string[] => documents.flatMap((document, i) =>
        (document.diagnostics ?? []).filter(d => d.severity === severity).map(d =>
            // LSP 3.18 lets a message be markup; ortho's own are plain strings.
            `${filePaths[i]} line ${d.range.start.line + 1}: ${typeof d.message === 'string' ? d.message : d.message.value}`));
    const failures = reported(1);
    if (failures.length > 0) {
        throw new Error(`Failed to parse:\n${failures.map(failure => `  ${failure}`).join('\n')}`);
    }

    const model = {
        $type: 'Model',
        packages: documents.flatMap(d => d.parseResult.value.packages)
    } as unknown as Model;
    return { model, documents, warnings: reported(2) };
}
