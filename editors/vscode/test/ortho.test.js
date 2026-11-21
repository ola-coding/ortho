// The parts of the extension that need no VS Code: reading what ortho prints, and
// running it. Run with `npm test`; set ORTHO_BIN to an ortho binary to run it
// on the examples too, as CI does.
'use strict';

const test = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');
const {
  readReport,
  viewTitle,
  tabTitles,
  runOrtho,
  keepOnly,
  bundledOrtho,
  chooseOrtho,
  orthoFolder,
  makeExecutable,
  markedSource,
  mirrorModel,
  requirementLines,
  relinkMarkdown,
} = require('../src/ortho');

const repository = path.resolve(__dirname, '..', '..', '..');

test('reads the diagrams ortho wrote', () => {
  const report = readReport(
    'Wrote out/logical-view-coffee-machine.svg (24 node(s), 22 edge(s))\n' +
      'Wrote C:/my models/out/process view.svg (3 lifeline(s), 4 message(s))\r\n',
    '',
  );
  assert.deepEqual(report.written, [
    { file: 'out/logical-view-coffee-machine.svg', summary: '24 node(s), 22 edge(s)' },
    { file: 'C:/my models/out/process view.svg', summary: '3 lifeline(s), 4 message(s)' },
  ]);
  assert.deepEqual(report.problems, []);
});

test('reads the table of requirements ortho wrote', () => {
  const report = readReport('Wrote out/requirements-coffee-machine.md (15 requirement(s), 15 satisfied)\n', '');
  assert.deepEqual(report.written, [
    { file: 'out/requirements-coffee-machine.md', summary: '15 requirement(s), 15 satisfied' },
  ]);
  assert.equal(viewTitle('out/requirements-coffee-machine.md'), 'Requirements coffee machine');
  assert.deepEqual(tabTitles(['requirements-web-shop.md', 'use-case-view-web-shop.svg']), [
    'Requirements',
    'Use case view',
  ]);
});

test('reads the problems of a model that does not load', () => {
  const report = readReport(
    '',
    "Failed to parse:\n  /models/a.sysml line 3: Expected ';' or '{' but found '}'.\n" +
      "  C:/my models/b line 2.sysml line 14: Could not resolve reference to Definition named 'Nope'.\n",
  );
  assert.deepEqual(report.problems, [
    { file: '/models/a.sysml', line: 3, severity: 'error', message: "Expected ';' or '{' but found '}'." },
    {
      file: 'C:/my models/b line 2.sysml',
      line: 14,
      severity: 'error',
      message: "Could not resolve reference to Definition named 'Nope'.",
    },
  ]);
  assert.deepEqual(report.notes, []);
});

test('tells warnings at a line from other notes', () => {
  const report = readReport(
    '',
    "Warning: /m/a.sysml line 1: Package 'Base' is named after a standard library package.\n" +
      "Skipped 'Functions': it exposes nothing.\n" +
      "Warning: 'Logical view' is too big for landscape A4.\n",
  );
  assert.deepEqual(report.problems, [
    {
      file: '/m/a.sysml',
      line: 1,
      severity: 'warning',
      message: "Package 'Base' is named after a standard library package.",
    },
  ]);
  assert.deepEqual(report.notes, [
    "Skipped 'Functions': it exposes nothing.",
    "Warning: 'Logical view' is too big for landscape A4.",
  ]);
});

test('titles a view from its file name', () => {
  assert.equal(viewTitle('/out/logical-view-survey-drone.svg'), 'Logical view survey drone');
  assert.equal(viewTitle('C:/out/use_case.svg'), 'Use case');
  assert.equal(viewTitle('/out/.svg'), 'View');
});

test('leaves out of the tabs the words every title ends with', () => {
  const files = ['use-case-view-survey-drone.svg', 'logical-view-survey-drone.svg', 'process-view-survey-drone.svg'];
  assert.deepEqual(tabTitles(files), ['Use case', 'Logical', 'Process']);
  assert.deepEqual(tabTitles(['functions.svg', 'functions-detail.svg']), ['Functions', 'Functions detail']);
  assert.deepEqual(tabTitles(['a-view.svg', 'view.svg']), ['A view', 'View']);
  assert.deepEqual(tabTitles(['logical-view-survey-drone.svg']), ['Logical view survey drone']);
});

