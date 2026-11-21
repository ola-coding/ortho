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
/** `Wrote <file>.svg (<summary>)`, one line per diagram, or `.md` for a table of requirements. */
const WROTE = /^Wrote (.+?\.(?:svg|md)) \((.*)\)$/;

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
 * A view's title from its file name: `logical-view-survey-drone.svg` is
 * "Logical view survey drone", and `requirements-survey-drone.md` "Requirements
 * survey drone".
 * @param {string} file
 */
function viewTitle(file) {
  const words = path.basename(file).replace(/\.(svg|md)$/i, '').replace(/[-_]+/g, ' ').trim();
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

/**
 * Copies a model's .sysml files, as ortho reads them from its folder, into `root`, at
 * the place the folder has under `cwd`, with the text of each file open in an editor
 * as it stands there, saved or not. Run in `root`, ortho then names each file as it
 * would the file itself from `cwd`, and the model renders as typed.
 * @param {string} folder the model's folder
 * @param {string} cwd the folder ortho would run in, which holds `folder`
 * @param {string} root
 * @param {Map<string, string>} typed the text of each file open in an editor, by its path
 * @returns {string} the copy of the model's folder
 */
function mirrorModel(folder, cwd, root, typed) {
  const mirror = path.join(root, path.relative(cwd, folder));
  fs.mkdirSync(mirror, { recursive: true });
  for (const name of fs.readdirSync(folder)) {
    const file = path.join(folder, name);
    if (!name.endsWith('.sysml') || !fs.statSync(file).isFile()) continue;
    const text = typed.get(file);
    if (text === undefined) fs.copyFileSync(file, path.join(mirror, name));
    else fs.writeFileSync(path.join(mirror, name), text);
  }
  return mirror;
}

/**
 * A model's text with its comments blanked, line ends kept, so that nothing written
 * in a comment is taken for a declaration.
 * @param {string} text
 */
function withoutComments(text) {
  return text.replace(/\/\*[\s\S]*?\*\/|\/\/[^\n]*/g, (comment) => comment.replace(/[^\n]/g, ' '));
}

/**
 * Where each requirement is declared, by its ID, its short name, and by its name for
 * one without an ID, so that a requirement's box in the table can open its line.
 * @param {{ file: string, text: string }[]} sources each model file, named as the preview opens it
 * @returns {{ byId: Record<string, { file: string, line: number }>, byName: Record<string, { file: string, line: number }> }}
 */
function requirementLines(sources) {
  /** @type {Record<string, { file: string, line: number }>} */
  const byId = {};
  /** @type {Record<string, { file: string, line: number }>} */
  const byName = {};
  const name = String.raw`('(?:[^'\\]|\\.)*'|[A-Za-z_][A-Za-z0-9_]*)`;
  const declaration = new RegExp(String.raw`\brequirement\s+(?:<\s*${name}\s*>\s*)?${name}?`, 'g');
  const plain = (/** @type {string} */ written) => written.replace(/^'(.*)'$/s, '$1').replace(/\\(.)/g, '$1');
  for (const { file, text } of sources) {
    const code = withoutComments(text);
    for (const match of code.matchAll(declaration)) {
      // `requirement def` declares a definition, which no table lists.
      if (match[2] === 'def') continue;
      const at = { file, line: code.slice(0, match.index).split('\n').length };
      if (match[1] !== undefined) byId[plain(match[1])] ??= at;
      else if (match[2] !== undefined) byName[plain(match[2])] ??= at;
    }
  }
  return { byId, byName };
}

/**
 * A path as ortho writes it in a link: forward slashes, and spaces and parentheses
 * escaped, so that Markdown reads the link whole.
 * @param {string} relative
 */
function linkPath(relative) {
  return relative.split(path.sep).join('/').replace(/ /g, '%20').replace(/\(/g, '%28').replace(/\)/g, '%29');
}

/**
 * A table of requirements written into `from`, with its links into the model made to
 * work from `to`, as ortho would have written them there. Links within the table, to
 * an anchor, stay. `real` gives the file a link names, for a table written from a copy
 * of the model.
 * @param {string} markdown
 * @param {string} from
 * @param {string} to
 * @param {(file: string) => string} [real]
 */
function relinkMarkdown(markdown, from, to, real = (file) => file) {
  return markdown.replace(/(?<!\\)\]\(([^)\s]+)\)/g, (whole, target) => {
    if (target.startsWith('#') || /^[a-z][a-z0-9+.-]*:/i.test(target)) return whole;
    const hash = target.indexOf('#');
    const file = hash < 0 ? target : target.slice(0, hash);
    const fragment = hash < 0 ? '' : target.slice(hash);
    const named = real(path.resolve(from, decodeURIComponent(file)));
    return `](${linkPath(path.relative(to, named))}${fragment})`;
  });
}

module.exports = {
  mirrorModel,
  requirementLines,
  relinkMarkdown,
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
