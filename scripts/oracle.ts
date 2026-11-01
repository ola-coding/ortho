// Records what this TypeScript version produces at each stage, for the Rust port
// to be checked against: every model's tree, references and diagnostics, and each
// view's graph, ELK input and output, layout and SVG. Written to rust/oracle/, not
// committed, and gone with the TypeScript once the port replaces it.
// Run with: npm run oracle, after npm run omg-models.
import { existsSync, mkdirSync, readFileSync, readdirSync, rmSync, statSync, writeFileSync } from 'node:fs';
import { join, relative, resolve } from 'node:path';
import { AstUtils, URI } from 'langium';
import type { LangiumDocument, Reference } from 'langium';
import type { Model, ViewUsage } from '../src/generated/ast.js';
import { extractDeploymentGraph } from '../src/diagrams/deployment.js';
import { extractImplementationGraph } from '../src/diagrams/implementation.js';
import { extractLogicalGraph } from '../src/diagrams/logical.js';
import { extractPhysicalGraph } from '../src/diagrams/physical.js';
import { extractSequenceModel } from '../src/diagrams/sequence.js';
import { extractUseCaseGraph } from '../src/diagrams/use-case.js';
import { elk, layoutGraph } from '../src/layout/elk-layout.js';
import { layoutTree } from '../src/layout/tree-layout.js';
import { layoutUseCases } from '../src/layout/use-case-layout.js';
import { declaredViews, diagramKind, exposedElements, viewFileName, whyUndrawable } from '../src/model/views.js';
import { createSysmlServices } from '../src/parser/sysml-module.js';
import { renderView } from '../src/pipeline.js';
import { renderSequenceSvg } from '../src/render/sequence-renderer.js';
import { renderSvg } from '../src/render/svg-renderer.js';
import type { DiagramTitle } from '../src/render/svg-renderer.js';
import { darkGlassTheme, lightTheme } from '../src/render/theme.js';
import type { Theme } from '../src/render/theme.js';

const OUT = 'rust/oracle';
const OMG = '.omg-models/sysml/src';
const CONSTRUCTS = 'rust/tests/constructs.txt';

function sysmlFiles(path: string): string[] {
    if (statSync(path).isFile()) {
        return [path];
    }
    return readdirSync(path, { withFileTypes: true })
        .sort((a, b) => (a.name < b.name ? -1 : 1))
        .flatMap(entry => {
            const full = join(path, entry.name);
            return entry.isDirectory() ? sysmlFiles(full) : full.endsWith('.sysml') ? [full] : [];
        })
        .map(file => file.replace(/\\/g, '/'));
}

function writeLines(file: string, lines: string[]): void {
    mkdirSync(join(file, '..'), { recursive: true });
    writeFileSync(file, lines.map(line => `${line}\n`).join(''));
}

// ---------------------------------------------------------------- Models that test a parser

/**
 * Each construct on its own, in three places: at the top of a file, in a package
 * and in an action definition. A line that does not parse hides no other.
 */
function writeConstructs(): string[] {
    const dir = `${OUT}/models/constructs`;
    mkdirSync(dir, { recursive: true });
    const lines = readFileSync(CONSTRUCTS, 'utf-8').split('\n').filter(Boolean);
    const places = [(t: string) => `${t}\n`, (t: string) => `package P {\n${t}\n}\n`, (t: string) => `action def A {\n${t}\n}\n`];
    return lines.flatMap((line, i) => places.map((place, j) => {
        const file = `${dir}/${String(i * places.length + j).padStart(5, '0')}.sysml`;
        writeFileSync(file, place(line));
        return file;
    }));
}

