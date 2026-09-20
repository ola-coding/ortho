# Developing ortho

Notes for changing this tool. For *using* it, see [README.md](README.md).

## Pipeline

```text
.sysml text
  → parse            Langium-generated parser + custom scoping   (src/parser/)
  → model            resolved AST, cross-references linked        (src/generated/)
  → extract          one extractor per diagram type               (src/diagrams/)
  → layout           elkjs (graph diagrams only)                  (src/layout/)
  → render           hand-written SVG serialization               (src/render/)
  → .svg file
```

`src/pipeline.ts` wires extractor + layout + renderer into one entry per
diagram type and is the single place to register a new diagram.
`src/cli.ts` is the CLI, and the only way in: ortho is a command-line tool,
with no programmatic API to keep stable.

### Why these dependencies

- **Langium** — TypeScript-native language workbench (Xtext lineage, same as
  the OMG pilot implementation's tooling). We need cross-reference resolution
  and scoping, which is the bulk of the work in a SysML parser; hand-rolling
  it would be far larger than the grammar itself. Grammar rules are written
  against the OMG textual-notation spec, not copied from the EPL-licensed
  pilot implementation.
- **elkjs** — pure JS port of the Eclipse Layout Kernel: layered EA/Visio-like
  layouts, hierarchical (nested) nodes, port-aware routing. An ordinary npm
  dependency, no external binary or service.
- **No renderer dependency** — SVG is emitted as text. No headless browser,
  no DOM. This is what keeps the "self-contained, no external programs"
  constraint true, and it makes output deterministic and diffable.

## Design decisions and gotchas

### Name resolution (`src/parser/sysml-scope.ts`)

Two custom Langium services, both load-bearing:

- **`SysmlScopeComputation`** exports every named element under its fully
  qualified name *and* publishes it into each enclosing container under its
  relatively qualified name. So `fc` resolves inside `Physical`,
  `Physical::fc` from a sibling package, and the full FQN globally — mirroring
  how SysML v2 walks enclosing namespaces.
- **`SysmlScopeProvider`** resolves connector-end feature chains from both
  directions. Segments *after* the first (`battery.powerOut`) resolve against
  the members of the previous segment's type, including members inherited
  through `:>`. The *first* segment additionally sees the features its
  enclosing definition inherits through `:>`, so a realization can
  `connect heater.hotWaterOut to hotWaterOut` where `hotWaterOut` is declared
  on the block it specializes. Locally declared members shadow inherited ones
  and are left to the default scope. Without this, every specialization would
  have to redeclare the ports it inherits — which models a *different* port that
  merely shares a name. The bundled examples no longer lean on this (they
  declare concrete parts rather than realizing abstract blocks), so
  `tests/parser.test.ts` is what keeps it honest.

### SysML v2 conformance (`src/parser/sysml-validator.ts`)

A model ortho accepts should load in any other SysML v2 tool. Most of that is
the grammar's job: an import states its visibility, `include`, `perform` and
both ends of `satisfy` name usages, and a message payload resolves to an
`attribute def`. Three rules the grammar cannot express are checked after
linking, each as an error:

- **A reserved word is not a name.** The spec reserves 128 words; ortho's
  grammar reserves only the 31 it parses, so without this check `part frame`
  would pass here and fail everywhere else. The survey drone had exactly that.
- **A connector end starts at a usage.** `Hardware::CoffeeMachine::controller`
  reaches into a definition, which SysML v2 rejects; the chain has to start at
  a usage and continue with dots, `Hardware::machine.controller`.
- **A use case declares its subject before its actors**, because the subject
  is its first parameter. A secondary actor, added on a single use case,
  therefore needs that use case to restate its subject.

Every one of these was found by running the examples through the OMG pilot
implementation; [LANGUAGE.md](LANGUAGE.md) records that check.

### elkjs: edge coordinates are relative to the edge's container

ELK reports each edge's coordinates relative to the node that **contains**
the edge, not to the diagram root, and names that node in the edge's
`container` field. Usually it is the least common ancestor of the two ends —
but an edge from a box's own port to one of its children (a boundary port on
the physical view, such as the drone's `rfLink` feeding the radio fitted
inside it) lives *inside* that box, which no ancestor walk finds. elkjs 0.9.3
does **not** support `org.eclipse.elk.json.edgeCoords: ROOT` (the option is
absent from the bundle — verified, not assumed), so `src/layout/elk-layout.ts`
translates every edge, and every label ELK placed on it, by its container's
absolute position, keeping the ancestor walk only as a fallback. If a future
elkjs adds the option, that code can go.

