'use strict';

const { test } = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const { instructionFiles } = require('../src/instructions');

const language = fs.readFileSync(path.join(__dirname, '..', 'ai', 'LANGUAGE.md'), 'utf8');
const byPath = (files) => Object.fromEntries(files.map((file) => [file.path, file.content]));

test('writes a skill for Claude Code, with the language beside it, and instructions for Copilot', () => {
  const files = byPath(instructionFiles(language));
  assert.deepEqual(Object.keys(files), [
    '.claude/skills/sysml-v2-ortho/SKILL.md',
    '.claude/skills/sysml-v2-ortho/LANGUAGE.md',
    '.github/instructions/sysml-v2.instructions.md',
  ]);
  const skill = files['.claude/skills/sysml-v2-ortho/SKILL.md'];
  assert.match(skill, /^---\nname: sysml-v2-ortho\ndescription: .*\.sysml.*\n---\n/);
  assert.match(skill, /\[LANGUAGE\.md\]\(LANGUAGE\.md\), beside this file,/);
  assert.equal(files['.claude/skills/sysml-v2-ortho/LANGUAGE.md'], language);
  assert.match(files['.github/instructions/sysml-v2.instructions.md'], /^---\napplyTo: "\*\*\/\*\.sysml"\n---\n/);
});

test('tells the agent to check its work with ortho and to fix the cause, not the line', () => {
  for (const content of [instructionFiles(language)[0].content, instructionFiles(language)[2].content]) {
    assert.match(content, /ortho render <model folder> -o <a temporary folder>/);
    assert.match(content, /Fix every error and every warning before you finish/);
    assert.match(content, /Never delete or comment out the line a problem points at/);
  }
});

test('carries the whole language in the Copilot instructions, a heading level deeper', () => {
  const copilot = instructionFiles(language)[2].content;
  assert.match(copilot, /\n## The SysML v2 that ortho reads\n/);
  assert.match(copilot, /\n### Logical view\n/);
  // A line starting with # inside a code block is SysML, not a heading.
  const fenced = instructionFiles('# Title\n\n```sysml\n#refinement dependency A to B;\n```\n')[2].content;
  assert.match(fenced, /\n## Title\n/);
  assert.match(fenced, /\n#refinement dependency A to B;\n/);
});

test('carries a current copy of LANGUAGE.md, whose links to the repository point at GitHub', () => {
  const original = fs.readFileSync(path.join(__dirname, '..', '..', '..', 'LANGUAGE.md'), 'utf8');
  const relinked = original.replace(/\]\((?!http|#)/g, '](https://github.com/ola-coding/ortho/blob/main/');
  assert.equal(language.slice(language.indexOf('# The SysML v2')), relinked);
});
