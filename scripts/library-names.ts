// Lists the names OMG's standard library declares, package by package, into
// src/model/library.ts, so that a view may name a library element without
// ortho loading the library. Run with: npm run library-names, after
// npm run omg-models has fetched the library.
//
// Only names are kept: a declaration is read as far as its name, and every
// body that is not a package's is skipped.
import { readdirSync, readFileSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { RESERVED_WORDS } from '../src/parser/sysml-validator.js';

const LIBRARY = '.omg-models/sysml.library';
const OUT = 'src/model/library.ts';

// KerML's reserved words that SysML's do not include (KerML §8.2.2.6).
const KERML_WORDS = [
    'assoc', 'behavior', 'bool', 'chains', 'class', 'classifier', 'composite', 'conjugate', 'conjugates',
    'conjugation', 'connector', 'const', 'datatype', 'differences', 'disjoining', 'disjoint', 'expr', 'feature',
    'featured', 'featuring', 'function', 'interaction', 'intersects', 'inv', 'inverse', 'inverting', 'member',
    'metaclass', 'multiplicity', 'namespace', 'portion', 'predicate', 'redefinition', 'specialization', 'step',
    'struct', 'subclassifier', 'subset', 'subtype', 'type', 'typed', 'typing', 'unions', 'var'
];
const KEYWORDS = new Set([...RESERVED_WORDS, ...KERML_WORDS]);
const SKIPPED = new Set(['doc', 'comment', 'rep', 'language', 'filter', 'dependency', 'expose', 'render']);

interface Package {
    names: Set<string>;
    imports: Set<string>;
}

const packages = new Map<string, Package>();

function files(dir: string): string[] {
    return readdirSync(dir, { withFileTypes: true }).flatMap(entry => {
        const path = join(dir, entry.name);
        return entry.isDirectory() ? files(path) : /\.(sysml|kerml)$/.test(entry.name) ? [path] : [];
    });
}

function tokensOf(text: string): string[] {
    const code = text
        .replace(/\/\/\*[\s\S]*?\*\//g, ' ')
        // A comment ends the `doc` or `comment` element it is the body of.
        .replace(/\/\*[\s\S]*?\*\//g, ' ; ')
        .replace(/\/\/[^\n]*/g, ' ')
        .replace(/"(?:\\.|[^"\\])*"/g, ' "" ');
    return code.match(/'(?:\\.|[^'\\])*'|[A-Za-z_]\w*|::\*\*|::\*|::|[^\s\w]/g) ?? [];
}

const unquote = (token: string) => token.startsWith("'") ? token.slice(1, -1).replace(/\\(.)/g, '$1') : token;
const isName = (token: string | undefined): token is string =>
    !!token && (token.startsWith("'") || (/^[A-Za-z_]\w*$/.test(token) && !KEYWORDS.has(token)));

function packageNamed(name: string): Package {
    let pkg = packages.get(name);
    if (!pkg) {
        pkg = { names: new Set(), imports: new Set() };
        packages.set(name, pkg);
    }
    return pkg;
}

/** The names a declaration gives, its short name included: `<kg> kilogram`. */
function declared(words: string[]): string[] {
    const names: string[] = [];
    for (let k = 0; k < words.length; k++) {
        const word = words[k];
        if (word === '#' || word === '@') {
            // A metadata keyword, `#Safety`, or annotation names nothing here.
            k++;
            while (words[k + 1] === '::') {
                k += 2;
            }
        } else if (word === '<' && isName(words[k + 1]) && words[k + 2] === '>') {
            names.push(unquote(words[k + 1]));
            k += 2;
        } else if (isName(word)) {
            names.push(unquote(word));
            break;
        } else if ([':', '=', '[', '(', '~', '.'].includes(word)) {
            break;
        }
    }
    return names;
}

function scan(tokens: string[]): void {
    // The package each open brace belongs to, or undefined for any other body.
    const stack: Array<string | undefined> = [];
    let i = 0;
    while (i < tokens.length) {
        if (tokens[i] === '}') {
            stack.pop();
            i++;
            continue;
        }
        let j = i;
        while (j < tokens.length && !['{', '}', ';'].includes(tokens[j])) {
            j++;
        }
        const words = tokens.slice(i, j);
        const owner = stack.length === 0 ? '' : stack[stack.length - 1];
        let opened: string | undefined;
        if (owner !== undefined && words.length > 0 && !SKIPPED.has(words[0])) {
            const at = words.indexOf('package');
            if (at >= 0) {
                const [name] = declared(words.slice(at + 1));
                if (name) {
                    opened = owner ? `${owner}::${name}` : name;
                    packageNamed(opened);
                    if (owner) {
                        packageNamed(owner).names.add(name);
                    }
                }
            } else if (owner && words.includes('import')) {
                const path = words.slice(words.indexOf('import') + 1).filter(w => w !== 'all');
                if (words.includes('public')) {
                    const last = path[path.length - 1];
                    if (last === '::*' || last === '::**') {
                        packageNamed(owner).imports.add(path.slice(0, -1).filter(w => w !== '::').map(unquote).join('::'));
                    } else if (isName(last)) {
                        packageNamed(owner).names.add(unquote(last));
                    }
                }
            } else if (owner && words.includes('alias')) {
                declared(words.slice(words.indexOf('alias') + 1)).forEach(n => packageNamed(owner).names.add(n));
            } else if (owner) {
                declared(words).forEach(n => packageNamed(owner).names.add(n));
            }
        }
        if (tokens[j] === '{') {
            stack.push(opened);
        }
        i = tokens[j] === '}' ? j : j + 1;
    }
}

for (const file of files(LIBRARY).sort()) {
    scan(tokensOf(readFileSync(file, 'utf8')));
}

const entries = [...packages].sort(([a], [b]) => a.localeCompare(b)).map(([name, pkg]) =>
    `    ${JSON.stringify(name)}: {\n        names: ${JSON.stringify([...pkg.names].sort())},\n`
    + `        imports: ${JSON.stringify([...pkg.imports].sort())}\n    }`);

writeFileSync(OUT, `// Generated by scripts/library-names.ts from OMG's standard library, release
// 2026-09; do not edit. Run npm run library-names to regenerate it.

/**
 * The names each package of the standard library declares, and the packages
 * it imports publicly, so that a view may name a library element, such as a
 * part typed \`SpatialItems::SpatialItem\`, without ortho loading the library.
 */
export const LIBRARY: Readonly<Record<string, { names: readonly string[]; imports: readonly string[] }>> = {
${entries.join(',\n')}
};
`);
console.log(`Wrote ${OUT}: ${packages.size} packages, ${[...packages.values()].reduce((n, p) => n + p.names.size, 0)} names`);
