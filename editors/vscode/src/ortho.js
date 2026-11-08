// @ts-check
// Running `ortho render` and reading what it prints. Nothing here needs VS Code, so the
// tests run it with plain Node.
'use strict';

const { execFile } = require('node:child_process');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');

/**
 * @typedef {{ file: string, line: number, severity: 'error' | 'warning', message: string }} Problem
 * @typedef {{ file: string, summary: string }} Written
 * @typedef {{ written: Written[], problems: Problem[], notes: string[] }} Report
 * @typedef {{ code: number | null, stdout: string, stderr: string, missing: boolean }} Run
 */

/** `<file>.sysml line <n>: <message>`, as ortho reports a problem in a model file. */
const AT_LINE = /^(.+?\.sysml) line (\d+): (.*)$/;
/** `Wrote <file>.svg (<summary>)`, one line per diagram. */
const WROTE = /^Wrote (.+?\.svg) \((.*)\)$/;

/**
 * What ortho printed: the diagrams it wrote, the problems it found at a line of a model
 * file, and everything else worth showing, such as a view it skipped.
 * @param {string} stdout
 * @param {string} stderr
 * @returns {Report}
 */
function readReport(stdout, stderr) {
  /** @type {Report} */
  const report = { written: [], problems: [], notes: [] };
  for (const line of stdout.split(/\r?\n/)) {
    const wrote = WROTE.exec(line);
    if (wrote) report.written.push({ file: wrote[1], summary: wrote[2] });
  }
  for (const raw of stderr.split(/\r?\n/)) {
    if (!raw.trim() || raw === 'Failed to parse:') continue;
    // A model that does not load lists each problem indented under "Failed to parse:".
    const warning = raw.startsWith('Warning: ');
    const line = warning ? raw.slice('Warning: '.length) : raw.trim();
    const at = AT_LINE.exec(line);
    if (at) {
      report.problems.push({
        file: at[1],
        line: Number(at[2]),
        severity: warning ? 'warning' : 'error',
        message: at[3],
      });
    } else {
      report.notes.push(raw.trim());
    }
  }
  return report;
}

/**
 * A diagram's title from its file name: `logical-view-survey-drone.svg` is
 * "Logical view survey drone".
 * @param {string} file
 */
function viewTitle(file) {
  const words = path.basename(file).replace(/\.svg$/i, '').replace(/[-_]+/g, ' ').trim();
  return words ? words[0].toUpperCase() + words.slice(1) : 'View';
}

/**
 * Short titles for tabs: the words every title ends with are left out, so the survey
 * drone's six views are "Deployment", "Implementation" and so on, not each
 * "... view survey drone". A single view keeps its whole title.
 * @param {string[]} files
 */
function tabTitles(files) {
  const titles = files.map(viewTitle);
  if (titles.length < 2) return titles;
  const split = titles.map((title) => title.split(' '));
  let common = 0;
  while (
    split.every((words) => common < words.length - 1) &&
    split.every((words) => words.at(-1 - common)?.toLowerCase() === split[0].at(-1 - common)?.toLowerCase())
  ) {
    common += 1;
  }
  return split.map((words) => words.slice(0, words.length - common).join(' '));
}

/**
 * Where cargo installs ortho, for when VS Code was started without it on its PATH, as
 * from the macOS Dock.
 */
function cargoOrtho() {
  return path.join(os.homedir(), '.cargo', 'bin', process.platform === 'win32' ? 'ortho.exe' : 'ortho');
}

/**
 * The ortho the extension carries in its `bin` folder: each package the Marketplace
 * gives out is for one system, with the ortho built for it. One run with F5 from a
 * clone carries none.
 * @param {string} extensionPath
 * @param {string} [platform]
 */
function bundledOrtho(extensionPath, platform = process.platform) {
  return path.join(extensionPath, 'bin', platform === 'win32' ? 'ortho.exe' : 'ortho');
}

