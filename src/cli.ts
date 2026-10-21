import { mkdir, readFile, writeFile, readdir, stat } from 'node:fs/promises';
import { dirname, join } from 'node:path';
import { Command } from 'commander';
import { createSysmlServices } from './parser/sysml-module.js';
import { parseSysmlFiles } from './parser/parse.js';
import type { DiagramTitle } from './render/svg-renderer.js';
import { renderView } from './pipeline.js';
import { declaredViews, viewHeading } from './model/views.js';
import { getTheme, themes } from './render/theme.js';

interface ExpandedInput {
    isDirectory: boolean;
    files: string[];
}

async function expandInput(path: string): Promise<ExpandedInput> {
    const info = await stat(path);
    if (!info.isDirectory()) {
        return { isDirectory: false, files: [path] };
    }
    const entries = await readdir(path);
    return {
        isDirectory: true,
        files: entries.filter(e => e.endsWith('.sysml')).sort().map(e => join(path, e))
    };
}

// Read from the package rather than hard-coding, so `--version` cannot drift.
// Resolves the same from src/ under tsx and from dist/ once built.
const pkg = JSON.parse(
    await readFile(new URL('../package.json', import.meta.url), 'utf-8')
) as { version: string };

const program = new Command();

program
    .name('ortho')
    .description('Render diagrams from SysML v2 model files')
    .version(pkg.version);

program
    .command('render')
    .description('Render the view the first model file declares')
    .argument('<models...>', 'one or more .sysml files or directories (linked as one workspace)')
    .requiredOption('-o, --out <file>', 'output SVG file path')
    .option('-t, --title <heading>', "override the heading (default: the view's name)")
    .option('--theme <name>', `paint the diagram with a theme (${Object.keys(themes).join(', ')})`, 'light')
    .action(async (modelPaths: string[], options: { out: string; title?: string; theme: string }) => {
        const theme = getTheme(options.theme);
        if (!theme) {
            console.error(`Unknown theme "${options.theme}". Available: ${Object.keys(themes).join(', ')}.`);
            process.exitCode = 1;
            return;
        }
        const inputs = await Promise.all(modelPaths.map(expandInput));
        const uniqueFiles = [...new Set(inputs.flatMap(i => i.files).map(f => f.replace(/\\/g, '/')))];
        if (uniqueFiles.length === 0) {
            console.error('No .sysml files found in the given inputs.');
            process.exitCode = 1;
            return;
        }

        const services = createSysmlServices();
        const { model, documents } = await parseSysmlFiles(services.Sysml, uniqueFiles);

        const views = declaredViews(documents[0].parseResult.value);
        if (views.length !== 1) {
            console.error(`${uniqueFiles[0]} declares ${views.length === 0 ? 'no view' : `${views.length} views`}; `
                + 'ortho renders the one view its first file declares.');
            process.exitCode = 1;
            return;
        }

        const firstInput = modelPaths[0].replace(/\\/g, '/');
        const title: DiagramTitle = {
            heading: options.title ?? viewHeading(views[0]),
            source: inputs[0].isDirectory ? `${firstInput}/*.sysml` : uniqueFiles.join(', ')
        };

        const result = await renderView(views[0], model, title, theme);
        await mkdir(dirname(options.out), { recursive: true });
        await writeFile(options.out, result.svg, 'utf-8');
        console.log(`Wrote ${options.out} (${result.summary})`);
    });

await program.parseAsync(process.argv);
