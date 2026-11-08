# Ortho for SysML v2

A VS Code extension for writing SysML v2 models with
[ortho](https://github.com/ola-coding/ortho), which draws a model's
architectural views as SVG.

It is at an early stage. What it does now:

- **Shows the diagrams beside the model.** Click the preview button at the
  top right of a `.sysml` file, or press Ctrl+K V (Cmd+K V on a Mac). A panel
  opens with a tab for each view the model declares, and draws them again
  every time you save. Click a box to open the line that declares it, beside
  the preview. Click anywhere else in a diagram to switch between fitting the
  panel and actual size.
- **Saves the diagrams as SVG,** for a document, a slide or your repository.
  *Save SVG…* at the top of the preview saves the view shown, and *Save
  all…* every view, into a folder you pick, each named as `ortho render`
  names it, so saving again replaces them. The files are exactly what
  `ortho render` writes. While the model does not load, nothing is saved,
  since the faded diagrams no longer match it.
- **Puts ortho's problems in the Problems panel,** underlined in the file: a
  name that does not resolve, a syntax error, a warning. While the model does
  not load, the last diagrams stay in the panel, faded, until it does.
- **Colours SysML v2** in `.sysml` files: keywords, the names definitions and
  packages declare, comments and notes, strings, quoted names, numbers and
  metadata such as `#Safety`. The keywords are exactly the ones ortho reads.
- **Comments a line** with Ctrl+/ and closes brackets and quotes.
- **Teaches AI agents the model.** *Ortho: Add AI Instructions*, in the
  Command Palette, writes a skill for Claude Code
  (`.claude/skills/sysml-v2-ortho/`) and instructions for GitHub Copilot
  (`.github/instructions/sysml-v2.instructions.md`) into your workspace. They
  carry the SysML v2 that ortho reads, and tell the agent to render the model
  after every change and fix what ortho reports, rather than delete the line
  it points at. It asks before replacing files that are already there.
- **Opens an example.** *Ortho: Open Example* copies the coffee machine or
  the survey drone into a folder you pick and opens its use case view with
  the preview beside it. The copy is yours to change; a folder outside your
  workspace opens in a new window.

A model is every `.sysml` file in the folder of the file you preview, read
together, as `ortho render <folder>` reads them. The diagrams are written to
VS Code's storage for the extension, never into your workspace, until you
save them.

The extension carries ortho, built for your system, so there is nothing else
to install. It runs that one, and puts it first on the PATH of VS Code's
terminals, so an agent working there checks the model with the same ortho as
the preview. Its version is the extension's. To run another ortho, set
**Ortho: Path** to the command or the binary. In a folder you have not told
VS Code to trust, the folder's own **Ortho: Path** is ignored, so a
repository you cloned cannot choose a program for the extension to run.

| Setting | Default | What it does |
| --- | --- | --- |
| `ortho.path` | empty | The ortho to run, as a command or a path. Empty runs the one the extension carries. |
| `ortho.theme` | `light` | `light` for documents, or `dark-glass` for slides. |

What comes next is in the
[backlog](https://github.com/ola-coding/ortho/blob/main/DEVELOPMENT.md#backlog):
the Marketplace.

It targets SysML v2 only. It uses the language id `sysml`, so it works beside
other SysML v2 extensions, such as SysIDE.

## Trying it

It is not on the Marketplace yet. To run it from a clone of the repository,
open the repository, or `editors/vscode`, in VS Code and press F5. A second
window opens on the coffee machine example with the extension loaded, and
picks up your changes when you reload it. Run this way, it carries no ortho
and runs `ortho` from the PATH: install it with `cargo install ortho`, or set
**Ortho: Path** to the one `cargo build` writes, `target/debug/ortho`.

## Working on it

The code is plain JavaScript, with no dependencies and no build step:

- `src/extension.js`: the commands, the preview panel and the Problems.
- `src/ortho.js`: runs `ortho render` and reads what it prints. It needs no
  VS Code, so `npm test` tests it with Node; set `ORTHO_BIN` to an ortho
  binary to run it on the examples too, as CI does.
- `src/instructions.js` and `src/examples.js`: the AI instructions and the
  examples offered, tested with Node too.
- `media/`: the preview panel's page.

The grammar, `syntaxes/sysml.tmLanguage.json`, is written from ortho's
keyword list in `src/parser/lexer.rs` by `cargo xtask vscode-grammar`. Change
`xtask/src/vscode_grammar.rs` and run that, rather than editing the JSON; CI
fails if the two differ. The same task copies ortho's LANGUAGE.md to
`ai/LANGUAGE.md`, which the AI instructions carry, so they never drift from
what ortho reads, each example's `.sysml` files to `examples/`, which
*Open Example* copies, and the repository's LICENSE and icon, since the
package holds only what is in this folder. CHANGELOG.md is written by hand,
for the Marketplace's Changelog tab.
