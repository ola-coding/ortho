# Ortho for SysML v2, in Zed

**Check a SysML v2 model as you write it, and see its architecture.** Ortho
draws six architectural views of a SysML v2 model, in SysML v2's own
graphical notation: use case, logical, implementation, physical, deployment
and process. This extension brings ortho's checks into Zed, and writes the
diagrams beside the model every time you save.

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
- **The diagrams on every save.** Saving a model file writes its views as SVG
  into a `diagrams/` folder beside the model, exactly as
  `ortho render <model folder> -o <model folder>/diagrams` does, rewriting
  only the diagrams that changed. On a view's declaration, the code actions
  (Cmd-. , or Ctrl-. on Linux and Windows) offer *Show diagram*, which opens
  that view's SVG in Zed. Drag its tab beside the model: Zed shows it again
  each time you save.
- **SysML v2 colouring,** the same as in the VS Code extension: keywords, the
  names that definitions and packages declare, comments, strings, numbers and
  metadata such as `#Safety`. Packages, definitions and the main usages are in
  Zed's outline (Cmd-Shift-O).

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

- `"diagrams"`: the folder, inside the model's folder, that the diagrams are
  written to; `"diagrams"` unless set. `false` writes none, and leaves out
  *Show diagram*.
- `"theme"`: `"light"`, the default, or `"dark-glass"`, a transparent version
  for dark slides.

```json
{
  "lsp": {
    "ortho": {
      "initialization_options": { "diagrams": "out", "theme": "light" }
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
