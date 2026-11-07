# Ortho for SysML v2

A VS Code extension for writing SysML v2 models with
[ortho](https://github.com/ola-coding/ortho), which draws a model's
architectural views as SVG.

It is at an early stage. What it does now:

- **Shows the diagrams beside the model.** Click the preview button at the
  top right of a `.sysml` file, or press Ctrl+K V (Cmd+K V on a Mac). A panel
  opens with a tab for each view the model declares, and draws them again
  every time you save. Click a diagram to switch between fitting the panel
  and actual size.
- **Puts ortho's problems in the Problems panel,** underlined in the file: a
  name that does not resolve, a syntax error, a warning. While the model does
  not load, the last diagrams stay in the panel, faded, until it does.
- **Colours SysML v2** in `.sysml` files: keywords, the names definitions and
  packages declare, comments and notes, strings, quoted names, numbers and
  metadata such as `#Safety`. The keywords are exactly the ones ortho reads.
- **Comments a line** with Ctrl+/ and closes brackets and quotes.

A model is every `.sysml` file in the folder of the file you preview, read
together, as `ortho render <folder>` reads them. The diagrams are written to
VS Code's storage for the extension, never into your workspace.

The preview needs ortho installed: `cargo install ortho`, or a download from
the [releases](https://github.com/ola-coding/ortho/releases). If VS Code
cannot find it, set **Ortho: Path** to the binary.

| Setting | Default | What it does |
| --- | --- | --- |
| `ortho.path` | `ortho` | The ortho command, or the full path to the binary. |
| `ortho.theme` | `light` | `light` for documents, or `dark-glass` for slides. |

What comes next is in the
[backlog](https://github.com/ola-coding/ortho/blob/main/DEVELOPMENT.md#backlog):
the examples, AI instructions, and ortho itself inside the extension.

It targets SysML v2 only. It uses the language id `sysml`, so it works beside
other SysML v2 extensions, such as SysIDE.

## Trying it

It is not on the Marketplace yet. Install ortho first (`cargo install ortho`,
or a download from the
[releases](https://github.com/ola-coding/ortho/releases)), then either:

- **Install the latest build.** Every CI run packages the extension. Open the
  latest run of the
  [CI workflow](https://github.com/ola-coding/ortho/actions/workflows/ci.yml)
  on the branch you want, download **ortho-vscode** under Artifacts and unzip
  it. In VS Code, open the Extensions view, click **...** at its top, choose
  **Install from VSIX...** and pick `ortho-vscode.vsix`. Or from a terminal:
  `code --install-extension ortho-vscode.vsix`.
- **Or run it from a clone** of the repository: open the repository, or
  `editors/vscode`, in VS Code and press F5. A second window opens on the
  coffee machine example with the extension loaded, and picks up your changes
  when you reload it.

## Working on it

The code is plain JavaScript, with no dependencies and no build step:

- `src/extension.js`: the commands, the preview panel and the Problems.
- `src/ortho.js`: runs `ortho render` and reads what it prints. It needs no
  VS Code, so `npm test` tests it with Node; set `ORTHO_BIN` to an ortho
  binary to run it on the examples too, as CI does.
- `media/`: the preview panel's page.

The grammar, `syntaxes/sysml.tmLanguage.json`, is written from ortho's
keyword list in `src/parser/lexer.rs` by `cargo xtask vscode-grammar`. Change
`xtask/src/vscode_grammar.rs` and run that, rather than editing the JSON; CI
fails if the two differ.
