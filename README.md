# ortho

**Orthographic projection for architecture.** Just as a technical
drawing shows one object through several standard views, `ortho` renders the
five views of the 4+1 model as SVG — straight from SysML v2 text, with no
drawing tool in the loop.

It is fully self-contained: no external programs, no network calls, no LLMs at
render time, so it produces the same bytes on a laptop and in CI.

```sh
ortho render models -d part-definition -o diagrams/logical-view.svg
```

| CLI diagram type | 4+1 view | Shows |
| --- | --- | --- |
| `part-definition` | Logical View | part/interface/requirement defs, compositions, connections, satisfy traces |
| `use-case` | Scenarios | actors, use case ellipses, system boundary, includes |
| `package` | Development View | package folders, nesting, «import» dependencies |
| `allocation` | Physical View | software parts allocated onto the parts that run them |
| `sequence` | Process View | lifelines and ordered messages of one scenario |

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

Create `model.sysml`:

```sysml
package Demo {
    port def PowerPort;

    part def Robot {
        attribute mass_kg : Real = 42;
        part controller : Controller;
        part battery : Battery;
        connect battery.out to controller.in_;
    }
    part def Controller { port in_ : PowerPort; }
    part def Battery { port out : PowerPort; }

    requirement massReq;
    satisfy massReq by Robot;
}
```

Render it:

```sh
npx ortho render model.sysml -d part-definition -o robot.svg
```

Open `robot.svg` in any browser.

## CLI reference

```text
ortho render <models...> -d <type> -o <file.svg> [-t <heading>]
```

- `<models...>` — one or more `.sysml` files and/or directories (a directory
  means all `.sysml` files directly inside it). All inputs are parsed and
  linked as one workspace: cross-file references use qualified names
  (`System::Aircraft::fc`).
- `-d, --diagram` — one of the five types in the table above.
- `-o, --out` — output SVG path. Outputs go wherever you point them; the
  tool never writes anywhere else.
- `-t, --title` — override the frame heading. Default:
  `<View name> — <root packages of the first input>`. The generating file
  path always appears as small provenance text in the diagram corner.

**File selection is the scoping mechanism.** Each render lists the files
that belong on that diagram, subject first. In particular, the sequence
diagram renders *every* message it sees — so keep one scenario per file and
pass exactly one scenario file (plus the files its types come from):

```sh
npx ortho render scenario-x.sysml software.sysml system.sysml -d sequence -o x.svg
```

## Typical project integration

Keep models in `models/`, commit generated SVGs (everything-as-code), and
wire renders into npm scripts:

```json
{
  "scripts": {
    "diagrams": "npm run diagrams:logical && npm run diagrams:packages",
    "diagrams:logical": "ortho render models/system.sysml models/mechanics.sysml models/electronics.sysml models/requirements.sysml -d part-definition -o diagrams/logical-view.svg",
    "diagrams:packages": "ortho render models -d package -o diagrams/development-view.svg"
  }
}
```

Because rendering is deterministic and dependency-free, a CI job can run
`npm run diagrams` and fail on a dirty git diff to keep diagrams in sync
with models.

## The language subset

The grammar is a growing, spec-oriented subset of the SysML v2 textual
notation: packages/imports, part/port/interface defs and usages, attributes,
connections (`connect a.x to b.y`), requirements + `satisfy`, use cases with
actors/subjects/includes, `allocate`, and actions with `message`/`then`.
See `grammar/sysml.langium` for the exact grammar and
[DEVELOPMENT.md](DEVELOPMENT.md) for deliberate deviations (e.g. no UML
`extend` — SysML v2 dropped it).

## Programmatic API

```ts
import { createSysmlServices, parseSysmlFiles, diagramTypes } from 'ortho';

const services = createSysmlServices();
const { model } = await parseSysmlFiles(services.Sysml, ['models/system.sysml']);
const { svg } = await diagramTypes['part-definition'].render(model, {
    heading: 'Logical View — MySystem',
    source: 'models/system.sysml'
});
```

## Further reading

- [DEVELOPMENT.md](DEVELOPMENT.md) — architecture, design decisions and the
  deliberate spec deviations behind the tool.

## Example models

Two complete worked models ship with the tool, each with all five generated
views:

- [examples/auv-system/](examples/auv-system/) — an autonomous unmanned aerial
  vehicle: aircraft, radio controller, batteries and charger, with a
  follow-person scenario. The larger of the two, and the one that pushes the
  layout hardest.
- [examples/coffee-machine/](examples/coffee-machine/) — a bean-to-cup coffee
  machine: water and coffee subsystems, a milk frother and one control board,
  with a make-cappuccino scenario. The leaner one, and the better starting
  point to read.

Both use the same partition, and it is the one to copy for a mechatronic
product — where mass and geometry belong to mechanics and the power budget to
electronics:

```text
requirements.sysml   use-cases.sysml   scenario-*.sysml   system.sysml
                                                mechanics.sysml
                                                electronics.sysml
                                                software.sysml
```

It works because `system.sysml` holds only two things — the interfaces that
cross a domain boundary, and blocks that declare their *ports only* — while
every domain **specializes** the block it realizes
(`Mechanics::FoldingPropeller :> System::Propeller`). So all three domains
depend on the architecture and none of them on each other, which is what lets a
domain be reviewed, or replaced, on its own.

## License

MIT — see [LICENSE](LICENSE).