const WORDS = ['part', 'def', 'port', 'action', 'attribute', 'use', 'case', 'item', 'if', 'then', 'else', 'first', 'end',
    'in', 'out', 'ref', 'connect', 'to', 'from', 'interface', 'bind', 'flow', 'message', 'of', 'accept', 'send', 'via',
    'do', 'state', 'entry', 'transition', 'perform', 'allocate', 'public', 'private', 'import', 'alias', 'for', 'package',
    'standard', 'library', 'metadata', 'comment', 'doc', 'about', 'succession', 'not', 'all', 'new', 'as', 'assert',
    'satisfy', 'by', 'loop', 'while', 'until', 'event', 'occurrence', 'view', 'expose', 'filter', 'return', 'variant',
    'abstract', ':', ':>', ':>>', '::>', '::', '::*', '::**', '.', '.?', ';', '{', '}', '(', ')', '[', ']', '<', '>', '=',
    ':=', '=>', '@', '#', ',', '~', '*', '..', '->', '+', '-', '?', '??', '"s"', "'q'", '1', '.5', 'x'];
const TOKEN = /\/\*[\s\S]*?\*\/|\/\/[^\n]*|"(?:\\.|[^"\\\n])*"|'(?:\\.|[^'\\\n])*'|[A-Za-z_]\w*|\d+(?:\.\d+)?|::\*\*|::\*|::>|:>>|!==|===|\.\.|\.\?|::|:=|:>|->|=>|==|!=|<=|>=|\*\*|\?\?|@@|\S/g;

/** Copies of `files`, each with one token deleted, doubled, swapped with the next, or replaced. */
function writeBroken(files: string[], perFile: number, seed: number, dir: string, first: number): string[] {
    let state = seed;
    const random = (): number => {
        state = (state + 0x6D2B79F5) | 0;
        let t = Math.imul(state ^ (state >>> 15), 1 | state);
        t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t;
        return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
    };
    const below = (n: number): number => Math.floor(random() * n);
    const written: string[] = [];
    for (const file of files) {
        const text = readFileSync(file, 'utf-8');
        const tokens = [...text.matchAll(TOKEN)].filter(m => !m[0].startsWith('//') && !m[0].startsWith('/*'));
        if (tokens.length < 3) {
            continue;
        }
        for (let k = 0; k < perFile; k++) {
            const token = tokens[below(tokens.length - 1)];
            const next = tokens[tokens.indexOf(token) + 1];
            const start = token.index;
            const end = start + token[0].length;
            const edits: Array<() => string> = [
                () => text.slice(0, start) + text.slice(end),
                () => `${text.slice(0, end)} ${token[0]}${text.slice(end)}`,
                () => text.slice(0, start) + next[0] + text.slice(end, next.index) + token[0] + text.slice(next.index + next[0].length),
                () => text.slice(0, start) + WORDS[below(WORDS.length)] + text.slice(end),
                () => text.slice(0, start) + tokens[below(tokens.length)][0] + text.slice(end)
            ];
            const copy = `${dir}/${String(first + written.length).padStart(6, '0')}.sysml`;
            writeFileSync(copy, edits[below(edits.length)]());
            written.push(copy);
        }
    }
    return written;
}

// ---------------------------------------------------------------- Trees

/** A tree as JSON, keys sorted, without what is empty or false, as the Rust parser writes it. */
function canonical(value: unknown): unknown {
    if (Array.isArray(value)) {
        return value.map(canonical);
    }
    if (value && typeof value === 'object') {
        const object = value as Record<string, unknown>;
        if ('$refText' in object) {
            return { $ref: object.$refText };
        }
        const out: Record<string, unknown> = {};
        for (const key of Object.keys(object).sort()) {
            const item = object[key];
            if ((key.startsWith('$') && key !== '$type') || item === false || item === undefined
                || (Array.isArray(item) && item.length === 0)) {
                continue;
            }
            out[key] = canonical(item);
        }
        return out;
    }
    return value;
}

function dumpTrees(name: string, files: string[]): void {
    const parser = createSysmlServices().Sysml.parser.LangiumParser;
    writeLines(`${OUT}/trees/${name}.tsv`, files.map(file => {
        const result = parser.parse(readFileSync(file, 'utf-8'));
        const rejected = result.lexerErrors.length + result.parserErrors.length > 0;
        return `${file}\t${rejected ? '{"$rejected":true}' : JSON.stringify(canonical(result.value))}`;
    }));
    console.log(`trees: ${name}, ${files.length} files`);
}

// ---------------------------------------------------------------- References and diagnostics

type Documents = Array<LangiumDocument<Model>>;