test('keeps only the latest render', () => {
  const base = fs.mkdtempSync(path.join(os.tmpdir(), 'ortho-keep-'));
  for (const name of ['1', '2', '3']) fs.mkdirSync(path.join(base, name));
  keepOnly(base, '3');
  assert.deepEqual(fs.readdirSync(base), ['3']);
  keepOnly(path.join(base, 'missing'), '3');
  fs.rmSync(base, { recursive: true });
});

test('runs the ortho the setting names, else the one the extension carries, else ortho on the PATH', () => {
  const bundled = bundledOrtho('/ext', 'linux');
  assert.equal(bundled, path.join('/ext', 'bin', 'ortho'));
  assert.equal(bundledOrtho('/ext', 'win32'), path.join('/ext', 'bin', 'ortho.exe'));
  assert.equal(chooseOrtho('/opt/ortho', bundled, () => true), '/opt/ortho');
  assert.equal(chooseOrtho(' ', bundled, () => true), bundled);
  assert.equal(chooseOrtho('', bundled, () => false), 'ortho');
  assert.equal(chooseOrtho(undefined, bundled, () => false), 'ortho');
});

test('puts the folder of the ortho the preview runs on the terminal PATH', () => {
  assert.equal(orthoFolder(path.join(path.sep, 'ext', 'bin', 'ortho')), path.join(path.sep, 'ext', 'bin'));
  // A bare command is already found on the PATH, and a folder of other things stays off it.
  assert.equal(orthoFolder('ortho'), undefined);
  assert.equal(orthoFolder(path.join(path.sep, 'usr', 'bin', 'env')), undefined);
});

test('makes the binary it carries runnable', { skip: process.platform === 'win32' }, () => {
  const file = path.join(fs.mkdtempSync(path.join(os.tmpdir(), 'ortho-bin-')), 'ortho');
  fs.writeFileSync(file, '', { mode: 0o644 });
  makeExecutable(file);
  assert.equal(fs.statSync(file).mode & 0o777, 0o755);
});

test('opens only a .sysml file of the model shown, from a box it marks', () => {
  const cwd = path.join(path.sep, 'work');
  const folder = path.join(cwd, 'examples', 'drone');
  const file = path.join(folder, 'logical.sysml');
  assert.deepEqual(markedSource(cwd, folder, 'examples/drone/logical.sysml', '12'), { file, line: 12 });
  assert.equal(markedSource(cwd, folder, 'examples/other/logical.sysml', '12'), undefined);
  assert.equal(markedSource(cwd, folder, 'examples/drone/sub/logical.sysml', '12'), undefined);
  assert.equal(markedSource(cwd, folder, 'examples/drone/../../secret.sysml', '1'), undefined);
  assert.equal(markedSource(cwd, folder, 'examples/drone/notes.txt', '1'), undefined);
  assert.equal(markedSource(cwd, folder, 'examples/drone/logical.sysml', 'x'), undefined);
  assert.equal(markedSource(cwd, folder, 'examples/drone/logical.sysml', '0'), undefined);
  assert.equal(markedSource(cwd, folder, null, '1'), undefined);
});

test('reports an ortho that cannot be found', async () => {
  const run = await runOrtho(path.join(os.tmpdir(), 'no-such-ortho'), '.', os.tmpdir(), 'light');
  assert.equal(run.missing, true);
});

test('copies a model with what its editors hold, where ortho will name it alike', () => {
  const cwd = fs.mkdtempSync(path.join(os.tmpdir(), 'ortho-workspace-'));
  const folder = path.join(cwd, 'models', 'machine');
  fs.mkdirSync(path.join(folder, 'diagrams'), { recursive: true });
  fs.writeFileSync(path.join(folder, 'a.sysml'), 'package A;\n');
  fs.writeFileSync(path.join(folder, 'b.sysml'), 'package B;\n');
  fs.writeFileSync(path.join(folder, 'notes.txt'), 'not a model\n');
  fs.writeFileSync(path.join(folder, 'diagrams', 'c.sysml'), 'package C;\n');
  const root = fs.mkdtempSync(path.join(os.tmpdir(), 'ortho-copy-'));
  const mirror = mirrorModel(folder, cwd, root, new Map([[path.join(folder, 'a.sysml'), 'package Typed;\n']]));
  assert.equal(mirror, path.join(root, 'models', 'machine'));
  // As ortho reads a folder: its own .sysml files, not those of folders inside it.
  assert.deepEqual(fs.readdirSync(mirror).sort(), ['a.sysml', 'b.sysml']);
  assert.equal(fs.readFileSync(path.join(mirror, 'a.sysml'), 'utf8'), 'package Typed;\n');
  assert.equal(fs.readFileSync(path.join(mirror, 'b.sysml'), 'utf8'), 'package B;\n');
  fs.rmSync(cwd, { recursive: true });
  fs.rmSync(root, { recursive: true });
});

