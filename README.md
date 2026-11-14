<h1>
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="logo/ortho-logo-dark.svg">
    <img src="logo/ortho-logo.svg" alt="ortho" width="320">
  </picture>
</h1>

**Architecture diagrams from SysML v2 text.** Write your model in the SysML v2
textual notation, commit it next to your code, and `ortho` draws six
architectural views of it as SVG, in SysML v2's own graphical notation. Just
as a technical drawing shows one object through several standard views, so
the diagrams show one model.

<p align="center">
  <img src="examples/coffee-machine/diagrams/physical-view-coffee-machine.svg"
       alt="ortho's physical view of a coffee machine: parts with ports, wired together, each listing the functions it performs"
       width="820">
</p>

## Why ortho

- **The model is the source, the diagrams follow.** Every view is drawn from
  the text, so a diagram cannot drift from the model, and a review shows the
  change to both.
- **Standard SysML v2, in and out.** ortho reads OMG SysML v2.0's textual
  notation, loading all 252 of OMG's example, training and validation models,
  and draws in its graphical notation (§8.2.3). Views are declared with the
  standard view definitions, so another SysML v2 tool reads the same model.
- **Written by AI agents, reviewed by people.** `ortho init` tells an agent
  the SysML v2 ortho reads and has it check its work with ortho; you review
  the text and the diagrams.
- **One program, nothing else.** No modelling tool, no licence server, no
  Java and no network calls. The same model gives the same SVG on a laptop
  and in CI.
- **Made for paper and slides.** Diagrams are sized for landscape A4, with a
  dark theme for slides.

## The six views

The [coffee machine example](examples/coffee-machine/), one model in six
views. Click a diagram to see it full size.

<table>
  <tr>
    <td align="center" valign="top"><a href="VIEWS.md#use-case-view"><b>Use case</b></a><br>
      <a href="examples/coffee-machine/diagrams/use-case-view-coffee-machine.svg"><img src="examples/coffee-machine/diagrams/use-case-view-coffee-machine.svg" alt="Use case view of the coffee machine" width="420"></a></td>
    <td align="center" valign="top"><a href="VIEWS.md#logical-view"><b>Logical</b></a><br>
      <a href="examples/coffee-machine/diagrams/logical-view-coffee-machine.svg"><img src="examples/coffee-machine/diagrams/logical-view-coffee-machine.svg" alt="Logical view of the coffee machine" width="420"></a></td>
  </tr>
  <tr>
    <td align="center" valign="top"><a href="VIEWS.md#implementation-view"><b>Implementation</b></a><br>
      <a href="examples/coffee-machine/diagrams/implementation-view-coffee-machine.svg"><img src="examples/coffee-machine/diagrams/implementation-view-coffee-machine.svg" alt="Implementation view of the coffee machine" width="340"></a></td>
    <td align="center" valign="top"><a href="VIEWS.md#physical-view"><b>Physical</b></a><br>
      <a href="examples/coffee-machine/diagrams/physical-view-coffee-machine.svg"><img src="examples/coffee-machine/diagrams/physical-view-coffee-machine.svg" alt="Physical view of the coffee machine" width="420"></a></td>
  </tr>
  <tr>
    <td align="center" valign="top"><a href="VIEWS.md#deployment-view"><b>Deployment</b></a><br>
      <a href="examples/coffee-machine/diagrams/deployment-view-coffee-machine.svg"><img src="examples/coffee-machine/diagrams/deployment-view-coffee-machine.svg" alt="Deployment view of the coffee machine" width="200"></a></td>
    <td align="center" valign="top"><a href="VIEWS.md#process-view"><b>Process</b></a><br>
      <a href="examples/coffee-machine/diagrams/process-view-coffee-machine.svg"><img src="examples/coffee-machine/diagrams/process-view-coffee-machine.svg" alt="Process view of the coffee machine" width="420"></a></td>
  </tr>
</table>

| View | Shows | Drawn from what you write |
| --- | --- | --- |
| Use case | How the system is used, and by whom | `use case`, `subject`, `actor`, `include` |
| Logical | What the system does: a tree of functions, and the use case each refines | `action def`, `action`, `#refinement dependency` |
| Implementation | The static structure of the software: packages, modules, their APIs and the functions they perform | `package`, `part def`, `port`, `perform` |
| Physical | The product itself: its parts, their ports and how they are wired | `part`, `port`, `connect`, `interface` |
| Deployment | Which hardware runs which software | `allocate` |
| Process | How the software modules talk at run time, as a sequence of messages | `occurrence def`, `message` |

A model needs only the views it wants. [VIEWS.md](VIEWS.md) says what each
view is for, and [LANGUAGE.md](LANGUAGE.md) what to write for it.

## Install