async function load(files: string[]): Promise<{ documents: Documents; services: ReturnType<typeof createSysmlServices>['Sysml'] }> {
    const { shared, Sysml } = createSysmlServices();
    const documents = await Promise.all(files.map(file =>
        shared.workspace.LangiumDocuments.getOrCreateDocument(URI.file(resolve(file))))) as Documents;
    await shared.workspace.DocumentBuilder.build(documents, { validation: true });
    return { documents, services: Sysml };
}

const repoPath = (document: LangiumDocument): string => relative(process.cwd(), document.uri.fsPath).replace(/\\/g, '/');

function dumpLinks(name: string, documents: Documents, services: Awaited<ReturnType<typeof load>>['services']): number {
    const locator = services.workspace.AstNodeLocator;
    const links: string[] = [];
    const diagnostics: string[] = [];
    let errors = 0;
    for (const document of documents) {
        const file = repoPath(document);
        for (const node of AstUtils.streamAst(document.parseResult.value)) {
            for (const { reference, property, index } of AstUtils.streamReferences(node)) {
                const target = (reference as Reference).ref;
                links.push(JSON.stringify({
                    file,
                    node: locator.getAstNodePath(node),
                    property,
                    index,
                    text: reference.$refText,
                    target: target ? `${repoPath(AstUtils.getDocument(target))}#${locator.getAstNodePath(target)}` : null
                }));
            }
        }
        for (const diagnostic of document.diagnostics ?? []) {
            errors += diagnostic.severity === 1 ? 1 : 0;
            diagnostics.push(JSON.stringify({
                file,
                severity: diagnostic.severity,
                line: diagnostic.range.start.line + 1,
                character: diagnostic.range.start.character + 1,
                message: diagnostic.message
            }));
        }
    }
    writeLines(`${OUT}/links/${name}.jsonl`, links);
    writeLines(`${OUT}/diagnostics/${name}.jsonl`, diagnostics);
    console.log(`links: ${name}, ${documents.length} files, ${links.length} references, ${diagnostics.length} diagnostics`);
    return errors;
}

// ---------------------------------------------------------------- Views

// What ELK is given and gives back, for the view being recorded.
let elkCalls: Array<{ input: unknown; output: unknown }> | undefined;
const elkLayout = elk.layout.bind(elk) as (graph: object, args?: object) => Promise<object>;
(elk as unknown as { layout: typeof elkLayout }).layout = async (graph, args) => {
    const input = JSON.stringify(graph);
    const result = await elkLayout(graph, args);
    elkCalls?.push({ input: JSON.parse(input), output: JSON.parse(JSON.stringify(result)) });
    return result;
};

const graphOf = {
    'use-case': { extract: extractUseCaseGraph, layout: layoutUseCases },
    'logical': { extract: extractLogicalGraph, layout: layoutTree },
    'implementation': { extract: extractImplementationGraph, layout: (graph: Parameters<typeof layoutGraph>[0]) => layoutGraph(graph, { gridLeaves: true }) },
    'physical': { extract: extractPhysicalGraph, layout: layoutGraph },
    'deployment': { extract: extractDeploymentGraph, layout: (graph: Parameters<typeof layoutGraph>[0]) => layoutGraph(graph, { algorithm: 'rectpacking' }) }
};

/** One view, stage by stage. Its SVG must be what the pipeline itself renders. */
async function dumpView(view: ViewUsage, model: Model, title: DiagramTitle, stem: string): Promise<void> {
    const write = (stage: string, content: string): void => {
        mkdirSync(join(stem, '..'), { recursive: true });
        writeFileSync(`${stem}.${stage}`, content);
    };
    const kind = diagramKind(view);
    const exposed = exposedElements(view);
    let svg: (theme: Theme) => string;
    if (kind === 'process') {
        const sequence = extractSequenceModel(exposed);
        write('graph.json', JSON.stringify(sequence));
        svg = theme => renderSequenceSvg(sequence, title, theme);
    } else {
        const graph = graphOf[kind].extract(exposed, model);
        write('graph.json', JSON.stringify(graph));
        elkCalls = [];
        const laidOut = await graphOf[kind].layout(graph);
        if (elkCalls.length > 0) {
            write('elk.json', JSON.stringify(elkCalls));
        }
        elkCalls = undefined;
        write('layout.json', JSON.stringify(laidOut));
        svg = theme => renderSvg(laidOut, title, theme);
    }
    const light = svg(lightTheme);
    if (light !== (await renderView(view, model, title, lightTheme)).svg) {
        throw new Error(`${stem}: this script and the pipeline render differently`);
    }
    write('svg', light);
    write('dark.svg', svg(darkGlassTheme));
}

