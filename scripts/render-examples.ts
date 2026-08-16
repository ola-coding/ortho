// Renders every 4+1 view of each bundled example model into its own
// diagrams/ folder. Run with: npm run render:examples
import { mkdir, writeFile } from 'node:fs/promises';
import { createSysmlServices } from '../src/parser/sysml-module.js';
import { parseSysmlFiles } from '../src/parser/parse.js';
import { diagramTypes } from '../src/pipeline.js';

interface Job {
    out: string;
    diagram: keyof typeof diagramTypes;
    heading: string;
    /** First file is the diagram's subject; the rest supply referenced packages. */
    files: string[];
}

interface Example {
    base: string;
    jobs: Job[];
}

/** Which files each of the five views is rendered from. Layouts differ per example. */
interface ViewFiles {
    logical: string[];
    scenarios: string[];
    development: string[];
    physical: string[];
    process: string[];
}

/** The five 4+1 views. Headings and diagram types are fixed; file lists are not. */
function standardJobs(system: string, scenario: { file: string; heading: string }, files: ViewFiles): Job[] {
    return [
        {
            out: 'logical-view.svg', diagram: 'part-definition',
            heading: `Logical View — ${system}`,
            files: files.logical
        },
        {
            out: 'scenarios-view.svg', diagram: 'use-case',
            heading: `Scenarios — Operating the ${system}`,
            files: files.scenarios
        },
        {
            out: 'development-view.svg', diagram: 'package',
            heading: `Development View — ${system}`,
            files: files.development
        },
        {
            out: 'physical-view.svg', diagram: 'allocation',
            heading: 'Physical View — Software Deployment',
            files: files.physical
        },
        {
            out: scenario.file.replace('scenario-', 'process-view-').replace('.sysml', '.svg'),
            diagram: 'sequence',
            heading: `Process View — ${scenario.heading}`,
            files: files.process
        }
    ];
}

// The development view lists the architectural packages only — scenarios are
// deliberately left out. A scenario is an instance of behaviour, not a unit of
// the architecture: its only import is the package it exercises, and a project
// with twenty scenarios would put twenty leaf nodes on the package diagram, all
// pointing at the same place. They belong on the process view.

// Both examples share one partition: a system level (requirements, use-cases,
// system) over mechanics, electronics and software. Every domain depends only on
// system.sysml, so the design file list is the architecture plus the domains
// that realize it.
const DOMAINS = [
    'system.sysml', 'mechanics.sysml', 'electronics.sysml', 'requirements.sysml'
];
const ARCHITECTURE = [
    'requirements.sysml', 'system.sysml', 'mechanics.sysml', 'electronics.sysml',
    'software.sysml', 'use-cases.sysml'
];
const DEPLOY = ['software.sysml', ...DOMAINS];

/** Every view but the process view is the same file list in both examples. */
function viewFiles(scenario: string): ViewFiles {
    return {
        logical: DOMAINS,
        scenarios: ['use-cases.sysml', ...DOMAINS],
        development: ARCHITECTURE,
        physical: DEPLOY,
        process: [scenario, ...DEPLOY]
    };
}

// No per-domain logical views. Rendering one is possible — pass just that
// domain's file plus system.sysml — but the result is a strict subset of the
// full logical view with nothing of its own, because file selection cannot
// subset system.sysml down to the blocks one domain realizes. A domain view only
// becomes worth committing once scoping is finer than a file; see the
// `view def` / `viewpoint` item in DEVELOPMENT.md.
const SCENARIO = {
    'examples/auv-system': { file: 'scenario-follow-person.sysml', heading: 'Follow Person' },
    'examples/coffee-machine': { file: 'scenario-make-cappuccino.sysml', heading: 'Make Cappuccino' }
};

const examples: Example[] = [
    {
        base: 'examples/auv-system',
        jobs: standardJobs('AUV System', SCENARIO['examples/auv-system'],
            viewFiles(SCENARIO['examples/auv-system'].file))
    },
    {
        base: 'examples/coffee-machine',
        jobs: standardJobs('Coffee Machine', SCENARIO['examples/coffee-machine'],
            viewFiles(SCENARIO['examples/coffee-machine'].file))
    }
];

for (const example of examples) {
    const out = `${example.base}/diagrams`;
    await mkdir(out, { recursive: true });
    for (const job of example.jobs) {
        const services = createSysmlServices();
        const { model } = await parseSysmlFiles(services.Sysml, job.files.map(f => `${example.base}/${f}`));
        const result = await diagramTypes[job.diagram].render(model, {
            heading: job.heading,
            source: `${example.base}/${job.files[0]}`
        });
        await writeFile(`${out}/${job.out}`, result.svg, 'utf-8');
        console.log(`Wrote ${out}/${job.out} (${result.summary})`);
    }
}
