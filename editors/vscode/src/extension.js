// @ts-check
// The live preview: a panel beside a SysML v2 model showing every view ortho draws from
// it, rendered again on every save, with ortho's problems in the Problems panel.
'use strict';

const vscode = require('vscode');
const crypto = require('node:crypto');
const path = require('node:path');
const {
  readReport,
  viewTitle,
  tabTitles,
  runOrtho,
  keepOnly,
  chooseOrtho,
  orthoFolder,
  markedSource,
} = require('./ortho');
const fs = require('node:fs');
const { instructionFiles } = require('./instructions');
const { EXAMPLES, freeName, firstFile } = require('./examples');

const INSTALL_URL = 'https://github.com/ola-coding/ortho#install';
/** The file of an example just copied into a folder another window opens, to open there. */
const OPEN_ON_START = 'ortho.openOnStart';

/** @param {vscode.ExtensionContext} context */
function activate(context) {
  putOnTerminalPath(context);
  const problems = vscode.languages.createDiagnosticCollection('ortho');
  const preview = new Preview(context, problems);
  context.subscriptions.push(
    problems,
    preview,
    vscode.commands.registerCommand('ortho.openPreview', (/** @type {vscode.Uri | undefined} */ uri) =>
      preview.open(uri),
    ),
    vscode.commands.registerCommand('ortho.renderAgain', () => preview.render()),
    vscode.commands.registerCommand('ortho.addAiInstructions', () => addAiInstructions(context)),
    vscode.commands.registerCommand('ortho.openExample', () => openExample(context)),
    vscode.workspace.onDidChangeConfiguration((event) => {
      if (event.affectsConfiguration('ortho.path')) putOnTerminalPath(context);
    }),
    // A preview open when VS Code closed opens again on the same model.
    vscode.window.registerWebviewPanelSerializer(Preview.viewType, {
      async deserializeWebviewPanel(panel, /** @type {{ folder?: unknown } | undefined} */ state) {
        preview.adopt(panel, typeof state?.folder === 'string' ? state.folder : undefined);
      },
    }),
  );
  openCopiedExample(context);
}

function deactivate() {}

/** The ortho the preview runs: the one `ortho.path` names, else `ortho` on the PATH. */
function orthoOf() {
  return chooseOrtho(vscode.workspace.getConfiguration('ortho').get('path', ''));
}

/**
 * Puts the ortho the preview runs first on the PATH of VS Code's terminals, where
 * agents run their commands, so that they check their work with the same ortho.
 * @param {vscode.ExtensionContext} context
 */
function putOnTerminalPath(context) {
  const terminals = context.environmentVariableCollection;
  terminals.clear();
  const folder = orthoFolder(orthoOf());
  if (!folder) return;
  terminals.description = 'Puts the ortho that the Ortho extension runs on the PATH.';
  terminals.prepend('PATH', folder + path.delimiter);
}

/**
 * Copies an example the extension carries into a folder the user picks, then opens its
 * use case view with the preview beside it. A copy outside the open workspace opens as
 * a folder of its own, where the extension, starting there, opens the file.
 * @param {vscode.ExtensionContext} context
 */
async function openExample(context) {
  const picked = await vscode.window.showQuickPick(
    EXAMPLES.map((example) => ({ label: example.label, detail: example.detail, example })),
    { placeHolder: 'Which example should be copied and opened?' },
  );
  if (!picked) return;
  const { example } = picked;
  const parents = await vscode.window.showOpenDialog({
    canSelectFiles: false,
    canSelectFolders: true,
    canSelectMany: false,
    openLabel: 'Copy Here',
    title: `Where should the ${example.label.toLowerCase()} example go?`,
    defaultUri: vscode.workspace.workspaceFolders?.[0]?.uri,
  });
  if (!parents?.[0]) return;
  const parent = parents[0];
  const taken = new Set((await vscode.workspace.fs.readDirectory(parent)).map(([name]) => name));
  const target = vscode.Uri.joinPath(parent, freeName(example.name, (name) => taken.has(name)));
  const source = vscode.Uri.joinPath(context.extensionUri, 'examples', example.name);
  const files = (await vscode.workspace.fs.readDirectory(source)).map(([name]) => name);
  await vscode.workspace.fs.createDirectory(target);
  for (const file of files) {
    await vscode.workspace.fs.copy(vscode.Uri.joinPath(source, file), vscode.Uri.joinPath(target, file));
  }
  const first = vscode.Uri.joinPath(target, firstFile(files));
  if (vscode.workspace.getWorkspaceFolder(first)) {
    await showWithPreview(first);
    return;
  }
  await context.globalState.update(OPEN_ON_START, first.toString());
  const open = vscode.workspace.workspaceFolders?.length ? { forceNewWindow: true } : {};
  await vscode.commands.executeCommand('vscode.openFolder', target, open);
}