async function dumpViews(name: string, documents: Documents): Promise<void> {
    const model = { $type: 'Model', packages: documents.flatMap(d => d.parseResult.value.packages) } as unknown as Model;
    const lines: string[] = [];
    const taken = new Set<string>();
    for (const document of documents) {
        const source = repoPath(document);
        for (const view of declaredViews(document.parseResult.value)) {
            const base = `${name}/${viewFileName(view).replace(/\.svg$/, '')}`;
            let id = base;
            for (let n = 2; taken.has(id); n++) {
                id = `${base}-${n}`;
            }
            taken.add(id);
            const line: Record<string, unknown> = { id, source, name: view.name, definition: view.definition };
            const skipped = whyUndrawable(view);
            if (skipped) {
                line.skipped = skipped;
            } else {
                line.kind = diagramKind(view);
                try {
                    await dumpView(view, model, { heading: view.name ?? '', definition: view.definition, source }, `${OUT}/views/${id}`);
                } catch (error) {
                    line.error = error instanceof Error ? error.message : String(error);
                }
            }
            lines.push(JSON.stringify(line));
        }
    }
    const rendered = lines.filter(line => !line.includes('"skipped"') && !line.includes('"error"')).length;
    mkdirSync(`${OUT}/views`, { recursive: true });
    writeFileSync(`${OUT}/views/index.jsonl`, lines.map(line => `${line}\n`).join(''), { flag: 'a' });
    console.log(`views: ${name}, ${lines.length} declared, ${rendered} rendered`);
}

/** The committed diagrams are what the examples must render to. */
function checkCommitted(example: string): void {
    const diagrams = `examples/${example}/diagrams`;
    for (const file of readdirSync(diagrams)) {
        const recorded = readFileSync(`${OUT}/views/${example}/${file.replace(/\.svg$/, '')}.svg`, 'utf-8');
        if (recorded !== readFileSync(`${diagrams}/${file}`, 'utf-8')) {
            throw new Error(`${diagrams}/${file} is not what the example renders to; run npm run render:examples`);
        }
    }
}

// ---------------------------------------------------------------- All of it

if (!existsSync(OMG)) {
    throw new Error(`${OMG} is missing; run npm run omg-models first.`);
}
rmSync(OUT, { recursive: true, force: true });

const omg = ['examples', 'training', 'validation'].flatMap(folder => sysmlFiles(join(OMG, folder)));
const examples = ['coffee-machine', 'survey-drone'];
const fixtures = sysmlFiles('tests/fixtures');
const ours = [...examples.flatMap(example => sysmlFiles(`examples/${example}`)), ...fixtures];

dumpTrees('omg', omg);
dumpTrees('ours', ours);
dumpTrees('constructs', writeConstructs());
const brokenDir = `${OUT}/models/broken`;
mkdirSync(brokenDir, { recursive: true });
const broken = writeBroken([...omg, ...ours], 12, 42, brokenDir, 0);
broken.push(...writeBroken([...omg, ...ours], 30, 2026, brokenDir, broken.length));
dumpTrees('broken', broken);

const workspaces: Array<[string, string[]]> = [
    ['omg', omg],
    ...examples.map((example): [string, string[]] => [example, sysmlFiles(`examples/${example}`)]),
    ...fixtures.map((file): [string, string[]] => [`fixture-${file.replace(/^.*\/|\.sysml$/g, '')}`, [file]])
];
for (const [name, files] of workspaces) {
    const { documents, services } = await load(files);
    if (dumpLinks(name, documents, services) === 0) {
        await dumpViews(name, documents);
    }
}
examples.forEach(checkCommitted);
console.log(`Wrote ${OUT}/`);
