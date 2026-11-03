<h1>
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="logo/ortho-logo-dark.svg">
    <img src="logo/ortho-logo.svg" alt="ortho" width="320">
  </picture>
</h1>

**Orthographic projection for architecture.** Just as a technical drawing
shows one object through several standard views, `ortho` renders six
architectural views as SVG, straight from SysML v2 text: use case, logical,
implementation, physical, deployment and process.

It is one self-contained program — no runtime, no external programs, no
network calls — so the same model gives the same SVG on a laptop and in CI.

## Install

Download ortho for your system from the
[latest release](https://github.com/ola-coding/ortho/releases/latest):

| System               | File                                      |
| -------------------- | ----------------------------------------- |
| Windows              | `ortho-x86_64-pc-windows-msvc.zip`        |
| macOS, Apple silicon | `ortho-aarch64-apple-darwin.tar.gz`       |
| macOS, Intel         | `ortho-x86_64-apple-darwin.tar.gz`        |
| Linux, x86-64        | `ortho-x86_64-unknown-linux-musl.tar.gz`  |
| Linux, ARM           | `ortho-aarch64-unknown-linux-musl.tar.gz` |

Unpack it and put `ortho` (`ortho.exe` on Windows) in a folder on your `PATH`.
On Linux or macOS, from a terminal:

```sh
curl -L https://github.com/ola-coding/ortho/releases/latest/download/ortho-aarch64-apple-darwin.tar.gz | tar xz
sudo mv ortho-aarch64-apple-darwin/ortho /usr/local/bin/
```

with the file for your system in place of the Apple silicon one. On macOS, a
file downloaded with a browser is quarantined, and macOS refuses to open it;
`xattr -d com.apple.quarantine ortho` lifts that. Check that it works with
`ortho --version`.

Or build it from source with [Rust](https://rustup.rs):

```sh
cargo install --git https://github.com/ola-coding/ortho
```

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

The `view` at the end is the diagram: its name is the heading, and it shows
what it exposes. Render it:

```sh
ortho render logical.sysml -o diagrams
```

Open `diagrams/logical-view-coffee-machine.svg` in a browser: a tree of ten
functions, headed "Logical view — Coffee Machine".

## Options

```text
ortho render <models...> -o <folder> [--theme <name>]
```

- `<models...>`: `.sysml` files, or folders of them, read as one model.
- `-o, --out`: the folder to write to, one SVG per view, named after the view:
  `view 'Logical view'` writes `logical-view.svg`, and a short name,
  `view <logical> 'Logical view'`, `logical.svg`. Missing folders are created.
- `--theme`: `light` (default) or `dark-glass`, a transparent version for dark
  slides. PowerPoint, Keynote and Google Slides all insert SVG directly.

## Using it in a project

ortho renders every view a model declares, so one command draws them all.
Commit your models and the generated SVGs:

```sh
ortho render models -o diagrams
```

Rendering is deterministic, so CI can render again and fail if Git shows a
change: the diagrams can never drift from the models. In GitHub Actions:

```yaml
- name: Diagrams are up to date
  run: |
    curl -sSL https://github.com/ola-coding/ortho/releases/latest/download/ortho-x86_64-unknown-linux-musl.tar.gz | tar xz -C "$RUNNER_TEMP"
    "$RUNNER_TEMP/ortho-x86_64-unknown-linux-musl/ortho" render models -o diagrams
    test -z "$(git status --porcelain diagrams)" || { git status diagrams; exit 1; }
```

To keep every run on the same version, use `releases/download/v0.1.0/` in
place of `releases/latest/download/`.

## Examples

Two complete models, each with all six views rendered in its `diagrams/`
folder:

- [examples/coffee-machine/](examples/coffee-machine/): a bean-to-cup coffee
  machine. The smaller one, and the best place to start reading.
- [examples/survey-drone/](examples/survey-drone/): a quadcopter that surveys
  an area from the air, with a tablet ground station.

The [pitch deck](https://ola-coding.github.io/ortho/) shows every view of
both, with the model source beside each diagram.

## Learn more

- [VIEWS.md](VIEWS.md): what each view is for, and how it looks.
- [LANGUAGE.md](LANGUAGE.md): what to write, view by view.
- [DEVELOPMENT.md](DEVELOPMENT.md): for working on ortho itself.

## License

MIT, see [LICENSE](LICENSE).
