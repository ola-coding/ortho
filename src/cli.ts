import { mkdir, readFile, writeFile, readdir, stat } from 'node:fs/promises';
import { join } from 'node:path';
import { Command } from 'commander';
import { createSysmlServices } from './parser/sysml-module.js';
import { parseSysmlFiles } from './parser/parse.js';
import { renderViews } from './pipeline.js';
import { viewHeading } from './model/views.js';
import { getTheme, themes } from './render/theme.js';

/** A file, or the .sysml files in a folder. */
async function expandInput(path: string): Promise<string[]> {
    if (!(await stat(path)).isDirectory()) {
        return [path];
    }
    return (await readdir(path)).filter(e => e.endsWith('.sysml')).sort().map(e => join(path, e));
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
    .description('Render every view the model declares, one SVG each')
    .argument('<models...>', 'one or more .sysml files or folders, read as one model')
    .requiredOption('-o, --out <folder>', 'the folder to write the SVGs to, each named after its view')
    .option('--theme <name>', `paint the diagrams with a theme (${Object.keys(themes).join(', ')})`, 'light')
    .action(async (modelPaths: string[], options: { out: string; theme: string }) => {
        const theme = getTheme(options.theme);
        if (!theme) {
            console.error(`Unknown theme "${options.theme}". Available: ${Object.keys(themes).join(', ')}.`);
            process.exitCode = 1;
            return;
        }
        const inputs = await Promise.all(modelPaths.map(expandInput));
        const uniqueFiles = [...new Set(inputs.flat().map(f => f.replace(/\\/g, '/')))];
        if (uniqueFiles.length === 0) {
            console.error('No .sysml files found in the given inputs.');
            process.exitCode = 1;
            return;
        }

        try {
            const parsed = await parseSysmlFiles(createSysmlServices().Sysml, uniqueFiles);
            for (const warning of parsed.warnings) {
                console.error(`Warning: ${warning}`);
            }
            const { rendered, skipped } = await renderViews(parsed, theme);
            for (const { view, reason } of skipped) {
                console.error(`Skipped '${viewHeading(view)}': ${reason}`);
            }
            if (rendered.length === 0) {
                console.error(skipped.length > 0 ? 'The model declares no view ortho can draw.'
                    : "The model declares no view. ortho draws the views a model declares, such as view 'Functions' : GeneralView { expose Functions; }.");
                process.exitCode = 1;
                return;
            }
            await mkdir(options.out, { recursive: true });
            for (const { file, result } of rendered) {
                const path = join(options.out, file);
                await writeFile(path, result.svg, 'utf-8');
                console.log(`Wrote ${path} (${result.summary})`);
            }
        } catch (error) {
            // A model that does not load is reported, not a crash.
            console.error(error instanceof Error ? error.message : String(error));
            process.exitCode = 1;
        }
    });

await program.parseAsync(process.argv);