/**
 * Which ortho to run: the one the `ortho.path` setting names, else the one the
 * extension carries, else `ortho` on the PATH.
 * @param {string | undefined} setting
 * @param {string} bundled
 * @param {(file: string) => boolean} [exists]
 */
function chooseOrtho(setting, bundled, exists = fs.existsSync) {
  const named = setting?.trim();
  if (named) return named;
  return exists(bundled) ? bundled : 'ortho';
}

/**
 * The folder to put first on a terminal's PATH, so that `ortho` there is the ortho the
 * preview runs: that binary's folder, or none for a bare command, already looked up on
 * the PATH.
 * @param {string} ortho
 */
function orthoFolder(ortho) {
  return path.isAbsolute(ortho) && /^ortho(\.exe)?$/i.test(path.basename(ortho)) ? path.dirname(ortho) : undefined;
}

/**
 * Makes a binary runnable, in case unpacking the extension lost its mode.
 * @param {string} file
 */
function makeExecutable(file) {
  if (process.platform === 'win32' || !fs.existsSync(file)) return;
  const mode = fs.statSync(file).mode;
  if ((mode & 0o111) !== 0o111) fs.chmodSync(file, mode | 0o755);
}

/**
 * Runs `ortho render <model> -o <out> --theme <theme>` in `cwd`, the folder a diagram
 * names its model file relative to. Never rejects: a model that does not load is a
 * result, and so is an ortho that cannot be found.
 * @param {string} ortho the command, or a path to the binary
 * @param {string} model a .sysml file or a folder of them
 * @param {string} out
 * @param {string} theme
 * @param {string} [cwd]
 * @returns {Promise<Run>}
 */
function runOrtho(ortho, model, out, theme, cwd) {
  const args = ['render', model, '-o', out, '--theme', theme];
  /** @param {string} command @returns {Promise<Run>} */
  const run = (command) =>
    new Promise((resolve) => {
      execFile(command, args, { cwd, windowsHide: true, maxBuffer: 16 * 1024 * 1024 }, (error, stdout, stderr) => {
        const failure = /** @type {(NodeJS.ErrnoException & { code?: unknown }) | null} */ (error);
        const missing = failure?.code === 'ENOENT';
        const code = failure ? (typeof failure.code === 'number' ? failure.code : missing ? null : 1) : 0;
        resolve({ code, stdout: String(stdout), stderr: String(stderr), missing });
      });
    });
  return run(ortho).then((result) =>
    result.missing && ortho === 'ortho' && fs.existsSync(cargoOrtho()) ? run(cargoOrtho()) : result,
  );
}

/**
 * The model file a box in a diagram marks, `data-file` and `data-line`, as a path and
 * a line, 1-based. ortho names the file from where it ran, `cwd`. A mark naming
 * anything but a .sysml file of the model shown is not followed.
 * @param {string} cwd
 * @param {string} folder the model's folder
 * @param {unknown} file
 * @param {unknown} line
 * @returns {{ file: string, line: number } | undefined}
 */
function markedSource(cwd, folder, file, line) {
  if (typeof file !== 'string' || !file.endsWith('.sysml')) return undefined;
  const resolved = path.resolve(cwd, file);
  if (path.relative(folder, resolved) !== path.basename(resolved)) return undefined;
  const number = Number(line);
  return Number.isInteger(number) && number >= 1 ? { file: resolved, line: number } : undefined;
}

/**
 * Removes every render in `base` but `keep`, so a model's diagrams stay only while
 * they are the latest that rendered.
 * @param {string} base
 * @param {string} keep
 */
function keepOnly(base, keep) {
  for (const name of fs.existsSync(base) ? fs.readdirSync(base) : []) {
    if (name !== keep) fs.rmSync(path.join(base, name), { recursive: true, force: true });
  }
}

module.exports = {
  readReport,
  viewTitle,
  tabTitles,
  runOrtho,
  keepOnly,
  cargoOrtho,
  bundledOrtho,
  chooseOrtho,
  orthoFolder,
  makeExecutable,
  markedSource,
};
