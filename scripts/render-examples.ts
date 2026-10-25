// Renders every view of each bundled example into its own diagrams/ folder, as
// `ortho render <example> -o <example>/diagrams` does.
// Run with: npm run render:examples
import { mkdir, readdir, rm, writeFile } from 'node:fs/promises';
import { createSysmlServices } from '../src/parser/sysml-module.js';
import { parseSysmlFiles } from '../src/parser/parse.js';
import { renderViews } from '../src/pipeline.js';

for (const base of ['examples/survey-drone', 'examples/coffee-machine']) {
    const files = (await readdir(base)).filter(f => f.endsWith('.sysml')).sort().map(f => `${base}/${f}`);
    const { rendered } = await renderViews(await parseSysmlFiles(createSysmlServices().Sysml, files));
    const out = `${base}/diagrams`;
    await mkdir(out, { recursive: true });
    // A view that is gone leaves no diagram behind.
    for (const stale of (await readdir(out)).filter(f => f.endsWith('.svg'))) {
        await rm(`${out}/${stale}`);
    }
    for (const { file, result } of rendered) {
        await writeFile(`${out}/${file}`, result.svg, 'utf-8');
        console.log(`Wrote ${out}/${file} (${result.summary})`);
    }
}
