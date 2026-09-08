// Renders every view of each bundled example into its own diagrams/ folder.
// Run with: npm run render:examples
import { mkdir, writeFile } from 'node:fs/promises';
import { createSysmlServices } from '../src/parser/sysml-module.js';
import { parseSysmlFiles } from '../src/parser/parse.js';
import { diagramTypes } from '../src/pipeline.js';

interface Job {
    out: string;
    diagram: keyof typeof diagramTypes;
    /** First file is the view's own model; the rest supply types it references. */
    files: string[];
}

/**
 * One model file per view. Four of the six stand alone; deployment is nothing
 * but the mapping between the other two, and the process view's lifelines are
 * typed by the software modules, so both are rendered with their companions.
 */
function standardJobs(): Job[] {
    return [
        { out: 'use-case-view.svg', diagram: 'use-case', files: ['use-case.sysml'] },
        { out: 'logical-view.svg', diagram: 'logical', files: ['logical.sysml'] },
        { out: 'implementation-view.svg', diagram: 'implementation', files: ['implementation.sysml'] },
        { out: 'physical-view.svg', diagram: 'physical', files: ['physical.sysml'] },
        {
            out: 'deployment-view.svg', diagram: 'deployment',
            files: ['deployment.sysml', 'implementation.sysml', 'physical.sysml']
        },
        {
            out: 'process-view.svg', diagram: 'process',
            files: ['process.sysml', 'implementation.sysml']
        }
    ];
}

const examples = [
    { base: 'examples/auv-system', system: 'AUV System' },
    { base: 'examples/coffee-machine', system: 'Coffee Machine' }
];

for (const example of examples) {
    const out = `${example.base}/diagrams`;
    await mkdir(out, { recursive: true });
    for (const job of standardJobs()) {
        const services = createSysmlServices();
        const { model } = await parseSysmlFiles(services.Sysml, job.files.map(f => `${example.base}/${f}`));
        const result = await diagramTypes[job.diagram].render(model, {
            heading: `${diagramTypes[job.diagram].view} — ${example.system}`,
            source: `${example.base}/${job.files[0]}`
        });
        await writeFile(`${out}/${job.out}`, result.svg, 'utf-8');
        console.log(`Wrote ${out}/${job.out} (${result.summary})`);
    }
}
