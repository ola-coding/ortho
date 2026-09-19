# ortho

**Orthographic projection for architecture.** Just as a technical drawing
shows one object through several standard views, `ortho` renders six
architectural views as SVG — straight from SysML v2 text, with no drawing tool
in the loop.

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
| `physical` | Physical view | the product: parts nested in their assemblies, their ports, and the wiring between them |
| `deployment` | Deployment view | hardware nodes, grouped by the device they sit in, with the software they host drawn inside |
| `process` | Process view | lifelines and ordered messages of one scenario |

The set is Kruchten's 4+1 with two changes: his Physical view is split into
Physical (the product itself) and Deployment (which hardware runs which
software), and the Logical view holds functions rather than structure.
[VIEWS.md](VIEWS.md) defines what belongs on each view and why.

## Requirements

- Node.js 24 or newer. Nothing else.

CI typechecks and runs the suite on both 24 and 26, so the floor is tested
rather than merely declared.

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
npm install              # also builds dist/ via prepare
npm test                 # vitest suites
npm run typecheck        # noEmit check over src, tests and scripts
npm run render:examples  # regenerate the example diagrams
npm run deck             # build the pitch deck into marketing/
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

Open `logical-view.svg` in any browser: a tree of ten functions, headed
"Logical view — Functions".

## CLI reference

```text
ortho render <models...> -d <type> -o <file.svg> [-t <heading>]
```

- `<models...>` — one or more `.sysml` files and/or directories (a directory
  means all `.sysml` files directly inside it). All inputs are parsed and
  linked as one workspace: cross-file references use qualified names
  (`Hardware::SurveyDrone::fc`).
- `-d, --diagram` — one of the six types in the table above.
- `-o, --out` — output SVG path. Outputs go wherever you point them; the
  tool never writes anywhere else.
- `-t, --title` — override the frame heading. Default:
  `<View name> — <root packages of the first input>`, or the directory name
  when the first input is a directory. Every input file is listed as small
  provenance text in the diagram corner, so a view rendered with companions
  names all of them.

`ortho --version` prints the package version; `ortho --help` and
`ortho render --help` list the commands and options.

### One model file per view

Each view is rendered from the file that holds it. Four of the six stand
alone; two need companions, because their content is a relation to other
files:

```sh
# Self-contained: use-case, logical, implementation, physical
npx ortho render physical.sysml -d physical -o physical-view.svg

# Deployment is nothing but the mapping between software and hardware
npx ortho render deployment.sysml implementation.sysml physical.sysml \
    -d deployment -o deployment-view.svg

# The process view's lifelines are typed by the software modules
npx ortho render process.sysml implementation.sysml -d process -o process-view.svg
```

The process view renders *every* message it sees, so keep one scenario per
file and pass exactly one scenario file.

Pass `-t` for those two. The default heading names the first input's root
package, so `deployment.sysml` renders as "Deployment view — Deployment"
rather than naming your system — which is why the bundled examples override
it and read "Deployment view — Survey Drone".

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

## Example models

Two complete worked models ship with the tool, each with all six generated
views in its `diagrams/` folder:

- [examples/survey-drone/](examples/survey-drone/) — a quadcopter that surveys
  a ground area from the air: airframe, avionics and a tablet ground station,
  with an ArduPilot-style flight stack and a whole survey mission as its
  scenario. The larger of the two — fourteen parts across two assemblies and
  twenty-four deployed modules — and the one that pushes the layout hardest.
- [examples/coffee-machine/](examples/coffee-machine/) — a bean-to-cup coffee
  machine: water and coffee paths, a steam wand and one control board, with a
  make-cappuccino scenario. Ten physical parts in one enclosure; the leaner
  one, and the better starting point to read.

Both use the same partition — one file per view, named for the view it feeds:

```text
use-case.sysml   logical.sysml   implementation.sysml
                 physical.sysml  deployment.sysml   process.sysml
```

It works because the views own distinct content rather than slicing shared
content: `logical.sysml` names no component, and `implementation.sysml` and
`physical.sysml` never reference each other — software knows nothing of
boards, hardware nothing of programs. The mapping between them exists only in
`deployment.sysml`, which declares nothing of its own. DEVELOPMENT.md's
[one model file per view](DEVELOPMENT.md#one-model-file-per-view) gives the
full rule and the three conventions that keep it true.

## The language subset

The grammar is a growing, spec-oriented subset of the SysML v2 textual
notation:

- packages and imports
- part, port and interface defs and usages, with `:>` specialization and
  multiplicities such as `[4]` or `[1..3]`
- attributes
- connections (`connect a.x to b.y`), optionally named and typed by an
  interface
- use cases with actors, subjects and `include`
- `allocate`
- actions, nested to any depth, and `message` / `then` for scenarios

`requirement`, `satisfy` and `perform` also parse, but no view draws them. One
relation cannot be written yet: `allocate` cannot name a function, so the
realization of a Logical-view function by a component is not expressible.

See `grammar/sysml.langium` for the exact grammar and
[DEVELOPMENT.md](DEVELOPMENT.md) for the deliberate deviations from the spec
(e.g. no UML `extend` — SysML v2 dropped it).

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
- [Pitch deck](https://ola-coding.github.io/ortho/) — the six views of both
  example models, model source beside each generated diagram. Rebuilt and
  published to GitHub Pages on every push to `main`; to build it locally, run
  `npm run deck` and open `marketing/ortho-deck.html`.

## License

MIT — see [LICENSE](LICENSE).
