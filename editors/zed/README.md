# Ortho for SysML v2, in Zed

**Check a SysML v2 model as you write it, and see its architecture.** Ortho
draws six architectural views of a SysML v2 model, in SysML v2's own
graphical notation: use case, logical, implementation, physical, deployment
and process. This extension brings ortho's checks into Zed, and shows the
diagrams in a live preview in your browser, drawn again every time you save.

It is made for models that AI agents write and people review, so it helps
check and read a model rather than type it.

## Features

- **Problems as you type.** A name that does not resolve, a syntax error, or
  two views that disagree, such as a function nothing performs, is underlined
  in the file and listed in Zed's project diagnostics, for every file of the
  model, not only the one open.
- **Go to definition** (F12, or Cmd-click) from any name to the element it
  names, in whichever file of the model declares it, a segment of a chain
  such as `beverage.prepareCoffee.grindBeans` included.
- **A live preview.** Open a `.sysml` file and the model's diagrams open in
  your browser, a tab for each view. Put the window beside Zed: every save
  draws them again. While the model has a problem the last diagrams stay,
  faded, with the problem above them.
- **From a box to its line.** Click a box in the preview and Zed opens the
  line that declares it. The other way, the code actions on a view's
  declaration (Cmd-. , or Ctrl-. on Linux and Windows) offer *Show in the
  preview*, which turns the page to that view.
- **Nothing written until you ask.** The preview keeps the diagrams to
  itself. *Save all* on the page writes them as SVG into a `diagrams/` folder
  beside the model, exactly as
  `ortho render <model folder> -o <model folder>/diagrams` does, and *Save
  SVG* downloads the one shown.
- **SysML v2 colouring,** the same as in the VS Code extension: keywords, the
  names that definitions and packages declare, comments, strings, numbers and
  metadata such as `#Safety`. Packages, definitions and the main usages are in
  Zed's outline (Cmd-Shift-O).

## Starting a project

ortho itself carries what a project starts with, so it is one command in
Zed's terminal, with any ortho, in any editor:

```sh
ortho init                           # AI instructions, into this folder
ortho init --example coffee-machine  # and an example model, or survey-drone, or web-shop
```

The AI instructions are a skill for Claude Code, instructions for GitHub
Copilot, and an `AGENTS.md`, which Zed's own agent reads, written only if the
project has no such file yet. They carry the SysML v2 that ortho reads, and
tell the agent to render the model after every change and fix what ortho
reports, rather than delete the line it points at. The coffee machine and
the survey drone have all six views; the web shop is software only, with
four.

## Your model

A model is every `.sysml` file in the folder of a file you open, read
together. Each view is declared in the model with SysML v2's standard view
definitions, and every view is optional: a model draws only the views it
declares. [LANGUAGE.md](https://github.com/ola-coding/ortho/blob/main/LANGUAGE.md)
says what to write for each view.

## ortho

The extension runs `ortho lsp`, from ortho 0.4.0 on. It uses the `ortho` on
your PATH, and otherwise downloads the release of ortho for your system from
GitHub, once, and keeps it up to date. To run another one, name it in your
Zed settings:

```json
{
  "lsp": {
    "ortho": {
      "binary": { "path": "/path/to/ortho" }
    }
  }
}
```

## Settings

Under `lsp.ortho.initialization_options`:

- `"preview"`: `false` keeps the preview from opening by itself with the
  first model file. *Show in the preview* still opens it.
- `"diagrams"`: the folder, inside the model's folder, that *Save all* writes
  the diagrams to; `"diagrams"` unless set.
- `"theme"`: `"light"`, the default, or `"dark-glass"`, a transparent version
  for dark slides.

```json
{
  "lsp": {
    "ortho": {
      "initialization_options": { "preview": false, "diagrams": "out" }
    }
  }
}
```

## Beside another SysML v2 extension

The language is named *SysML v2*, as other SysML v2 extensions for Zed name
it, so the two work side by side: Zed runs both language servers on the same
files. To run only ortho's, say so in your settings:
`"languages": { "SysML v2": { "language_servers": ["ortho"] } }`.

Highlighting is from Nomograph's
[tree-sitter-sysml](https://gitlab.com/nomograph/tree-sitter-sysml), MIT.