/**
 * Opens the example the last Open example copied, once the window that opens its
 * folder starts. Any other window leaves it to that one.
 * @param {vscode.ExtensionContext} context
 */
async function openCopiedExample(context) {
  const copied = context.globalState.get(OPEN_ON_START);
  if (typeof copied !== 'string') return;
  const first = vscode.Uri.parse(copied);
  if (!vscode.workspace.getWorkspaceFolder(first)) return;
  await context.globalState.update(OPEN_ON_START, undefined);
  await showWithPreview(first);
}

/** @param {vscode.Uri} uri */
async function showWithPreview(uri) {
  await vscode.window.showTextDocument(uri);
  await vscode.commands.executeCommand('ortho.openPreview', uri);
}

/**
 * Writes the AI instructions into a workspace folder: a skill for Claude Code and
 * instructions for Copilot, from the copy of ortho's LANGUAGE.md the extension carries.
 * Files that are already there are replaced only once the user says so.
 * @param {vscode.ExtensionContext} context
 */
async function addAiInstructions(context) {
  const folders = vscode.workspace.workspaceFolders ?? [];
  const folder =
    folders.length > 1
      ? await vscode.window.showWorkspaceFolderPick({ placeHolder: 'Where should the AI instructions go?' })
      : folders[0];
  if (!folder) {
    if (folders.length === 0) vscode.window.showInformationMessage('Open a folder to add AI instructions to.');
    return;
  }
  const bytes = await vscode.workspace.fs.readFile(vscode.Uri.joinPath(context.extensionUri, 'ai', 'LANGUAGE.md'));
  const files = instructionFiles(new TextDecoder().decode(bytes));
  const existing = [];
  for (const file of files) {
    const uri = vscode.Uri.joinPath(folder.uri, file.path);
    if (await exists(uri)) existing.push(file.path);
  }
  if (existing.length > 0) {
    const replace = 'Replace';
    const answer = await vscode.window.showWarningMessage(
      `Replace the AI instructions already in ${folder.name}?`,
      { modal: true, detail: existing.join('\n') },
      replace,
    );
    if (answer !== replace) return;
  }
  for (const file of files) {
    const uri = vscode.Uri.joinPath(folder.uri, file.path);
    await vscode.workspace.fs.createDirectory(vscode.Uri.joinPath(uri, '..'));
    await vscode.workspace.fs.writeFile(uri, new TextEncoder().encode(file.content));
  }
  const open = 'Open the skill';
  const answer = await vscode.window.showInformationMessage(
    `Added AI instructions for Claude Code and Copilot to ${folder.name}.`,
    open,
  );
  if (answer === open) {
    await vscode.window.showTextDocument(vscode.Uri.joinPath(folder.uri, files[0].path));
  }
}

/** @param {vscode.Uri} uri */
async function exists(uri) {
  try {
    await vscode.workspace.fs.stat(uri);
    return true;
  } catch {
    return false;
  }
}

/** A diagram ortho wrote, as text, to show inline where a click on a box can be heard. */
function readSvg(/** @type {string} */ file) {
  try {
    return fs.readFileSync(file, 'utf8');
  } catch {
    return '';
  }
}

/** The model a file belongs to: every .sysml file in its folder, read as one model. */
function modelOf(/** @type {vscode.Uri} */ uri) {
  return path.dirname(uri.fsPath);
}

class Preview {
  static viewType = 'ortho.preview';

  /**
   * @param {vscode.ExtensionContext} context
   * @param {vscode.DiagnosticCollection} problems
   */
  constructor(context, problems) {
    this.context = context;
    this.problems = problems;
    /** @type {vscode.WebviewPanel | undefined} */
    this.panel = undefined;
    /** The folder whose model is shown. */
    /** @type {string | undefined} */
    this.folder = undefined;
    /** Where ortho ran for the model shown, which the boxes name their files from. */
    /** @type {string | undefined} */
    this.cwd = undefined;
    this.rendering = false;
    this.again = false;
    /** @type {Map<string, string>} each model's render whose diagrams are on screen */
    this.shown = new Map();
    /** @type {vscode.Uri[]} the files that have problems now, to clear on the next render */
    this.reported = [];
    /** @type {vscode.Disposable[]} */
    this.disposables = [
      vscode.workspace.onDidSaveTextDocument((document) => {
        if (this.panel && this.isModelFile(document.uri)) this.render();
      }),
      vscode.window.onDidChangeActiveTextEditor((editor) => {
        // The preview follows the model being edited.
        if (!this.panel || !editor || editor.document.languageId !== 'sysml') return;
        const folder = modelOf(editor.document.uri);
        if (folder !== this.folder) this.show(folder);
      }),
      vscode.workspace.onDidChangeConfiguration((event) => {
        if (this.panel && event.affectsConfiguration('ortho')) this.render();
      }),
    ];
  }

