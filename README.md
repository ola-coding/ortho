# ortho

**Orthographic projection for architecture.** Just as a technical
drawing shows one object through several standard views, `ortho` renders the
six architectural views as SVG — straight from SysML v2 text, with no
drawing tool in the loop.

It is fully self-contained: no external programs, no network calls, no LLMs at
render time, so it produces the same bytes on a laptop and in CI.

```sh
ortho render models/logical.sysml -d logical -o diagrams/logical-view.svg
```

| CLI diagram type | View | Shows |
| --- | --- | --- |
| `use-case` | Use case view | actors, use case ellipses, system boundary, includes |
| `logical` | Logical view | the capability tree: what the system does, as functions |
| `implementation` | Implementation view | software packages, the modules in them, «import» dependencies |
| `physical` | Physical view | the product: parts, ports, connections, cabling |
| `deployment` | Deployment view | hardware nodes with the software they host drawn inside |
| `process` | Process view | lifelines and ordered messages of one scenario |

[VIEWS.md](VIEWS.md) defines what belongs on each view and why the set divides
this way.

## Requirements

- Node.js 18 or newer. Nothing else.

## Install

### Into another project (recommended)

Add it as a dev-dependency from a local path or a git URL — the tool's own
`src/` and build output stay inside `node_modules/ortho` and never mix with
your project's code:

```sh
npm install --save-dev ../path/to/ortho
# or
npm install --save-dev git+https://your-git-host/ortho.git
```

Both forms build the package automatically on install (its `prepare` script
runs the Langium code generation and TypeScript build). Then:

```sh
npx ortho --help
```

### Working on this repo itself

```sh
npm install          # also builds dist/ via prepare
npm test             # vitest suites
npm run render:examples  # regenerate the example diagrams
```

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
npx ortho render logical.sysml -d logical -o logical-view.svg
```

Open `logical-view.svg` in any browser.

## CLI reference

```text
ortho render <models...> -d <type> -o <file.svg> [-t <heading>]
```

- `<models...>` — one or more `.sysml` files and/or directories (a directory
  means all `.sysml` files directly inside it). All inputs are parsed and
  linked as one workspace: cross-file references use qualified names
  (`Hardware::Aircraft::fc`).
- `-d, --diagram` — one of the six types in the table above.
- `-o, --out` — output SVG path. Outputs go wherever you point them; the
  tool never writes anywhere else.
- `-t, --title` — override the frame heading. Default:
  `<View name> — <root packages of the first input>`. The generating file
  path always appears as small provenance text in the diagram corner.

**One model file per view.** Each view is rendered from the file that holds
it. Four of the six stand alone; two need a companion, because their content
is a relation between other views:

```sh
# Self-contained
npx ortho render physical.sysml -d physical -o physical-view.svg

# Deployment is nothing but the mapping between the other two files
npx ortho render deployment.sysml implementation.sysml physical.sysml \
    -d deployment -o deployment-view.svg

# The process view's lifelines are typed by the software modules
npx ortho render process.sysml implementation.sysml -d process -o process-view.svg
```

The process view renders *every* message it sees, so keep one scenario per
file and pass exactly one scenario file.

## Typical project integration

Keep models in `models/`, commit generated SVGs (everything-as-code), and
wire renders into npm scripts:

```json
{
  "scripts": {
    "diagrams": "npm run diagrams:logical && npm run diagrams:physical",
    "diagrams:logical": "ortho render models/logical.sysml -d logical -o diagrams/logical-view.svg",
    "diagrams:physical": "ortho render models/physical.sysml -d physical -o diagrams/physical-view.svg"
  }
}
```

Because rendering is deterministic and dependency-free, a CI job can run
`npm run diagrams` and fail on a dirty git diff to keep diagrams in sync
with models.

## The language subset

The grammar is a growing, spec-oriented subset of the SysML v2 textual
notation: packages/imports, part/port/interface defs and usages, attributes,
connections (`connect a.x to b.y`), use cases with actors/subjects/includes,
`allocate`, and actions with nesting and `message`/`then`.
See `grammar/sysml.langium` for the exact grammar and
[DEVELOPMENT.md](DEVELOPMENT.md) for deliberate deviations (e.g. no UML
`extend` — SysML v2 dropped it).

## Programmatic API

```ts
import { createSysmlServices, parseSysmlFiles, diagramTypes } from 'ortho';

const services = createSysmlServices();
const { model } = await parseSysmlFiles(services.Sysml, ['models/logical.sysml']);
const { svg } = await diagramTypes['logical'].render(model, {
    heading: 'Logical view — MySystem',
    source: 'models/logical.sysml'
});
```

## Further reading

- [VIEWS.md](VIEWS.md) — the six views: what each one admits, and why.
- [DEVELOPMENT.md](DEVELOPMENT.md) — architecture, design decisions and the
  deliberate spec deviations behind the tool.

## Example models

Two complete worked models ship with the tool, each with all six generated
views:

- [examples/auv-system/](examples/auv-system/) — an autonomous unmanned aerial
  vehicle: aircraft, radio controller, batteries and charger, with a
  follow-person scenario. The larger of the two, and the one that pushes the
  layout hardest.
- [examples/coffee-machine/](examples/coffee-machine/) — a bean-to-cup coffee
  machine: water and coffee paths, a steam wand and one control board, with a
  make-cappuccino scenario. The leaner one, and the better starting point to
  read.

Both use the same partition — one file per view, named for the view it feeds:

```text
use-case.sysml   logical.sysml   implementation.sysml
                 physical.sysml  deployment.sysml   process.sysml
```

It works because the views own distinct content rather than slicing shared
content. `logical.sysml` names no component, `implementation.sysml` names no
board, and `physical.sysml` names no program — so each of those three parses
and renders entirely on its own. Only `deployment.sysml` references anything
outside itself, and that is the point: its whole content is the mapping
between the software and the hardware.

## License

MIT — see [LICENSE](LICENSE).
