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
`src/cli.ts` is the CLI; `src/index.ts` the programmatic API.

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

### elkjs: edge coordinates are relative to the LCA

ELK reports edge coordinates relative to the **least common ancestor
container** of the edge's endpoints, not to the diagram root. With nested
nodes (use-case system boundaries, nested packages) this silently offsets
every edge. elkjs 0.9.3 does **not** support
`org.eclipse.elk.json.edgeCoords: ROOT` (the option is absent from the
bundle — verified, not assumed), so `src/layout/elk-layout.ts` computes the
LCA offset itself and translates edge points to root coordinates. If a future
elkjs adds the option, that code can go.

### Text measurement without a browser

`src/render/text-metrics.ts` embeds standard Arial/Helvetica advance widths
(1/1000 em, ASCII 32–126, regular and bold). Extractors size boxes from
`measureText()`, never from character counts. The SVG font stack leads with
Arial so metrics match what renders; Segoe UI (later in the stack) is
narrower, so any error leaves *extra* room rather than overflowing. This is
the deliberate alternative to a headless browser or font-file parsing.

### Rendering conventions

- **Generalization edges are laid out super → sub** so ELK's `DOWN` direction
  puts supertypes above subtypes (EA convention). The renderer compensates by
  drawing the hollow triangle at the layout *source* end, via `marker-start`
  plus `orient="auto-start-reverse"`.
- **Ellipse clipping**: ELK attaches edges to bounding boxes, so terminal
  segments of edges touching ellipse nodes are extended to the actual curve
  (quadratic intersection). Two guards matter: a point already on/inside the
  curve must not clip — floating point otherwise picks the *far* intersection
  and draws a line straight through the ellipse — and the adjustment is
  capped at one radius.
- **Nothing on the physical view is left unconnected.** Port defs are not
  rendered at all: no edge kind in `physical.ts` can terminate on one, so
  they could only ever be disconnected boxes — they accounted for a third of the
  nodes on that view before being dropped. Their topology is already carried by
  the port markers on each part. Interface defs are left out for a related
  reason: an interface names the contract a connection satisfies, so it belongs
  on the connection's label rather than in a box nothing points at. A test
  asserts the orphan count stays zero.
- **Two views draw containment instead of edges.** The deployment view nests
  software inside the three-dimensional node that hosts it and emits no edges
  at all, so a host with nothing drawn in it visibly hosts nothing. The
  implementation view nests modules inside their package the same way. Both
  rely on ELK hierarchy; `node3d` also reserves `NODE_DEPTH` of extra padding
  at the top and right, because its depth edge is drawn *inside* its own bounds.
- **The logical view has no arrowheads.** Its branches are `decomposition`
  edges, drawn as plain lines: the capability tree's direction is carried by
  the layout (parents above children), not by a marker. A test asserts no
  marker of any kind reaches that SVG.
- **Label placement**: composition role names sit at 82% along the path (near
  the part end, EA convention); stereotype labels stay at the midpoint.
  Relationship *keywords* get «guillemets»; user-supplied names (e.g. a named
  allocation) render plainly.
- **Sequence diagrams bypass ELK entirely** — lifeline/message layout is
  deterministic, so `src/render/sequence-renderer.ts` does its own: per-gap
  column spacing (gaps widen only where a message label spans them) and
  activation bars (active from a message's arrival until the lifeline next
  sends — a heuristic that reads correctly for linear scenarios).

### Diagram headings

The frame pentagon carries the **view name plus the model's root package**
(`Logical view — Functions`), not UML diagram-kind jargon. The generating
`.sysml` path stays visible as small gray provenance text in the bottom-right
corner — everything-as-code means a diagram always names its source — and the
SVG `<title>` holds the combined string. The CLI's `-d logical` etc. are
command-line vocabulary only.

### One model file per view

There is no `--scope` flag. Each view is rendered from the file that holds it,
and all inputs are linked as one workspace. Four of the six views are
self-contained; the other two are rendered with companions because their
content is a relation between files:

- **deployment** needs `implementation.sysml` and `physical.sysml`, since it
  declares nothing but the mapping between them.
- **process** needs `implementation.sysml`, since its lifelines are typed by
  the software modules.

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
  dependency edges; nothing restricts visibility.
- **Requirements are not on any view.** `requirement`, `satisfy` and their
  traces still parse, but no extractor consumes them: the six views cover
  usage, function, software, product, deployment and runtime, and none of them
  is a specification view. The grammar keeps the surface rather than losing it.
- **Realization cannot be written.** `allocate` resolves its ends through the
  `Feature` union, which excludes `ActionUsage`, so a function cannot be
  allocated to the component that realizes it. Deployment works because both
  its ends are parts. See the backlog.
- **Attribute types are plain qualified names**, not cross-references —
  there is no standard library to resolve `Real`/`String` against yet.
- **Message ordering is document order**; `then` is accepted notation, not
  semantics.
