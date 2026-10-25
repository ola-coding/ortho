// Fetches OMG's SysML v2 models into .omg-models/, at the release the tests
// count against, with the standard library scripts/library-names.ts reads.
// Run with: npm run omg-models
//
// They are the example, training and validation models of OMG's
// SysML-v2-Release. They are EPL-2.0, so they are fetched, not committed.
import { execFileSync } from 'node:child_process';
import { existsSync, rmSync } from 'node:fs';

const DIR = '.omg-models';
const REPO = 'https://github.com/Systems-Modeling/SysML-v2-Release.git';
/** The 2026-09 release. */
const COMMIT = 'b7a6b77000c9136f8f24822fc9d2cb237c76267f';

const git = (...args: string[]): string =>
    execFileSync('git', ['-C', DIR, ...args], { encoding: 'utf8', stdio: ['ignore', 'pipe', 'pipe'] }).trim();

function fetchedCommit(): string | undefined {
    try {
        return existsSync(DIR) ? git('rev-parse', 'HEAD') : undefined;
    } catch {
        return undefined;
    }
}

const FOLDERS = ['sysml/src/examples', 'sysml/src/training', 'sysml/src/validation', 'sysml.library'];

if (fetchedCommit() === COMMIT) {
    git('sparse-checkout', 'set', ...FOLDERS);
    console.log(`${DIR}/ holds OMG's models at ${COMMIT.slice(0, 7)}`);
} else {
    rmSync(DIR, { recursive: true, force: true });
    execFileSync('git', ['init', '-q', DIR]);
    git('remote', 'add', 'origin', REPO);
    git('sparse-checkout', 'set', ...FOLDERS);
    git('fetch', '-q', '--depth', '1', '--filter=blob:none', 'origin', COMMIT);
    git('checkout', '-q', 'FETCH_HEAD');
    console.log(`Fetched OMG's models at ${COMMIT.slice(0, 7)} into ${DIR}/`);
}
