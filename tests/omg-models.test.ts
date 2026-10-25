import { existsSync, readdirSync } from 'node:fs';
import { join, resolve } from 'node:path';
import { URI } from 'langium';
import { describe, expect, it } from 'vitest';
import { createSysmlServices } from '../src/parser/sysml-module.js';

// OMG's own SysML v2 models, fetched by `npm run omg-models`. ortho should load
// every one of them; this counts how many it does, so the count only moves on
// purpose. Without the models, the test is skipped.
const ROOT = '.omg-models/sysml/src';
const LOADED = 242;

function sysmlFiles(dir: string): string[] {
    return readdirSync(dir, { withFileTypes: true }).flatMap(entry => {
        const path = join(dir, entry.name);
        return entry.isDirectory() ? sysmlFiles(path) : path.endsWith('.sysml') ? [path] : [];
    });
}

describe.skipIf(!existsSync(ROOT))("OMG's models", () => {
    it('load, as many as counted', async () => {
        // One model, as OMG's pilot implementation holds them: the training
        // models refer to one another across folders.
        const services = createSysmlServices();
        const files = ['examples', 'training', 'validation'].flatMap(folder => sysmlFiles(join(ROOT, folder)));
        const documents = await Promise.all(files.map(file =>
            services.shared.workspace.LangiumDocuments.getOrCreateDocument(URI.file(resolve(file)))));
        await services.shared.workspace.DocumentBuilder.build(documents, { validation: true });
        const loaded = documents.filter(d => !(d.diagnostics ?? []).some(e => e.severity === 1)).length;
        expect(loaded, `${loaded} of ${files.length} load: update LOADED`).toBe(LOADED);
    }, 120_000);
});
