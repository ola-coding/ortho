#!/usr/bin/env node
// CLI entry point: runs the compiled tool (npm run build produces dist/).
import { existsSync } from 'node:fs';
import { fileURLToPath } from 'node:url';

const cli = new URL('../dist/cli.js', import.meta.url);
if (!existsSync(fileURLToPath(cli))) {
    console.error('ortho: dist/ is not built. Run `npm run build` in the ortho package first.');
    process.exit(1);
}
await import(cli.href);
