import { readFile } from 'node:fs/promises';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, expect, it } from 'vitest';
import { clearDocuments, parseHelper } from 'langium/test';
import { createSysmlServices } from '../src/parser/sysml-module.js';
import type { Model } from '../src/generated/ast.js';

// Every model on LANGUAGE.md has to load, so the page never shows one that
// would not.

const root = resolve(dirname(fileURLToPath(import.meta.url)), '..');

function sysmlBlocks(doc: string): string[] {
    return [...doc.matchAll(/```sysml\r?\n([\s\S]*?)```/g)].map(m => m[1]);
}

describe('LANGUAGE.md', () => {
    it('holds only examples that parse, each on its own', async () => {
        const blocks = sysmlBlocks(await readFile(join(root, 'LANGUAGE.md'), 'utf-8'));
        expect(blocks.length).toBeGreaterThan(0);
        const services = createSysmlServices().Sysml;
        const parse = parseHelper<Model>(services);
        for (const block of blocks) {
            // An empty workspace per block, so no block leans on another's names.
            await clearDocuments(services);
            const document = await parse(block, { validation: true });
            const errors = (document.diagnostics ?? []).filter(d => d.severity === 1).map(d => d.message);
            expect(errors, block.split('\n')[0]).toEqual([]);
        }
    });
});
