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
./node_modules/.bin/ortho render logical.sysml -o logical-view.svg
```

Open `logical-view.svg` in a browser: a tree of ten functions, headed
"Logical view — Coffee Machine".

## The six views

Keep one model file per view, each declaring its view with one of SysML's
standard view definitions. Use case and logical stand alone; the others are
rendered together with the files they refer to, the view's own file first.

| View | Standard view | Model files |
| --- | --- | --- |
| Use case | `GeneralView` | `use-case.sysml` |
| Logical | `GeneralView` | `logical.sysml` |
| Implementation | `GeneralView` | `implementation.sysml logical.sysml` |
| Physical | `InterconnectionView` | `physical.sysml logical.sysml` |
| Deployment | `GeneralView` | `deployment.sysml implementation.sysml physical.sysml logical.sysml` |
| Process | `SequenceView` | `process.sysml implementation.sysml logical.sysml` |

For example:

```sh
ortho render physical.sysml logical.sysml -o physical-view.svg
```

## Options

```text
ortho render <models...> -o <file.svg> [-t <heading>] [--theme <name>]
```

- `<models...>`: `.sysml` files, or directories of them, read as one model.
  The first file declares the view to render.
- `-o, --out`: where to write the SVG. Missing folders are created.
- `-t, --title`: a heading other than the view's name.
- `--theme`: `light` (default) or `dark-glass`, a transparent version for dark
  slides. PowerPoint, Keynote and Google Slides all insert SVG directly.

## Using it in a project

Commit your models and the generated SVGs, and render them with npm scripts:

```json
{
  "scripts": {
    "diagrams": "npm run diagrams:logical && npm run diagrams:physical",
    "diagrams:logical": "ortho render models/logical.sysml -o diagrams/logical-view.svg",
    "diagrams:physical": "ortho render models/physical.sysml models/logical.sysml -o diagrams/physical-view.svg"
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
