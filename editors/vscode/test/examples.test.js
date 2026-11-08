'use strict';

const { test } = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const { EXAMPLES, freeName, firstFile } = require('../src/examples');

const carried = path.join(__dirname, '..', 'examples');
const repository = path.join(__dirname, '..', '..', '..', 'examples');
const folders = (root) =>
  fs
    .readdirSync(root, { withFileTypes: true })
    .filter((entry) => entry.isDirectory())
    .map((entry) => entry.name)
    .sort();

test('offers every example of the repository, and only those', () => {
  const names = EXAMPLES.map((example) => example.name).sort();
  assert.deepEqual(names, folders(repository));
  assert.deepEqual(names, folders(carried));
});

test('carries a current copy of each example model, without its diagrams', () => {
  for (const { name } of EXAMPLES) {
    const models = fs.readdirSync(path.join(repository, name)).filter((file) => file.endsWith('.sysml')).sort();
    assert.deepEqual(fs.readdirSync(path.join(carried, name)).sort(), models);
    for (const file of models) {
      assert.equal(
        fs.readFileSync(path.join(carried, name, file), 'utf8'),
        fs.readFileSync(path.join(repository, name, file), 'utf8'),
        `${name}/${file} differs: run cargo xtask vscode-grammar`,
      );
    }
    assert.equal(firstFile(models), 'use-case.sysml');
  }
});

test('copies next to an earlier copy, never over it', () => {
  assert.equal(freeName('survey-drone', () => false), 'survey-drone');
  const taken = new Set(['survey-drone', 'survey-drone-2']);
  assert.equal(freeName('survey-drone', (name) => taken.has(name)), 'survey-drone-3');
});

test('opens the use case view first, else the first model file', () => {
  assert.equal(firstFile(['logical.sysml', 'use-case.sysml', 'diagrams']), 'use-case.sysml');
  assert.equal(firstFile(['notes.md', 'physical.sysml', 'logical.sysml']), 'logical.sysml');
});
