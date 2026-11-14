# Ortho for SysML v2, in Zed

**Check a SysML v2 model as you write it, and see its architecture.** Ortho
draws six architectural views of a SysML v2 model, in SysML v2's own
graphical notation: use case, logical, implementation, physical, deployment
and process. This extension brings ortho's checks into Zed, and shows the
diagrams in your browser, drawn again every time you save.

It is made for models that AI agents write and people review.

## Features

- **SysML v2 coloring,** as in the VS Code extension: keywords, the names that
  definitions and packages declare, comments, strings, numbers and metadata
  such as `#Safety`. Packages, definitions and the main usages are in Zed's
  outline (Ctrl-Shift-O, or Cmd-Shift-O on a Mac). The highlighting is from
  Nomograph's [tree-sitter-sysml](https://gitlab.com/nomograph/tree-sitter-sysml), MIT.
- **A live preview.** Open a `.sysml` file and the model's diagrams open in
  your browser, a tab for each view. Open or edit a file of another model and
  the page turns to it. Close the page, and the next model file you open
  opens it again. While the model has a problem the last diagrams stay,
  faded, with the problem above them.
- **From a box to its line, and back.** Click a box on the page and Zed opens
  the line that declares it. On a view's declaration, the code action *Show
  in the preview* (Ctrl-., or Cmd-. on a Mac) turns the page to that view.
- **Problems as you type,** in every file of the model, not only the one
  open: a name that does not resolve, a syntax error, or two views that
  disagree, such as a function nothing performs. Zed's project diagnostics
  list them all.
- **Go to definition** (F12) from any name, in whichever file declares it,
  each segment of a chain such as `beverage.prepareCoffee.grindBeans`
  included.
- **Nothing written until you ask.** *Save all* on the page writes the SVGs
  into a `diagrams/` folder beside the model, exactly as `ortho render` does,
  and *Save SVG* downloads the one shown.

## Starting a project

In Zed's terminal:

```sh
ortho init                           # AI instructions, into this folder
ortho init --example coffee-machine  # and an example model, or survey-drone, or web-shop
```

The AI instructions are a skill for Claude Code, instructions for GitHub
Copilot and, unless the project has an agent file already, an `AGENTS.md`,
which Zed's agent reads. They carry the SysML v2 ortho reads, and tell the
agent to render after every change and fix what ortho reports.

## Your model

A model is every `.sysml` file in the folder of a file you open, read
together. It draws only the views it declares, with SysML v2's standard view
definitions: [LANGUAGE.md](https://github.com/ola-coding/ortho/blob/main/LANGUAGE.md)
says what to write for each.

## Settings

The extension runs `ortho lsp`, from ortho 0.4.0 on: the `ortho` on your
PATH, or else the release for your system, which it downloads from GitHub
once and keeps up to date. Under `lsp.ortho` in Zed's settings:

- `binary.path`: another ortho to run.
- `initialization_options.preview`: `false` keeps the page from opening by
  itself; *Show in the preview* still opens it.
- `initialization_options.diagrams`: the folder, inside the model's, that
  *Save all* writes to; `"diagrams"` unless set.
- `initialization_options.theme`: `"light"`, the default, or `"dark-glass"`,
  a transparent version for dark slides.

```json
{
  "lsp": {
    "ortho": {
      "binary": { "path": "/path/to/ortho" },
      "initialization_options": { "preview": false, "diagrams": "out" }
    }
  }
}
```