- **Only usage forms** exist for allocation (`allocate`), not `allocation def`.
- The grammar is a **growing subset** of SysML v2/KerML, front-loaded to what
  the six views need. Full spec compliance is an open-ended goal, not a
  pending task.

## Working on the code

```sh
npm install          # also builds dist/ (prepare script)
npm run build        # langium generate + tsc -p tsconfig.build.json → dist/
npm run typecheck    # noEmit check over src + tests + scripts
npm test             # vitest
npm run render:examples  # regenerate every example's diagrams/
```

Grammar changes require `npm run langium:generate` (folded into `build`),
which regenerates `src/generated/`. That directory is committed so a fresh
clone can typecheck before building.

**Toolchain trap:** never add `chevrotain` as a direct dependency. Langium
pins a specific range (`~11.0.3`); a direct `^11.0.3` lets npm hoist a newer
patch that breaks `langium generate` with a cryptic
`Error: non exhaustive match` (a chevrotain GAST-visitor error, not a Langium
bug). Let it resolve transitively. If that error ever appears, compare
`node_modules/chevrotain/package.json` against Langium's declared range first.

Test fixtures in `tests/fixtures/*.sysml` are inputs only. The real example
models are `examples/auv-system/` and `examples/coffee-machine/` — two
deliberately different domains, so a rendering change that only suits one of
them shows up immediately. Their committed SVGs should be regenerated (and
reviewed in a browser) whenever rendering changes.

Both carry the **same partition**, deliberately: one model file per view, each
named for the view it feeds. The two examples differ in subject and in scale —
the aircraft has nineteen physical parts against the machine's eleven, and a
capability tree half again as large — not in structure. A change that only
suits one of them shows up in the other's diagrams immediately.

```text
use-case.sysml   logical.sysml   implementation.sysml
                 physical.sysml  deployment.sysml   process.sysml
```

The load-bearing rule is that **a view file names nothing from another view**:

```text
use-case, logical, implementation, physical  →  (nothing)
deployment                                   →  implementation, physical
process                                      →  implementation
```

This works because the six views own distinct content rather than slicing
shared content. It is what lets four of the six parse and render entirely on
their own — a test asserts exactly that for both examples.

Three conventions produce that shape, and the example tests enforce all three:

- **`logical.sysml` names no component.** Not a thermoblock, not a control
  board, not a program: only what the system does. A capability tree that
  mentions a realization has stopped being medium-independent, and the test
  checks for the specific realization names declared in the other two files.
- **`implementation.sysml` and `physical.sysml` never reference each other.**
  Software knows nothing of boards and hardware knows nothing of programs. The
  mapping between them exists only in `deployment.sysml`, which is why that
  file declares nothing of its own.
- **The deployable parts are named once.** `implementation.sysml` declares a
  single `part sw { ... }` tree, and `deployment.sysml` allocates out of it.
  Allocation ends resolve to part *usages*, so without that tree there would be
  nothing for an `allocate` to name.

Requirements are absent from both examples. They parse, but no view consumes
them (see the deviations above), so committing them would leave model text that
no diagram can show.

Because `import` is not semantically enforced (see the deviations above), every
cross-file reference in these examples is written as a qualified name. That is
load-bearing, not stylistic: bare names do not resolve across files, so the
`Software::` and `Hardware::` prefixes in `deployment.sysml` are what make the
crossing visible in the source.

## Backlog

Grammar surface that parses but no diagram consumes yet: `perform`,
`requirement` / `satisfy`, import wildcards. Kept deliberately as spec
coverage.

Possible next steps, roughly by value:

- **`ActionUsage` in the `Feature` union** — the one change that unblocks
  realization. `allocate` resolves its ends through `Feature`, so today a
  function cannot be allocated to whatever realizes it, and the relation
  between the Logical view and the two realization views cannot be written at
  all, let alone drawn. Smallest change in this list, largest consequence.
- **`view def` / `viewpoint` elements** — SysML v2's first-class replacement
  for UML diagram kinds. Could drive both diagram scoping and frame headings
  from the model itself, replacing the one-file-per-view convention.
- **Width of the capability tree.** A tree lays every leaf on one row, so the
  AUV's logical view is ~3320px across for 27 functions. Row-wrapping deep
  branches, or an indented-list layout, would help before the tree grows again.
- State machines, constraints/calculations, item defs (typed message
  payloads), verification cases.
- Sequence combined fragments (loop/alt) and return-message styling.
- Edge bundling or subsystem splitting for wide compositions — the AUV's
  physical view is ~1980px wide because the aircraft composes eleven parts.
- Enforced import semantics, once the grammar is broad enough to need them.
- **The deck's figures are only as fresh as the committed SVGs.**
  `marketing/deck.template.html` quotes node and edge counts in its captions
  and inlines the diagrams by filename, so a rendering change means
  `npm run render:examples` *and* a pass over those captions before
  `npm run deck`. Nothing checks the numbers.
