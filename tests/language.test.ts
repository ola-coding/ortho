import { readFile, readdir } from 'node:fs/promises';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, expect, it } from 'vitest';
import { parseHelper } from 'langium/test';
import { createSysmlServices } from '../src/parser/sysml-module.js';
import type { Model } from '../src/generated/ast.js';

// LANGUAGE.md is the written subset. These tests keep it honest against the
// grammar it describes and the example models that use it.

const root = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const read = (path: string) => readFile(join(root, path), 'utf-8');

async function grammarKeywords(): Promise<Set<string>> {
    const grammar = await read('grammar/sysml.langium');
    return new Set([...grammar.matchAll(/'([a-z]+)'/g)].map(m => m[1]));
}

async function language(): Promise<string> {
    return read('LANGUAGE.md');
}

function indexKeywords(doc: string): Set<string> {
    const index = doc.slice(doc.indexOf('## Keyword index'));
    const section = index.slice(0, index.indexOf('\n## ', 1));
    return new Set([...section.matchAll(/^\| `([a-z]+)` \|/gm)].map(m => m[1]));
}

function sysmlBlocks(doc: string): string[] {
    return [...doc.matchAll(/```sysml\r?\n([\s\S]*?)```/g)].map(m => m[1]);
}

/** Words of the model text, with comments and strings taken out. */
function words(text: string): Set<string> {
    const code = text
        .replace(/\/\*[\s\S]*?\*\//g, ' ')
        .replace(/\/\/[^\n\r]*/g, ' ')
        .replace(/"[^"]*"/g, ' ');
    return new Set(code.match(/[A-Za-z_]\w*/g) ?? []);
}

async function exampleModels(): Promise<string[]> {
    const texts: string[] = [];
    for (const example of await readdir(join(root, 'examples'))) {
        for (const file of await readdir(join(root, 'examples', example))) {
            if (file.endsWith('.sysml')) {
                texts.push(await read(join('examples', example, file)));
            }
        }
    }
    return texts;
}

describe('LANGUAGE.md', () => {
    it('lists exactly the keywords the grammar knows', async () => {
        const inGrammar = await grammarKeywords();
        const inDoc = indexKeywords(await language());
        expect([...inDoc].sort()).toEqual([...inGrammar].sort());
    });

    it('holds only examples that parse, each on its own', async () => {
        const blocks = sysmlBlocks(await language());
        expect(blocks.length).toBeGreaterThan(0);
        for (const block of blocks) {
            // A fresh workspace per block, so no block leans on another's names.
            const parse = parseHelper<Model>(createSysmlServices().Sysml);
            const document = await parse(block, { validation: true });
            const errors = (document.diagnostics ?? []).filter(d => d.severity === 1).map(d => d.message);
            expect(errors, block.split('\n')[0]).toEqual([]);
        }
    });

    it('gives every keyword a worked example, in the example models or on the page', async () => {
        const used = new Set<string>();
        for (const text of [...await exampleModels(), ...sysmlBlocks(await language())]) {
            words(text).forEach(w => used.add(w));
        }
        const missing = [...await grammarKeywords()].filter(k => !used.has(k));
        expect(missing).toEqual([]);
    });
});
