<h1>
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="logo/ortho-logo-dark.svg">
    <img src="logo/ortho-logo.svg" alt="ortho" width="320">
  </picture>
</h1>

**Orthographic projection for architecture.** Just as a technical drawing
shows one object through several standard views, `ortho` renders six
architectural views as SVG, straight from SysML v2 text.

It is self-contained — no external programs, no network calls — so the same
model gives the same SVG on a laptop and in CI.

## Install

You need Node.js 24 or newer. Add ortho to your project from GitHub:

```sh
npm install --save-dev github:ola-coding/ortho
```

This installs the command as `./node_modules/.bin/ortho`. In npm scripts it is
simply `ortho`, which is how the rest of this page writes it.

## Your first diagram

Create `logical.sysml`:

```sysml
package Functions {
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
}
```

Render it:

```sh
./node_modules/.bin/ortho render logical.sysml -d logical -o logical-view.svg
```

Open `logical-view.svg` in a browser: a tree of ten functions, headed
"Logical view — Functions".

## The six views

Keep one model file per view. Use case and logical stand alone; the others
are rendered together with the files they refer to. List the view's own file
first.

| `-d` | Shows | Model files |
| --- | --- | --- |
| `use-case` | actors, use cases, system boundary | `use-case.sysml` |
| `logical` | what the system does, as a tree of functions | `logical.sysml` |
| `implementation` | software packages, their modules and APIs | `implementation.sysml logical.sysml` |
| `physical` | the product's parts, ports and wiring | `physical.sysml logical.sysml` |
| `deployment` | which hardware runs which software | `deployment.sysml implementation.sysml physical.sysml logical.sysml` |
| `process` | the messages of one runtime scenario | `process.sysml implementation.sysml logical.sysml` |

For example:

```sh
ortho render physical.sysml logical.sysml -d physical -o physical-view.svg
```

The process view draws every message it is given, so keep one scenario per
file.

## Options

```text
ortho render <models...> -d <view> -o <file.svg> [-t <heading>] [--theme <name>]
```

- `<models...>`: `.sysml` files, or directories of them, read as one model.
- `-d, --diagram`: one of the six views above.
- `-o, --out`: where to write the SVG. Missing folders are created.
- `-t, --title`: the diagram's heading. The default is the view name and the
  first file's package, e.g. "Deployment view — Deployment", so pass `-t` to
  name your system instead.
- `--theme`: `light` (default) or `dark-glass`, a transparent version for dark
  slides. PowerPoint, Keynote and Google Slides all insert SVG directly.

## Using it in a project

Commit your models and the generated SVGs, and render them with npm scripts:

```json
{
  "scripts": {
    "diagrams": "npm run diagrams:logical && npm run diagrams:physical",
    "diagrams:logical": "ortho render models/logical.sysml -d logical -o diagrams/logical-view.svg",
    "diagrams:physical": "ortho render models/physical.sysml models/logical.sysml -d physical -o diagrams/physical-view.svg"
  }
}
```

Rendering is deterministic, so CI can run `npm run diagrams` and fail if
`git diff` shows a change: the diagrams can never drift from the models.

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
