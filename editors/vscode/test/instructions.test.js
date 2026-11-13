'use strict';

const { test } = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const { instructionFiles, DESCRIPTION, SKILL } = require('../src/instructions');

const repository = path.join(__dirname, '..', '..', '..');
const read = (name) => fs.readFileSync(path.join(__dirname, '..', 'ai', `${name}.md`), 'utf8');
const ai = Object.fromEntries(['LANGUAGE', 'VIEWS', 'WORKFLOW', 'MISTAKES', 'AGENTS'].map((name) => [name, read(name)]));
const files = instructionFiles(ai);
const byPath = Object.fromEntries(files.map((file) => [file.path, file.content]));
const skill = byPath[`.claude/skills/${SKILL}/SKILL.md`];
const copilot = byPath['.github/instructions/sysml-v2.instructions.md'];

test('writes a skill with its references, Copilot instructions, and an AGENTS.md', () => {
  assert.deepEqual(Object.keys(byPath), [
    '.claude/skills/sysml-v2-ortho/SKILL.md',
    '.claude/skills/sysml-v2-ortho/LANGUAGE.md',
    '.claude/skills/sysml-v2-ortho/VIEWS.md',
    '.claude/skills/sysml-v2-ortho/MISTAKES.md',
    '.github/instructions/sysml-v2.instructions.md',
    'AGENTS.md',
  ]);
  assert.equal(byPath['.claude/skills/sysml-v2-ortho/LANGUAGE.md'], ai.LANGUAGE);
  assert.match(copilot, /^---\napplyTo: "\*\*\/\*\.sysml"\n---\n/);
});

// The Agent Skills format: a name that is the folder's, lower case and hyphens, and a
// description of at most 1024 characters with no angle brackets.
test('the skill is a valid Agent Skill', () => {
  const front = skill.match(/^---\nname: (.*)\ndescription: (.*)\n---\n/);
  assert.ok(front, 'frontmatter');
  assert.equal(front[1], SKILL);
  assert.match(front[1], /^[a-z0-9]+(-[a-z0-9]+)*$/);
  assert.equal(front[2], DESCRIPTION);
  assert.ok(DESCRIPTION.length <= 1024);
  assert.doesNotMatch(DESCRIPTION, /[<>]/);
});

test('every relative link in what is written leads to a file that is written', () => {
  for (const file of files) {
    for (const [, target] of file.content.matchAll(/\]\(([^)#]+)(#[^)]*)?\)/g)) {
      if (/^https?:/.test(target)) continue;
      const to = path.posix.normalize(path.posix.join(path.posix.dirname(file.path), target));
      assert.ok(to in byPath, `${file.path} links to ${target}`);
    }
  }
});

test('tells the agent to check its work with ortho and to fix the cause, not the line', () => {
  for (const content of [skill, copilot]) {
    assert.match(content, /ortho render <model folder> -o <a temporary folder>/);
    assert.match(content, /Fix every error and every warning before you finish/);
    assert.match(content, /Never delete or comment out the line a problem points at/);
  }
});

test('tells the agent to split a view too big for its page, never to drop elements', () => {
  for (const content of [skill, copilot, byPath['AGENTS.md']]) {
    assert.match(content, /too (wide|big) for|fit its page/);
    assert.match(content, /[Nn]ever (merge|drop)/);
  }
});

test('carries the language and the mistakes in the Copilot instructions, a heading level deeper', () => {
  assert.match(copilot, /\n## The SysML v2 that ortho reads\n/);
  assert.match(copilot, /\n### Logical view\n/);
  assert.match(copilot, /\n## Mistakes to avoid\n/);
  assert.doesNotMatch(copilot, /<!-- Copied/);
  // A line starting with # inside a code block is SysML, not a heading.
  const fenced = instructionFiles({ ...ai, LANGUAGE: '# Title\n\n```sysml\n#refinement dependency A to B;\n```\n' });
  const text = fenced.find((file) => file.path.endsWith('.instructions.md')).content;
  assert.match(text, /\n## Title\n/);
  assert.match(text, /\n#refinement dependency A to B;\n/);
});

test('writes AGENTS.md only where no other agent file is', () => {
  const agents = files.find((file) => file.path === 'AGENTS.md');
  assert.deepEqual(agents.unlessAny, ['AGENTS.md', 'AGENT.md', 'CLAUDE.md', 'GEMINI.md']);
  assert.match(agents.content, /^# SysML v2 models\n/);
  assert.match(agents.content, /\.claude\/skills\/sysml-v2-ortho\/SKILL\.md/);
});

// What the instructions say is written once, in the repository; the extension carries
// copies, which `cargo xtask vscode-grammar` writes and CI checks are current.
test('carries current copies of the files it is written from', () => {
  for (const [name, from] of [
    ['LANGUAGE', 'LANGUAGE.md'],
    ['VIEWS', 'VIEWS.md'],
    ['WORKFLOW', 'ai/WORKFLOW.md'],
    ['MISTAKES', 'ai/MISTAKES.md'],
    ['AGENTS', 'ai/AGENTS.md'],
  ]) {
    const original = fs.readFileSync(path.join(repository, from), 'utf8');
    assert.ok(ai[name].startsWith(`<!-- Copied from ortho's ${from} `), name);
    const strip = (text) => text.replace(/\]\([^)]*\)/g, '](…)');
    assert.equal(strip(ai[name].replace(/^<!--.*?-->\n\n/, '')), strip(original), name);
  }
});
