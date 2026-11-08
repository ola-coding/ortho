// What the extension's package.json promises: the version of the ortho it carries,
// and a folder VS Code does not trust never naming the program it runs.
'use strict';

const test = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');

const manifest = JSON.parse(fs.readFileSync(path.join(__dirname, '..', 'package.json'), 'utf8'));
const repository = path.resolve(__dirname, '..', '..', '..');

test('has the version of the ortho it carries, as Cargo.toml has it', () => {
  const cargo = fs.readFileSync(path.join(repository, 'Cargo.toml'), 'utf8');
  const version = /^version = "(.*)"$/m.exec(cargo)?.[1];
  assert.equal(manifest.version, version);
});

test('ignores the ortho a folder it does not trust names', () => {
  const untrusted = manifest.capabilities.untrustedWorkspaces;
  assert.equal(untrusted.supported, 'limited');
  assert.deepEqual(untrusted.restrictedConfigurations, ['ortho.path']);
});
