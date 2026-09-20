import { readFile, writeFile, readdir, stat } from 'node:fs/promises';
import { basename, join } from 'node:path';
import { Command } from 'commander';
import { createSysmlServices } from './parser/sysml-module.js';
import { parseSysmlFiles } from './parser/parse.js';
import type { DiagramTitle } from './render/svg-renderer.js';
import { diagramTypes, getDiagramType } from './pipeline.js';
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
    .description('Render one diagram from a set of model files')
    .argument('<models...>', 'one or more .sysml files or directories (linked as one workspace)')
    .requiredOption('-d, --diagram <type>', `diagram type to render (${Object.keys(diagramTypes).join(', ')})`)
    .requiredOption('-o, --out <file>', 'output SVG file path')
    .option('-t, --title <heading>', 'override the frame heading (default: "<view> — <root packages of first input>")')
    .option('--theme <name>', `paint the diagram with a theme (${Object.keys(themes).join(', ')})`, 'light')
    .action(async (modelPaths: string[], options: { diagram: string; out: string; title?: string; theme: string }) => {
        const theme = getTheme(options.theme);
        if (!theme) {
            console.error(`Unknown theme "${options.theme}". Available: ${Object.keys(themes).join(', ')}.`);
            process.exitCode = 1;
            return;
        }
        const diagramType = getDiagramType(options.diagram);
        if (!diagramType) {
            console.error(`Unsupported diagram type "${options.diagram}". Available: ${Object.keys(diagramTypes).join(', ')}.`);
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

        // Heading identity comes from the first input: its root packages, or
        // the directory name when a directory was given.
        const firstInput = modelPaths[0].replace(/\\/g, '/');
        const firstIsDir = inputs[0].isDirectory;
        const hint = firstIsDir
            ? basename(firstInput)
            : documents[0].parseResult.value.packages.map(p => p.name).join(', ');
        const title: DiagramTitle = {
            heading: options.title ?? (hint ? `${diagramType.view} — ${hint}` : diagramType.view),
            source: firstIsDir ? `${firstInput}/*.sysml` : uniqueFiles.join(', ')
        };

        const result = await diagramType.render(model, title, theme);
        await writeFile(options.out, result.svg, 'utf-8');
        console.log(`Wrote ${options.out} (${result.summary})`);
    });

await program.parseAsync(process.argv);