test('finds where each requirement is declared, by its ID or its name', () => {
  const text = [
    'package P {',
    "    // requirement <'X-9'> 'Commented out';",
    "    requirement def <'D'> Definition;",
    "    requirement <'REQ-1'> 'Espresso time' {",
    "        requirement <'REQ-1.1'> 'Heat-up time' { doc /* requirement <'Y'> in a doc */ }",
    '    }',
    '    requirement withoutId;',
    '    requirement <REQ2> plain;',
    '}',
  ].join('\n');
  assert.deepEqual(requirementLines([{ file: 'm/r.sysml', text }]), {
    byId: {
      'REQ-1': { file: 'm/r.sysml', line: 4 },
      'REQ-1.1': { file: 'm/r.sysml', line: 5 },
      REQ2: { file: 'm/r.sysml', line: 8 },
    },
    byName: { withoutId: { file: 'm/r.sysml', line: 7 } },
  });
});

test("points a table's links at the model from where it is saved", () => {
  const markdown =
    '<sub>Rendered by ortho from [requirements.sysml](../requirements.sysml).</sub>\n' +
    '| [REQ-1](#espresso-time) | Espresso time | [Water tank](../my%20parts%28v2%29.sysml#L12) |\n' +
    'A statement with \\](brackets) in it.\n';
  const from = path.join(os.tmpdir(), 'copy', 'machine', 'diagrams');
  const to = path.join(os.tmpdir(), 'docs', 'specification');
  const real = (/** @type {string} */ file) => file.replace(path.join('copy', 'machine'), path.join('models', 'machine'));
  assert.equal(
    relinkMarkdown(markdown, from, to, real),
    '<sub>Rendered by ortho from [requirements.sysml](../../models/machine/requirements.sysml).</sub>\n' +
      '| [REQ-1](#espresso-time) | Espresso time | [Water tank](../../models/machine/my%20parts%28v2%29.sysml#L12) |\n' +
      'A statement with \\](brackets) in it.\n',
  );
});

const ortho = process.env.ORTHO_BIN;

test('renders an example with the real ortho', { skip: !ortho && 'set ORTHO_BIN' }, async () => {
  const out = fs.mkdtempSync(path.join(os.tmpdir(), 'ortho-render-'));
  const model = path.join(repository, 'examples', 'coffee-machine');
  const run = await runOrtho(String(ortho), model, out, 'light', repository);
  assert.equal(run.code, 0, run.stderr);
  const report = readReport(run.stdout, run.stderr);
  assert.equal(report.written.length, 7);
  for (const written of report.written) assert.ok(fs.existsSync(written.file), written.file);
  assert.ok(report.written.some((written) => written.file.endsWith('requirements-coffee-machine.md')));
  // The diagram names its model file from the folder ortho ran in.
  const logical = report.written.find((written) => written.file.includes('logical'));
  assert.match(fs.readFileSync(String(logical?.file), 'utf8'), /examples\/coffee-machine\/logical\.sysml/);
  assert.deepEqual(report.problems, []);
  fs.rmSync(out, { recursive: true });
});

test('marks every box with a model file the preview can open', { skip: !ortho && 'set ORTHO_BIN' }, async () => {
  const out = fs.mkdtempSync(path.join(os.tmpdir(), 'ortho-marks-'));
  const folder = path.join(repository, 'examples', 'coffee-machine');
  const run = await runOrtho(ortho, folder, out, 'light', repository);
  assert.equal(run.code, 0, run.stderr);
  const svg = fs.readFileSync(path.join(out, 'logical-view-coffee-machine.svg'), 'utf8');
  const marks = [...svg.matchAll(/data-file="([^"]+)" data-line="(\d+)"/g)];
  assert.ok(marks.length > 10);
  for (const [, file, line] of marks) {
    const source = markedSource(repository, folder, file, line);
    assert.ok(source, `${file} line ${line}`);
    assert.match(fs.readFileSync(source.file, 'utf8').split('\n')[source.line - 1], /\b(action|use case)\b/);
  }
});