In VS Code, install
[Ortho for SysML v2](https://marketplace.visualstudio.com/items?itemName=ola-coding.ortho)
from the Marketplace; in Cursor or VSCodium, from
[Open VSX](https://open-vsx.org/extension/ola-coding/ortho). It carries
ortho, so there is nothing else to install.

Otherwise, download ortho for your system from the
[latest release](https://github.com/ola-coding/ortho/releases/latest), unpack
it and put `ortho` (`ortho.exe` on Windows) in a folder on your `PATH`:

| System               | File                                      |
| -------------------- | ----------------------------------------- |
| Windows              | `ortho-x86_64-pc-windows-msvc.zip`        |
| macOS, Apple silicon | `ortho-aarch64-apple-darwin.tar.gz`       |
| macOS, Intel         | `ortho-x86_64-apple-darwin.tar.gz`        |
| Linux, x86-64        | `ortho-x86_64-unknown-linux-musl.tar.gz`  |
| Linux, ARM           | `ortho-aarch64-unknown-linux-musl.tar.gz` |

macOS refuses to open a file downloaded with a browser until
`xattr -d com.apple.quarantine ortho` lifts the quarantine. With
[Rust](https://rustup.rs) installed, `cargo install ortho` builds it from
[crates.io](https://crates.io/crates/ortho).

## Your first diagram

Create `logical.sysml`:

```sysml
package Functions {
    private import StandardViewDefinitions::*;

    action def MakeBeverage {
        action prepareWater {
            action storeWater;
            action heatWater;
        }
        action prepareCoffee {
            action grindBeans;
            action extractShot;
        }
        action controlMachine {
            action selectBeverage;
            action sequenceRecipe;
        }
    }

    view 'Logical view — Coffee Machine' : GeneralView {
        expose MakeBeverage;
    }
}
```

The `view` is the diagram: its name is the heading, and it shows what it
exposes. Render it:

```sh
ortho render logical.sysml -o diagrams
```

and open `diagrams/logical-view-coffee-machine.svg` in a browser.

## Commands

```text
ortho render <models...> -o <folder> [--theme light|dark-glass] [--watch]
ortho init [--example coffee-machine|survey-drone|web-shop]
ortho lsp
```

- **`render`** reads the `.sysml` files and folders given as one model and
  writes an SVG for each view, named after it: `view 'Logical view'` writes
  `logical-view.svg`, and `view <logical> 'Logical view'` writes
  `logical.svg`. `--watch` renders again on every save. A view too big to
  read on its page is still drawn, with a warning;
  [split it](LANGUAGE.md#keeping-a-view-on-one-page).
- **`init`** writes AI instructions into the folder: a skill for Claude Code,
  instructions for GitHub Copilot and an `AGENTS.md` for other agents.
  `--example` also copies an example model.
- **`lsp`** is a language server for Zed, Neovim, Helix, Emacs or any editor
  that speaks LSP: problems as you type, go to definition, and a live preview
  of the diagrams in your browser.

## In CI

Commit the models and their SVGs. Rendering is deterministic, so CI can
render again and fail if anything changed. In GitHub Actions:

```yaml
- name: Diagrams are up to date
  run: |
    curl -sSL https://github.com/ola-coding/ortho/releases/latest/download/ortho-x86_64-unknown-linux-musl.tar.gz | tar xz -C "$RUNNER_TEMP"
    "$RUNNER_TEMP/ortho-x86_64-unknown-linux-musl/ortho" render models -o diagrams
    test -z "$(git status --porcelain diagrams)" || { git status diagrams; exit 1; }
```

To stay on one version, use `releases/download/v0.4.0/` in place of
`releases/latest/download/`.

## Examples

Three complete models, each with its views in its `diagrams/` folder:

- [coffee-machine](examples/coffee-machine/): a bean-to-cup coffee machine,
  the best place to start reading.
- [survey-drone](examples/survey-drone/): a quadcopter that surveys an area
  from the air, with a tablet ground station.
- [web-shop](examples/web-shop/): an online shop on AWS, software only, so
  four views.

The [pitch deck](https://ola-coding.github.io/ortho/) shows every view of
the first two beside the model source.

## Limits

- **Six views, not every SysML v2 diagram.** Requirements, states,
  calculations and constraints load, but no view draws them yet
  ([LANGUAGE.md](LANGUAGE.md#parsed-not-drawn)).
- **No standard library.** ortho knows the library's names, so a model may
  use them, but does not load it.

ortho is young, at version 0.4. The [backlog](DEVELOPMENT.md#backlog) lists
what comes next, [DEVELOPMENT.md](DEVELOPMENT.md) how ortho works inside, and
issues are welcome.

## License

MIT, see [LICENSE](LICENSE).
