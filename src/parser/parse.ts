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
}

/**
 * Parses and links one or more .sysml files as a single workspace, so
 * cross-file references resolve via qualified names. The returned model
 * merges all root packages in file order.
 */
export async function parseSysmlFiles(services: SysmlServices, filePaths: string[]): Promise<ParsedModel> {
    const documents: Array<LangiumDocument<Model>> = [];
    for (const filePath of filePaths) {
        const uri = URI.file(resolve(filePath));
        documents.push(await services.shared.workspace.LangiumDocuments.getOrCreateDocument(uri) as LangiumDocument<Model>);
    }
    await services.shared.workspace.DocumentBuilder.build(documents, { validation: true });

    const failures: string[] = [];
    documents.forEach((document, i) => {
        for (const error of (document.diagnostics ?? []).filter(d => d.severity === 1)) {
            failures.push(`  ${filePaths[i]} line ${error.range.start.line + 1}: ${error.message}`);
        }
    });
    if (failures.length > 0) {
        throw new Error(`Failed to parse:\n${failures.join('\n')}`);
    }

    const model = {
        $type: 'Model',
        packages: documents.flatMap(d => d.parseResult.value.packages)
    } as unknown as Model;
    return { model, documents };
}