### Text measurement without a browser

`src/render/text-metrics.ts` embeds standard Arial/Helvetica advance widths
(1/1000 em, ASCII 32–126, regular and bold). Extractors size boxes from
`measureText()`, never from character counts. The SVG font stack leads with
Arial so metrics match what renders; Segoe UI (later in the stack) is
narrower, so any error leaves *extra* room rather than overflowing. This is
the deliberate alternative to a headless browser or font-file parsing.

### Rendering conventions

- **No current view draws generalization or composition.** Since the physical
  view became an internal block diagram, containment is shown by nesting and
  types never meet on a diagram. The renderer keeps both markers: a
  generalization is laid out super → sub, so ELK's `DOWN` direction puts the
  supertype above, and the hollow triangle is drawn at the layout *source* end
  via `marker-start` plus `orient="auto-start-reverse"`.
- **Ellipse clipping**: ELK attaches edges to bounding boxes, so terminal
  segments of edges touching ellipse nodes are extended to the actual curve
  (quadratic intersection). Two guards matter: a point already on/inside the
  curve must not clip — floating point otherwise picks the *far* intersection
  and draws a line straight through the ellipse — and the adjustment is
  capped at one radius.
- **The physical view draws instances, not types.** It is an internal block
  diagram: one box per part *in the product*, nested inside the part that
  contains it, walked from the top-level definitions — those nothing uses as a
  part type and no other definition specializes — including parts and
  connections inherited through `:>`. Each connection is resolved segment by
  segment from the part whose body declares it, so a type fitted twice (the
  drone's IP radio, once in the drone and once in the ground station) is two
  boxes, each wired on its own. Drawing definitions instead, as the view
  originally did, collapsed both radios into one box and wired the operator's
  tablet straight to the drone's companion computer. Port defs, interface
  defs and attributes are not drawn: ports are markers on the parts, an
  interface is the label on the connection it types, and attribute values are
  specification, not topology.
- **Three views draw containment instead of edges.** The deployment view nests
  software inside the three-dimensional node that hosts it and emits no edges
  at all, so a host with nothing drawn in it visibly hosts nothing. The hosts in
  turn sit in a frame for the device they are fitted in, found from the physical
  containment: a device fitted exactly once is labelled by that usage
  (`drone : SurveyDrone`) and nested in its own container's frame. With no
  edges to layer, ELK's layered algorithm put every box in one row, so the
  deployment view uses `rectpacking` instead — level by level, because packing
  cannot lay out a hierarchy in one pass — which keeps the model's order and a
  page-shaped aspect ratio. (ELK's `box` algorithm packs too, but reorders by
  size.) The implementation view nests modules inside their package the same
  way, and the physical view nests each part inside its assembly. `node3d` also
  reserves `NODE_DEPTH` of extra padding at the top and right, because its
  depth edge is drawn *inside* its own bounds.
- **Use case lines are straight, and actors pick a side.** ELK places the use
  case view's shapes; `src/layout/straight-edges.ts` then redraws each
  association and include as one straight segment. Right-angled bends are a
  class-diagram habit, and even ELK's `POLYLINE` routes kinked at every layer.
  Aimed at an ellipse's centre, a line to a use case standing behind another
  would clip it, so the end slides round the outline 7.5° at a time until the
  line clears every other shape by 4 px. Lines to one actor meet it down the
  side of its figure in the order they leave it, so they never cross there,
  each as near its even share of that side as a clear line allows. Where a
  use case stands right behind another as the actor sees it — ELK centres an
  actor on its use cases, and the included ones stand a column further in —
  no sliding helps, so the actor is nudged up or down its column, 4 px at a
  time and at most 60, until all of its lines can be straight; it goes
  further only to find a place where none of them crosses a line already
  drawn. Lines between use cases are placed first, since no nudge moves
  them. A line nothing clears keeps ELK's `POLYLINE` route. Each
  «include» label then sits beside the middle of its line, on the first side
  clear of every shape and every other line. An actor declared on a use case
  def is primary and stands left; one added on a single use case only is
  secondary and stands right. An association has no direction, so the
  extractor reverses a secondary actor's edge purely as a layering hint; the
  renderer draws no marker on either.
- **The logical view is laid out by ortho, not ELK.** `src/layout/tree-layout.ts`
  draws the capability tree as a work-breakdown chart: a family whose children
  are all leaves is listed vertically beneath its parent, off a spine; any other
  family is spread in a row, one stem feeding a bus. The parent stands over its
  middle child, so the stem runs straight down into it; a row with an even
  number of children has no middle child, and there the parent stands over the
  middle of the row. Centring over the row in every case put the stem a few
  pixels beside the middle child whenever box widths differed, which read as a
  wobble rather than a line. ELK's layered algorithm put every leaf in one row —
  one such tree measured 3322 px across for 27 functions — and attached each
  branch at its own point on the parent, which read as wiring. Branches are
  `decomposition` edges with no arrowhead — direction is carried by the
  layout — and a test asserts no marker reaches that SVG.
- **Realization is a compartment, not an edge.** A part that `perform`s a
  function lists it in the spec's *perform actions* compartment
  (§7.17.6) — on the implementation view for a module, on the physical view
  for a device, and on both where a program and a device realize the same
  function. An edge would need the function and the component on one diagram,
  which is a seventh view; a compartment needs only the box that is already
  there. A part usage shows what its definition performs as well as its own,
  and `paddingOf` in `src/layout/elk-layout.ts` adds the compartment's height
  to a container's top padding so nested parts start below it.
- **Themes are colour only** (`src/render/theme.ts`). `light` is the drawing
  on paper the examples commit; `dark-glass` is for slides — no page, glass
  fills, light ink, one accent, and no frame, since a slide has its own
  title. Every coordinate is computed before a theme is consulted, so the two
  differ in paint alone; a test asserts that every position the light theme
  draws appears in the dark one. A theme that draws no gloss and no shadow
  must add nothing at all to the output, not an empty line: the committed
  example diagrams are byte-for-byte what they were before themes existed,
  and CI would fail otherwise.
- **Label placement.** Port labels, the interface labels on physical-view
  wires, «include» on the use case view and «import» on the implementation
  view are handed to ELK as real labels, so ELK reserves room for them, puts
  each port label outside its box beside the port (flipping sides when a
  neighbour would collide) and keeps wire and keyword labels clear of boxes.
  Port labels are painted after every edge, so no line strikes through one.
  Once a use case line is drawn straight, its label moves beside the
  straight line (see above). Every other edge label sits at the line's
  midpoint. Relationship *keywords* get «guillemets»; user-supplied names
  (e.g. a named allocation) render plainly.
- **Sequence diagrams bypass ELK entirely** — lifeline/message layout is
  deterministic, so `src/render/sequence-renderer.ts` does its own: per-gap
  column spacing (gaps widen only where a message label spans them) and
  activation bars (active from a message's arrival until the lifeline next
  sends — a heuristic that reads correctly for linear scenarios). A lifeline
  whose part is untyped, or typed by a part def some loaded use case casts as
  an actor, is a person and gets the use case view's stick figure; the heads are
  then bottom-aligned so every lifeline starts level.

### Diagram headings

The frame pentagon carries the **view name plus the model's root package**
(`Logical view — Functions`), not UML diagram-kind jargon. The `.sysml` paths
the diagram came from stay visible as small gray provenance text in the
bottom-right corner — every input, so the two views rendered with companions
name all of theirs — because everything-as-code means a diagram always names
its source. The SVG `<title>` holds the combined string. The CLI's
`-d logical` etc. are command-line vocabulary only.

### One model file per view

There is no `--scope` flag. Each view is rendered from the file that holds it,
and all inputs are linked as one workspace. Use case and logical are
self-contained; the other four are rendered with companions because their
content is a relation between files:

- **implementation** and **physical** need `logical.sysml`, since their parts
  name the functions they `perform`.
- **deployment** needs `implementation.sysml` and `physical.sysml`, since it
  declares nothing but the mapping between them, and `logical.sysml` behind
  them.
- **process** needs `implementation.sysml`, since its lifelines are typed by
  the software modules, and `logical.sysml` behind it.

This also keeps two extractors apart that would otherwise collide: a scenario
file nests actions just as a capability tree does, so `logical.ts` would happily
draw a scenario's action as a lonely root. Passing one file per view is what
stops it.

## Deliberate spec deviations

These are choices, not bugs:

- **No UML `extend`** for use cases. SysML v2 dropped it; scenario
  decomposition uses `include` and `perform`.
- **`import` parses but is not semantically enforced.** Everything is
  reachable by qualified name. The implementation view reads imports as
  dependency edges; nothing restricts visibility. The visibility itself has to
  be written, as the spec requires, but `private`, `protected` and `public`
  behave alike.
- **Requirements are not on any view.** `requirement`, `satisfy` and their
  traces still parse, but no extractor consumes them: the six views cover
  usage, function, software, product, deployment and runtime, and none of them
  is a specification view. The grammar keeps the surface: a non-functional
  requirement is often what proposes a technical solution on the
  implementation or physical view, so a model that carries requirements must
  keep loading.
- **Realization is written with `perform`, not `allocate`.** The spec defines
  allocation as a connection whose target realizes the source's intent
  (§7.15), and its own examples allocate a logical *component* to a physical
  one. ortho's logical view holds no components, so the relation runs from a
  function straight to what does it, which is what `perform` states (§7.17.6).
  `allocate` stays where both ends are components: the deployment view.
- **Attribute types are plain qualified names**, not cross-references —
  there is no standard library to resolve `Real`/`String` against yet. The
  examples import `ScalarValues::*` so that tools which have one resolve them.
- **Message ordering is document order**; `then` is accepted notation, not
  semantics.
- **Only usage forms** exist for allocation (`allocate`), not `allocation def`.
- The grammar is a **growing subset** of SysML v2/KerML, front-loaded to what
  the six views need, and [LANGUAGE.md](LANGUAGE.md) is that subset written
  down. Full spec compliance is an open-ended goal, not a pending task.

## Working on the code

```sh
npm install          # also builds dist/ (prepare script)
npm run build        # langium generate + tsc -p tsconfig.build.json → dist/
npm run typecheck    # noEmit check over src + tests + scripts
npm test             # vitest
npm run render:examples  # regenerate every example's diagrams/
```

CI runs `typecheck` and `test` on Node 24 and 26 — the floor `package.json`
declares, which the other jobs also use, and the newest line — then checks
that a fresh render matches the committed diagrams, and only then builds and
publishes the deck.
Nothing reaches GitHub Pages past a failing test or a stale SVG.

Grammar changes require `npm run langium:generate` (folded into `build`),
which regenerates `src/generated/`. That directory is committed so a fresh
clone can typecheck before building.

**Toolchain trap:** never add `chevrotain` as a direct dependency. Langium
pins a specific range (`~11.0.3`); a direct `^11.0.3` lets npm hoist a newer
patch that breaks `langium generate` with a cryptic
`Error: non exhaustive match` (a chevrotain GAST-visitor error, not a Langium
bug). Let it resolve transitively. If that error ever appears, compare
`node_modules/chevrotain/package.json` against Langium's declared range first.

**Checking against the OMG pilot implementation.** The one-off check in
[LANGUAGE.md](LANGUAGE.md#how-this-is-checked) can be repeated without
installing conda or Jupyter:

- Download `jupyter-sysml-kernel` from conda-forge (0.62.0 was used). The
  `.conda` file is a zip holding a `.tar.zst`, which holds
  `share/jupyter/kernels/sysml/jupyter-sysml-kernel-<version>-all.jar` and the
  standard library in `sysml.library/`. It needs Java 21.
- On Windows, `SysMLInteractive.loadLibrary` fails: the library's folder names
  contain spaces, and they reach the file system still URL-encoded. Copy the
  library with the spaces replaced, and read its three folders yourself from
  `jshell` with `SysMLInteractive.readAll(folder, true, ".kerml")` for the
  kernel libraries and `".sysml"` for the other two.
- Give one `SysMLInteractive.process(text)` session each example's six files
  in dependency order, use case, logical, implementation, physical,
  deployment, process, so cross-file references resolve, and print
  `getIssues()` after each.

Test fixtures in `tests/fixtures/*.sysml` are inputs only. The real example
models are `examples/survey-drone/` and `examples/coffee-machine/` — two
deliberately different domains, so a rendering change that only suits one of
them shows up immediately. Their committed SVGs should be regenerated (and
reviewed in a browser) whenever rendering changes.

Both carry the **same partition**, deliberately: one model file per view, each
named for the view it feeds. The two examples differ in subject and in scale —
the drone has fourteen physical parts against the machine's ten, two stacks
of software where the machine has one, and a capability tree a third again as
large — but not in structure.

```text
use-case.sysml   logical.sysml   implementation.sysml
                 physical.sysml  deployment.sysml   process.sysml
```

The load-bearing rule is that **a view file names nothing from another view**:

```text
use-case, logical                            →  (nothing)
implementation, physical                     →  logical
deployment                                   →  implementation, physical
process                                      →  implementation
```

This works because the six views own distinct content rather than slicing
shared content. The dependencies all point the same way, towards what the
system does: a component names the functions it realizes, and no function
ever names a component. A test asserts that use case and logical still parse
and render entirely on their own, for both examples.

Three conventions produce that shape, and the example tests enforce all three:

- **`logical.sysml` names no component.** Not a thermoblock, not a control
  board, not a program: only what the system does. A capability tree that
  mentions a realization has stopped being medium-independent, and the test
  checks for the specific realization names declared in the other two files.
- **`implementation.sysml` and `physical.sysml` never reference each other.**
  Software knows nothing of boards and hardware knows nothing of programs. The
  mapping between them exists only in `deployment.sysml`, which is why that
  file declares nothing of its own. Both may name `logical.sysml`, because
  both realize functions.
- **Both ends of an allocation are named once.** `implementation.sysml`
  declares a single `part sw { ... }` tree of deployable parts, and
  `physical.sysml` ends with one usage of the whole product,
  `part machine : CoffeeMachine` or `part system : SurveyDroneSystem`.
  `deployment.sysml` allocates from the one into the other,
  `Software::sw.video` to `Hardware::system.drone.companion`, because an
  allocation end is a feature chain that has to start at a usage. The
  product's usage is also what heads the physical view, which is why its
  top box reads `system : SurveyDroneSystem`.

Requirements are absent from both examples. They parse, but no view consumes
them (see the deviations above), so committing them would leave model text that
no diagram can show.

Because `import` is not semantically enforced (see the deviations above), every
cross-file reference in these examples is written as a qualified name. That is
load-bearing, not stylistic: bare names do not resolve across files, so the
`Software::` and `Hardware::` prefixes in `deployment.sysml` are what make the
crossing visible in the source.

**The deck's figures are only as fresh as the examples' committed SVGs.**
`marketing/deck.template.html` inlines the diagrams by filename, so a model or
rendering change means `npm run render:examples` before `npm run deck`, and a
figure still naming a diagram the examples no longer produce fails the deck
build outright, which takes the Pages deploy down with it. The test count on
the constraints sheet is filled in at build time from `vitest list`. The only
numbers still typed by hand are in the table comparing the two examples'
sizes, which wants a look when a model grows.

## Backlog

Possible next steps, in priority order:

- [x] **Raise the Node floor.** `engines` now declares `>=24`, the current LTS
  (supported until April 2028), and CI tests 24 and 26. It was `>=18`, two
  end-of-life generations back. 22 would have been the conventional floor, but
  it reaches end-of-life in April 2027, months after a first npm release, and
  raising a published floor is a breaking change. Nothing under `src/` had to
  change.
- [x] **Align the stem with the middle child in the logical view.** With an
  odd number of children the parent's stem now lands on the middle child's
  stem; with an even number it stays at the middle of the row. Both examples
  had the wobble: the survey drone's root sat 4.5 px beside `communicate`, the
  coffee machine's 13.3 px beside `prepareMilk`.
- [x] **Align with the SysML v2 spec.** [LANGUAGE.md](LANGUAGE.md) is the
  subset, view by view, written against formal/2026-03-02, and replaces the
  README's subset section. Running both examples through the OMG pilot
  implementation found seven kinds of deviation, all fixed: a bare `import`
  (the visibility is required), `Real` and `String` never imported, allocations
  reaching into a definition (`Hardware::CoffeeMachine::controller`), `frame`
  used as a name, undeclared message payloads, `perform use case` and
  definitions as `include`, `perform` and `satisfy` targets, and an actor
  declared before its subject. The grammar now requires what it can, a
  validator checks the rest, and tests keep the page, the grammar and the
  examples in step. Requirements stay, parsed but not drawn.
- [x] **Realization through `perform`.** A component now states the functions
  it realizes in its own body, and the implementation and physical views draw
  them in the spec's *perform actions* compartment (§7.17.6), so the relation
  between the logical view and the two that realize it needs no seventh view.
  The form was confirmed against the OMG pilot implementation first: a chain
  from a usage, `perform Functions::survey.communicate.streamVideo`, is valid,
  while a `::` path into the definition is not, so each logical file ends with
  one usage of its tree. `perform` now takes a feature chain where it took a
  bare use case name, and the validator checks that what is performed is an
  action or a use case. Implementation and physical render with
  `logical.sysml` alongside, leaving use case and logical the only
  self-contained views. In the coffee machine every leaf function is realized
  but one — nothing stores milk — and that gap is visible on the diagrams and
  asserted by a test.
- [x] **Dark glass renderer.** `--theme dark-glass` paints the same diagram
  for a dark slide: no page, so the slide shows through; glass boxes, a gloss
  across the top, a soft shadow and rounded corners; light ink and lines, with
  one cyan accent on «include» and the interface names on a wire; and no
  frame, since a slide carries its own title. SVG, not PNG — PowerPoint,
  Keynote and Slides all insert SVG, which stays sharp at any size and needs
  no rasterizer or bundled font. Themes are colour only: `src/render/theme.ts`
  holds both palettes, every coordinate is settled before a theme is
  consulted, and the committed example diagrams are byte-for-byte what they
  were.
- [ ] **Prepare for NPX** - Let us publish this cool tool in the right place such that it will be super easy to get started without downloading the full repo. Before we do that I would like to see a proper cleaning of package.json README and other files that will be needed for that action.
- [ ] **Imports that make names visible.** An import currently changes
  nothing: everything resolves by qualified name, and nothing else does. That
  makes ortho stricter than the spec rather than looser — a valid model that
  writes `private import Software::*;` and then `sw.video` fails to load here,
  and [LANGUAGE.md](LANGUAGE.md) has to tell people to qualify every
  cross-file reference. Once the package is on npm and people write ordinary
  SysML v2 against it, that is the first thing they will hit.
- [ ] **Declare each view with the spec's own view definitions.** The standard
  library defines `GeneralView`, `InterconnectionView`, `SequenceView`,
  `ActionFlowView` and `StateTransitionView` (§9.2.20). A model file could
  declare which of them it feeds — physical an interconnection view, process a
  sequence view, the rest general views — and the CLI could take the view from
  the model rather than from `-d`. This is the useful half of the old
  `view def` / `viewpoint` item; the other half, replacing one model file per
  view, is not wanted.