  /** @param {vscode.Uri} uri */
  isModelFile(uri) {
    return uri.scheme === 'file' && uri.fsPath.endsWith('.sysml') && modelOf(uri) === this.folder;
  }

  /** Opens the preview beside the editor, on the given file's model or the active one's. */
  open(/** @type {vscode.Uri | undefined} */ uri) {
    const target = uri ?? vscode.window.activeTextEditor?.document.uri;
    if (!target || !target.fsPath.endsWith('.sysml')) {
      vscode.window.showInformationMessage('Open a .sysml file to preview its views.');
      return;
    }
    if (this.panel) {
      this.panel.reveal(vscode.ViewColumn.Beside, true);
    } else {
      const panel = vscode.window.createWebviewPanel(
        Preview.viewType,
        'Ortho preview',
        { viewColumn: vscode.ViewColumn.Beside, preserveFocus: true },
        {},
      );
      this.adopt(panel, undefined);
    }
    this.show(modelOf(target));
  }

  /**
   * Takes over a panel, new or restored after VS Code restarts.
   * @param {vscode.WebviewPanel} panel
   * @param {string | undefined} folder
   */
  adopt(panel, folder) {
    this.panel = panel;
    panel.webview.options = {
      enableScripts: true,
      localResourceRoots: [this.outRoot(), vscode.Uri.joinPath(this.context.extensionUri, 'media')],
    };
    panel.webview.html = this.page(panel.webview);
    panel.onDidDispose(() => {
      this.panel = undefined;
      this.folder = undefined;
      this.clearProblems();
    });
    panel.webview.onDidReceiveMessage((message) => {
      if (message?.type === 'renderAgain') this.render();
      if (message?.type === 'open') this.openSource(message.file, message.line);
      if (message?.type === 'install') vscode.env.openExternal(vscode.Uri.parse(INSTALL_URL));
      if (message?.type === 'settings') vscode.commands.executeCommand('workbench.action.openSettings', 'ortho.path');
    });
    if (folder) this.show(folder);
  }

  /** @param {string} folder */
  show(folder) {
    this.folder = folder;
    if (this.panel) this.panel.title = `Ortho: ${path.basename(folder)}`;
    this.clearProblems();
    this.render();
  }

  /**
   * Opens the line a clicked box marks, in the editor beside the preview: the one
   * showing the model, else the first column.
   * @param {unknown} file
   * @param {unknown} line
   */
  async openSource(file, line) {
    if (!this.folder || !this.cwd) return;
    const source = markedSource(this.cwd, this.folder, file, line);
    if (!source) return;
    const column =
      vscode.window.visibleTextEditors.find((editor) => editor.document.languageId === 'sysml')?.viewColumn ??
      vscode.ViewColumn.One;
    const editor = await vscode.window.showTextDocument(vscode.Uri.file(source.file), { viewColumn: column });
    const text = editor.document.lineAt(Math.min(source.line - 1, editor.document.lineCount - 1));
    const start = new vscode.Position(text.lineNumber, text.firstNonWhitespaceCharacterIndex);
    editor.selection = new vscode.Selection(start, text.range.end);
    editor.revealRange(text.range, vscode.TextEditorRevealType.InCenterIfOutsideViewport);
  }

  outRoot() {
    return vscode.Uri.joinPath(this.context.globalStorageUri, 'diagrams');
  }

  /** Where one model's diagrams are written: never into the user's workspace. */
  outFolder(/** @type {string} */ folder) {
    const key = crypto.createHash('sha256').update(folder).digest('hex').slice(0, 16);
    return vscode.Uri.joinPath(this.outRoot(), key).fsPath;
  }

  /** Renders the model shown, once more after the current render if a save came in. */
  async render() {
    if (!this.panel || !this.folder) return;
    if (this.rendering) {
      this.again = true;
      return;
    }
    this.rendering = true;
    try {
      do {
        this.again = false;
        await this.renderOnce(this.folder);
      } while (this.again && this.panel && this.folder);
    } finally {
      this.rendering = false;
    }
  }

