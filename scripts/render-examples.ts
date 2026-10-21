// Renders every view of each bundled example into its own diagrams/ folder.
// Run with: npm run render:examples
import { mkdir, writeFile } from 'node:fs/promises';
import { createSysmlServices } from '../src/parser/sysml-module.js';
import { parseSysmlFiles } from '../src/parser/parse.js';
import { declaredViews, viewHeading } from '../src/model/views.js';
import { renderView } from '../src/pipeline.js';

interface Job {
    out: string;
    /** First file declares the view; the rest supply what it refers to. */
    files: string[];
}

/**
 * One model file per view. Use case and logical stand alone; the other four
 * are rendered with their companions, because their content is a relation
 * between files: implementation and physical name the functions their parts
 * perform, deployment is nothing but the mapping between those two, and the
 * process view's lifelines are typed by the software modules.
 */
const jobs: Job[] = [
    { out: 'use-case-view.svg', files: ['use-case.sysml'] },
    { out: 'logical-view.svg', files: ['logical.sysml'] },
    { out: 'implementation-view.svg', files: ['implementation.sysml', 'logical.sysml'] },
    { out: 'physical-view.svg', files: ['physical.sysml', 'logical.sysml'] },
    { out: 'deployment-view.svg', files: ['deployment.sysml', 'implementation.sysml', 'physical.sysml', 'logical.sysml'] },
    { out: 'process-view.svg', files: ['process.sysml', 'implementation.sysml', 'logical.sysml'] }
];

for (const base of ['examples/survey-drone', 'examples/coffee-machine']) {
    const out = `${base}/diagrams`;
    await mkdir(out, { recursive: true });
    for (const job of jobs) {
        const { model, documents } = await parseSysmlFiles(createSysmlServices().Sysml, job.files.map(f => `${base}/${f}`));
        const [view] = declaredViews(documents[0].parseResult.value);
        const result = await renderView(view, model, { heading: viewHeading(view), source: `${base}/${job.files[0]}` });
        await writeFile(`${out}/${job.out}`, result.svg, 'utf-8');
        console.log(`Wrote ${out}/${job.out} (${result.summary})`);
    }
}
