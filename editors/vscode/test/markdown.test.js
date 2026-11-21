// The preview page reads ortho's tables of requirements with media/markdown.js. These
// tests read the examples' tables with it, and check its anchors against ortho's.
'use strict';

const test = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');
const { parse, inline, plainText } = require('../media/markdown');
const { runOrtho } = require('../src/ortho');

const repository = path.resolve(__dirname, '..', '..', '..');

/** Every link in a table's blocks. */
function links(/** @type {any[]} */ blocks) {
  /** @type {string[]} */
  const found = [];
  const walk = (/** @type {any[]} */ children) => {
    for (const child of children) {
      if (child.type === 'link') found.push(child.href);
      if (child.children) walk(child.children);
    }
  };
  for (const block of blocks) {
    if (block.children) walk(block.children);
    if (block.type === 'table') {
      block.head.forEach(walk);
      block.rows.forEach((/** @type {any[][]} */ row) => row.forEach(walk));
    }
  }
  return found;
}

/** Each link to a requirement in the table names a heading the page marks. */
function assertAnchorsResolve(/** @type {string} */ markdown) {
  const blocks = parse(markdown);
  const anchors = new Set(blocks.filter((block) => block.type === 'heading').map((block) => block.id));
  const targets = links(blocks).filter((href) => href.startsWith('#'));
  assert.ok(targets.length > 0);
  for (const target of targets) assert.ok(anchors.has(target.slice(1)), target);
}

for (const example of ['coffee-machine', 'survey-drone', 'web-shop']) {
  test(`reads the ${example}'s table of requirements`, () => {
    const markdown = fs.readFileSync(
      path.join(repository, 'examples', example, 'diagrams', `requirements-${example}.md`),
      'utf8',
    );
    const blocks = parse(markdown);
    assert.equal(blocks[0].type, 'heading');
    assert.equal(blocks[0].level, 1);
    // A box under each requirement's heading: its ID and statement, and its rationale where it has one.
    const boxes = blocks.filter((block) => block.type === 'table' && plainText(block.head[1]) === 'Statement');
    assert.ok(boxes.length >= 14);
    for (const box of boxes) {
      assert.deepEqual(box.head.map(plainText).slice(0, 2), ['ID', 'Statement']);
      assert.match(plainText(box.rows[0][0]), /^REQ-\d+(\.\d+)*$/);
      assert.match(plainText(box.rows[0][1]), /\bshall\b/);
    }
    const trace = blocks.find((block) => block.type === 'table' && plainText(block.head[1]) === 'Requirement');
    assert.equal(trace?.rows.length, boxes.length);
    assertAnchorsResolve(markdown);
    for (const href of links(blocks).filter((target) => !target.startsWith('#'))) {
      assert.match(href, /^\.\.\/[a-z-]+\.sysml(#L\d+)?$/);
    }
  });
}

test('reads what ortho escapes, breaks and refers to as written', () => {
  assert.deepEqual(inline('a \\*b\\* \\| c\\\\d \\[e\\]'), [{ type: 'text', text: 'a *b* | c\\d [e]' }]);
  assert.deepEqual(inline('one<br><br>two'), [
    { type: 'text', text: 'one' },
    { type: 'br' },
    { type: 'br' },
    { type: 'text', text: 'two' },
  ]);
  assert.deepEqual(inline('&emsp;&emsp;Leg &amp; track &#176;'), [{ type: 'text', text: '\u2003\u2003Leg & track °' }]);
  assert.deepEqual(inline('<sub>From [a \\] b](../a%20b.sysml#L3).</sub>'), [
    {
      type: 'sub',
      children: [
        { type: 'text', text: 'From ' },
        { type: 'link', href: '../a%20b.sysml#L3', children: [{ type: 'text', text: 'a ] b' }] },
        { type: 'text', text: '.' },
      ],
    },
  ]);
  const [table] = parse('| ID | Statement |\n|:---|:---|\n| REQ-1 | a \\| b |\n');
  assert.equal(table.type, 'table');
  assert.deepEqual(table.rows[0].map(plainText), ['REQ-1', 'a | b']);
});

const ortho = process.env.ORTHO_BIN;

test("marks each heading with the anchor ortho links to", { skip: !ortho && 'set ORTHO_BIN' }, async () => {
  const model = fs.mkdtempSync(path.join(os.tmpdir(), 'ortho-anchors-'));
  fs.writeFileSync(
    path.join(model, 'requirements.sysml'),
    [
      'package SystemRequirements {',
      '    private import StandardViewDefinitions::*;',
      "    requirement <'REQ-1'> 'Brew time' {",
      "        doc /* The machine shall brew within 60 s. */",
      "        requirement <'REQ-1.1'> 'Brew time' { doc /* The machine shall heat within 30 s. */ }",
      '    }',
      "    requirement <'REQ-2'> 'Größe (max.) & Gewicht' { doc /* The machine shall weigh at most 9 kg. */ }",
      "    requirement <'REQ-3'> 'Requirements' { doc /* The machine shall fit. */ }",
      "    requirement 'No ID_here' { doc /* The machine shall work. */ }",
      "    view 'Requirements' : GridView { expose SystemRequirements; }",
      '}',
      '',
    ].join('\n'),
  );
  const out = path.join(model, 'out');
  await runOrtho(String(ortho), model, out, 'light');
  const markdown = fs.readFileSync(path.join(out, 'requirements.md'), 'utf8');
  assertAnchorsResolve(markdown);
  const headings = parse(markdown).filter((block) => block.type === 'heading').map((block) => block.id);
  assert.deepEqual(headings.slice(0, 6), ['requirements', 'brew-time', 'brew-time-1', 'größe-max--gewicht', 'requirements-1', 'no-id_here']);
  fs.rmSync(model, { recursive: true });
});