  /** @param {string} folder */
  async renderOnce(folder) {
    const ortho = orthoOf();
    const theme = vscode.workspace.getConfiguration('ortho').get('theme', 'light');
    // Each render gets a folder of its own, so the diagrams of the last render that
    // worked stay on screen while the model does not load.
    const base = this.outFolder(folder);
    const stamp = `${Date.now()}`;
    const out = path.join(base, stamp);
    this.post({ type: 'busy' });
    // Each diagram names its model file relative to where ortho runs: from the
    // workspace folder, as `examples/survey-drone/logical.sysml`.
    const cwd = vscode.workspace.getWorkspaceFolder(vscode.Uri.file(folder))?.uri.fsPath ?? folder;
    const run = await runOrtho(ortho, folder, out, theme, cwd);
    // The boxes name their files from where ortho ran.
    if (folder === this.folder) this.cwd = cwd;
    const report = readReport(run.stdout, run.stderr);
    if (report.written.length > 0) this.shown.set(base, stamp);
    keepOnly(base, this.shown.get(base) ?? '');
    if (!this.panel || folder !== this.folder) return;
    if (run.missing) {
      this.post({ type: 'missing', ortho });
      return;
    }
    this.setProblems(folder, report.problems);
    const webview = this.panel.webview;
    const titles = tabTitles(report.written.map((written) => written.file));
    const views = report.written.map((written, index) => ({
      title: titles[index],
      heading: viewTitle(written.file),
      summary: written.summary,
      svg: readSvg(path.join(out, path.basename(written.file))),
    }));
    const failed = run.code !== 0;
    const errors = report.problems.filter((problem) => problem.severity === 'error');
    this.post({
      type: 'rendered',
      model: path.basename(folder),
      folder,
      views,
      failed,
      errors: errors.map((problem) => `${path.basename(problem.file)} line ${problem.line}: ${problem.message}`),
      notes: report.notes,
      theme,
    });
  }

  /**
   * @param {string} folder
   * @param {import('./ortho').Problem[]} found
   */
  setProblems(folder, found) {
    this.clearProblems();
    /** @type {Map<string, vscode.Diagnostic[]>} */
    const byFile = new Map();
    for (const problem of found) {
      const file = path.resolve(folder, problem.file);
      const line = Math.max(0, problem.line - 1);
      const text = vscode.workspace.textDocuments.find((document) => document.uri.fsPath === file);
      // ortho reports a line, not a column: the line's text is marked, from its indent.
      const lineText = text && line < text.lineCount ? text.lineAt(line) : undefined;
      const range = lineText
        ? new vscode.Range(line, lineText.firstNonWhitespaceCharacterIndex, line, lineText.range.end.character)
        : new vscode.Range(line, 0, line, 1000);
      const severity =
        problem.severity === 'error' ? vscode.DiagnosticSeverity.Error : vscode.DiagnosticSeverity.Warning;
      const diagnostic = new vscode.Diagnostic(range, problem.message, severity);
      diagnostic.source = 'ortho';
      byFile.set(file, [...(byFile.get(file) ?? []), diagnostic]);
    }
    for (const [file, diagnostics] of byFile) {
      const uri = vscode.Uri.file(file);
      this.problems.set(uri, diagnostics);
      this.reported.push(uri);
    }
  }

  clearProblems() {
    for (const uri of this.reported) this.problems.delete(uri);
    this.reported = [];
  }

  /** @param {object} message */
  post(message) {
    this.panel?.webview.postMessage(message);
  }

  /** @param {vscode.Webview} webview */
  page(webview) {
    const nonce = crypto.randomBytes(16).toString('base64');
    const media = (/** @type {string} */ name) =>
      webview.asWebviewUri(vscode.Uri.joinPath(this.context.extensionUri, 'media', name));
    return `<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="UTF-8">
<meta http-equiv="Content-Security-Policy" content="default-src 'none'; img-src ${webview.cspSource}; style-src ${webview.cspSource}; script-src 'nonce-${nonce}';">
<meta name="viewport" content="width=device-width, initial-scale=1.0">
<link rel="stylesheet" href="${media('preview.css')}">
<title>Ortho preview</title>
</head>
<body>
<header>
  <nav id="tabs" role="tablist"></nav>
  <span id="status" role="status"></span>
</header>
<section id="messages"></section>
<main id="diagram"></main>
<script nonce="${nonce}" src="${media('preview.js')}"></script>
</body>
</html>`;
  }

  dispose() {
    this.panel?.dispose();
    for (const disposable of this.disposables) disposable.dispose();
  }
}

module.exports = { activate, deactivate };
