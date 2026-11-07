// @ts-check
// The live preview: a panel beside a SysML v2 model showing every view ortho draws from
// it, rendered again on every save, with ortho's problems in the Problems panel.
'use strict';

const vscode = require('vscode');
const crypto = require('node:crypto');
const path = require('node:path');
const { readReport, viewTitle, tabTitles, runOrtho, keepOnly } = require('./ortho');

const INSTALL_URL = 'https://github.com/ola-coding/ortho#install';

/** @param {vscode.ExtensionContext} context */
function activate(context) {
  const problems = vscode.languages.createDiagnosticCollection('ortho');
  const preview = new Preview(context, problems);
  context.subscriptions.push(
    problems,
    preview,
    vscode.commands.registerCommand('ortho.openPreview', (/** @type {vscode.Uri | undefined} */ uri) =>
      preview.open(uri),
    ),
    vscode.commands.registerCommand('ortho.renderAgain', () => preview.render()),
    // A preview open when VS Code closed opens again on the same model.
    vscode.window.registerWebviewPanelSerializer(Preview.viewType, {
      async deserializeWebviewPanel(panel, /** @type {{ folder?: unknown } | undefined} */ state) {
        preview.adopt(panel, typeof state?.folder === 'string' ? state.folder : undefined);
      },
    }),
  );
}

function deactivate() {}

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
    const settings = vscode.workspace.getConfiguration('ortho');
    const ortho = settings.get('path', 'ortho') || 'ortho';
    const theme = settings.get('theme', 'light');
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
      src: webview.asWebviewUri(vscode.Uri.file(path.join(out, path.basename(written.file)))).toString(),
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