test('reads the real ortho on a broken model', { skip: !ortho && 'set ORTHO_BIN' }, async () => {
  const model = fs.mkdtempSync(path.join(os.tmpdir(), 'ortho-broken-'));
  fs.writeFileSync(path.join(model, 'broken.sysml'), 'package P {\n    part def A {\n        part b : Nope;\n    }\n}\n');
  const run = await runOrtho(String(ortho), model, path.join(model, 'out'), 'light');
  assert.notEqual(run.code, 0);
  const report = readReport(run.stdout, run.stderr);
  assert.equal(report.written.length, 0);
  assert.equal(report.problems.length, 1);
  assert.equal(report.problems[0].line, 3);
  assert.equal(report.problems[0].severity, 'error');
  assert.equal(path.resolve(report.problems[0].file), path.join(model, 'broken.sysml'));
  fs.rmSync(model, { recursive: true });
});

test('a table saved elsewhere links as ortho writes it there', { skip: !ortho && 'set ORTHO_BIN' }, async () => {
  const model = path.join(repository, 'examples', 'web-shop');
  const first = fs.mkdtempSync(path.join(os.tmpdir(), 'ortho-first-'));
  const second = path.join(fs.mkdtempSync(path.join(os.tmpdir(), 'ortho-second-')), 'docs', 'specification');
  for (const out of [first, second]) assert.equal((await runOrtho(String(ortho), model, out, 'light', repository)).code, 0);
  const table = (/** @type {string} */ out) => fs.readFileSync(path.join(out, 'requirements-web-shop.md'), 'utf8');
  assert.equal(relinkMarkdown(table(first), first, second), table(second));
  fs.rmSync(first, { recursive: true });
  fs.rmSync(path.dirname(path.dirname(second)), { recursive: true });
});

test('renders the model as typed, naming its files as the model itself', { skip: !ortho && 'set ORTHO_BIN' }, async () => {
  const folder = path.join(repository, 'examples', 'coffee-machine');
  const direct = fs.mkdtempSync(path.join(os.tmpdir(), 'ortho-direct-'));
  assert.equal((await runOrtho(String(ortho), folder, direct, 'light', repository)).code, 0);
  const root = fs.mkdtempSync(path.join(os.tmpdir(), 'ortho-typed-'));
  const requirements = path.join(folder, 'requirements.sysml');
  const typed = fs.readFileSync(requirements, 'utf8').replace('at least 1.5 L of water', 'at least 1.8 L of water');
  const mirror = mirrorModel(folder, repository, root, new Map([[requirements, typed]]));
  const out = path.join(mirror, 'diagrams');
  const run = await runOrtho(String(ortho), mirror, out, 'light', root);
  assert.equal(run.code, 0, run.stderr);
  // Every box names the model's own file and line, as when ortho reads the model itself.
  for (const name of fs.readdirSync(direct).filter((file) => file.endsWith('.svg'))) {
    assert.equal(fs.readFileSync(path.join(out, name), 'utf8'), fs.readFileSync(path.join(direct, name), 'utf8'), name);
  }
  const table = fs.readFileSync(path.join(out, 'requirements-coffee-machine.md'), 'utf8');
  assert.match(table, /at least 1\.8 L of water/);
  assert.match(table, /\]\(\.\.\/physical\.sysml#L\d+\)/);
  // Each requirement in the table opens the line that declares it.
  const sources = fs.readdirSync(mirror)
    .filter((name) => name.endsWith('.sysml'))
    .map((name) => ({ file: name, text: fs.readFileSync(path.join(mirror, name), 'utf8') }));
  const { byId } = requirementLines(sources);
  const ids = [...table.matchAll(/^\| (REQ-[\d.]+) \|/gm)].map((match) => match[1]);
  assert.equal(ids.length, 15);
  for (const id of ids) {
    const at = byId[id];
    assert.ok(at, id);
    assert.ok(fs.readFileSync(path.join(mirror, at.file), 'utf8').split('\n')[at.line - 1].includes(`<'${id}'>`), id);
  }
  fs.rmSync(direct, { recursive: true });
  fs.rmSync(root, { recursive: true });
});
