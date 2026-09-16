// Builds the marketing deck from marketing/deck.template.html by inlining the
// real generated diagrams, syntax-highlighting the SysML excerpts and
// numbering the sheets. Run with: npm run deck
import { readFile, writeFile } from 'node:fs/promises';

const TEMPLATE = 'marketing/deck.template.html';
const STANDALONE_OUT = 'marketing/ortho-deck.html';
const ARTIFACT_OUT = 'marketing/ortho-deck.artifact.html';

/** Short prefixes used in the template's data-svg / {{DIM:...}} references. */
const DIAGRAM_DIRS: Record<string, string> = {
    drone: 'examples/survey-drone/diagrams',
    coffee: 'examples/coffee-machine/diagrams'
};

const KEYWORDS = new Set([
    'package', 'import', 'part', 'def', 'port', 'interface', 'attribute', 'requirement',
    'satisfy', 'by', 'connect', 'to', 'allocate', 'use', 'case', 'actor', 'subject',
    'include', 'perform', 'action', 'message', 'from', 'then', 'of', 'end', 'specializes'
]);

function escapeHtml(text: string): string {
    return text.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;');
}

/** Single-pass tokenizer so highlighting never re-matches inside emitted markup. */
function highlightSysml(code: string): string {
    const pattern = /(\/\/[^\n]*)|("[^"]*")|(\b\d+(?:\.\d+)?\b)|([A-Za-z_][A-Za-z0-9_]*)|(::|:>)/g;
    let out = '';
    let last = 0;
    for (const match of code.matchAll(pattern)) {
        const [text, comment, string, number, word, operator] = match;
        out += escapeHtml(code.slice(last, match.index));
        last = match.index + text.length;
        if (comment) {
            out += `<span class="t-comment">${escapeHtml(text)}</span>`;
        } else if (string) {
            out += `<span class="t-string">${escapeHtml(text)}</span>`;
        } else if (number) {
            out += `<span class="t-number">${text}</span>`;
        } else if (operator) {
            out += `<span class="t-op">${escapeHtml(text)}</span>`;
        } else if (word && KEYWORDS.has(word)) {
            out += `<span class="t-keyword">${word}</span>`;
        } else if (word && /^[A-Z]/.test(word)) {
            out += `<span class="t-type">${word}</span>`;
        } else {
            out += escapeHtml(text);
        }
    }
    return out + escapeHtml(code.slice(last));
}

/** "drone/logical-view.svg" -> repo-relative path. */
function resolveRef(ref: string): { path: string; key: string } {
    const [prefix, file] = ref.split('/');
    const dir = DIAGRAM_DIRS[prefix];
    if (!dir || !file) {
        throw new Error(`Unknown diagram reference "${ref}" (expected one of ${Object.keys(DIAGRAM_DIRS).join(', ')}/<file>.svg)`);
    }
    return { path: `${dir}/${file}`, key: `${prefix}-${file.replace('.svg', '')}` };
}

/**
 * Inlines an SVG: namespaces its marker ids (every file declares the same
 * ones, and duplicate ids in one document collide) and drops the fixed
 * width/height so CSS can scale it against the viewBox.
 */
function prepareSvg(svg: string, key: string): string {
    const body = svg
        .replace(/<\?xml[^>]*\?>/g, '')
        .replace(/(<svg[^>]*?)\swidth="[^"]*"/, '$1')
        .replace(/(<svg[^>]*?)\sheight="[^"]*"/, '$1');
    const ids = [...body.matchAll(/\sid="([^"]+)"/g)].map(m => m[1]);
    return ids.reduce(
        (acc, id) => acc
            .replace(new RegExp(`\\sid="${id}"`, 'g'), ` id="${key}-${id}"`)
            .replace(new RegExp(`url\\(#${id}\\)`, 'g'), `url(#${key}-${id})`),
        body
    );
}

function viewBoxSize(svg: string): string {
    const match = svg.match(/viewBox="0 0 ([\d.]+) ([\d.]+)"/);
    return match ? `${Math.round(Number(match[1]))} × ${Math.round(Number(match[2]))} px` : 'unknown';
}

let html = await readFile(TEMPLATE, 'utf-8');

// Inline every diagram the template asks for, so adding a figure needs no
// change here.
for (const ref of new Set([...html.matchAll(/data-svg="([^"]+)"/g)].map(m => m[1]))) {
    const { path, key } = resolveRef(ref);
    const raw = await readFile(path, 'utf-8');
    html = html
        .replaceAll(`{{DIM:${ref}}}`, viewBoxSize(raw))
        .replace(
            new RegExp(`<div class="figure-svg" data-svg="${ref.replace('/', '\\/')}"></div>`),
            `<div class="figure-svg">${prepareSvg(raw, key)}</div>`
        );
}

html = html.replace(
    /<pre class="code"([^>]*)>([\s\S]*?)<\/pre>/g,
    (_match, attrs: string, code: string) => `<pre class="code"${attrs}>${highlightSysml(code.replace(/^\n/, '').trimEnd())}</pre>`
);

// Number the sheets from their order in the document.
const total = (html.match(/<section class="sheet/g) ?? []).length;
let sheet = 0;
html = html.replace(
    /<span class="sheet-no"><\/span>/g,
    () => `<span class="sheet-no">Sheet ${String(++sheet).padStart(2, '0')} / ${String(total).padStart(2, '0')}</span>`
);
if (sheet !== total) {
    throw new Error(`Found ${total} sheets but ${sheet} sheet-number slots — every sheet needs one.`);
}

const unresolved = html.match(/\{\{[^}]+\}\}|data-svg="/g);
if (unresolved) {
    throw new Error(`Deck template has unresolved placeholders: ${[...new Set(unresolved)].join(', ')}`);
}

await writeFile(ARTIFACT_OUT, html, 'utf-8');
await writeFile(
    STANDALONE_OUT,
    `<!doctype html>\n<html lang="en">\n<head>\n<meta charset="utf-8" />\n<meta name="viewport" content="width=device-width, initial-scale=1" />\n<title>ortho — orthographic projection for architecture</title>\n</head>\n<body>\n${html}\n</body>\n</html>\n`,
    'utf-8'
);

console.log(`Wrote ${STANDALONE_OUT} and ${ARTIFACT_OUT} (${total} sheets)`);
