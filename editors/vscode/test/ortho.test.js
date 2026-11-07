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

test('reports an ortho that cannot be found', async () => {
  const run = await runOrtho(path.join(os.tmpdir(), 'no-such-ortho'), '.', os.tmpdir(), 'light');
  assert.equal(run.missing, true);
});

const ortho = process.env.ORTHO_BIN;

test('renders an example with the real ortho', { skip: !ortho && 'set ORTHO_BIN' }, async () => {
  const out = fs.mkdtempSync(path.join(os.tmpdir(), 'ortho-render-'));
  const model = path.join(repository, 'examples', 'coffee-machine');
  const run = await runOrtho(String(ortho), model, out, 'light', repository);
  assert.equal(run.code, 0, run.stderr);
  const report = readReport(run.stdout, run.stderr);
  assert.equal(report.written.length, 6);
  for (const written of report.written) assert.ok(fs.existsSync(written.file), written.file);
  // The diagram names its model file from the folder ortho ran in.
  const logical = report.written.find((written) => written.file.includes('logical'));
  assert.match(fs.readFileSync(String(logical?.file), 'utf8'), /examples\/coffee-machine\/logical\.sysml/);
  assert.deepEqual(report.problems, []);
  fs.rmSync(out, { recursive: true });
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
