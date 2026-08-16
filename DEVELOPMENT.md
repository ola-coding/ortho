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
  and are left to the default scope. Without this, every realization would have
  to redeclare the ports it inherits — which models a *different* port that
  merely shares a name, and is what the coffee-machine partition depends on.

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
- **Nothing on the logical view is left unconnected.** Port defs are not
  rendered at all: no edge kind in `part-definition.ts` can terminate on one, so
  they could only ever be disconnected boxes — they accounted for a third of the
  nodes on every logical view before being dropped. Their topology is already
  carried by the port markers on each part. Requirement *defs* had the same
  problem for a different reason (`satisfy` links to the requirement *usage*),
  fixed with a `typing` edge from usage to def, which completes
  part → requirement → specification. A test asserts the orphan count stays
  zero; interface contracts and their attributes want a dedicated interface view
  rather than floating boxes on this one.
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

The frame pentagon carries the **4+1 view name plus the model's root
package** (`Logical View — DroneLogical`), not UML diagram-kind jargon. The
generating `.sysml` path stays visible as small gray provenance text in the
bottom-right corner — everything-as-code means a diagram always names its
source — and the SVG `<title>` holds the combined string. The CLI's
`-d part-definition` etc. are command-line vocabulary only.

### File selection is the scoping mechanism

There is no `--scope` flag. Each render lists the files that belong on that
diagram, subject first, and all inputs are linked as one workspace. This
matters most for sequence diagrams, which render *every* message they see —
hence one scenario per file.

## Deliberate spec deviations

These are choices, not bugs:

- **No UML `extend`** for use cases. SysML v2 dropped it; scenario
  decomposition uses `include` and `perform`.
- **`import` parses but is not semantically enforced.** Everything is
  reachable by qualified name. The package diagram reads imports as
  dependency edges; nothing restricts visibility.
- **Attribute types are plain qualified names**, not cross-references —
  there is no standard library to resolve `Real`/`String` against yet.
- **Message ordering is document order**; `then` is accepted notation, not
  semantics.
- **Only usage forms** exist for allocation (`allocate`), not `allocation def`.
- The grammar is a **growing subset** of SysML v2/KerML, front-loaded to what
  the five views need. Full spec compliance is an open-ended goal, not a
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

Both carry the **same partition**, deliberately: a system level
(`requirements`, `use-cases`, `system`, plus a scenario file) over `mechanics`,
`electronics` and `software`. That is the layout a mechatronic product wants,
where mass and geometry are owned by mechanics and the power budget by
electronics. The two examples differ in subject and in scale — the aircraft has
fifteen architecture blocks against the machine's six, and pushes the layout
harder — not in structure. A change that only suits one of them shows up in the
other's diagrams immediately.

The load-bearing rule is that **dependencies point at the architecture, never
away from it**:

```text
UseCases, Mechanics, Electronics, Software  →  System
whichever packages declare a `satisfy`      →  Requirements
Requirements                                →  (nothing)
```

The coffee machine's `System` happens to be a pure sink: every requirement there
is satisfied by a domain component. The AUV's is not — `weightReq` and
`speedReq` are properties of the whole aircraft, so `System → Requirements`.
Both are correct; the invariant is the direction, not the absence of the edge.

Three conventions produce that shape, and package-diagram tests enforce all
three:

- **`system.sysml` declares blocks, not designs.** Each block carries only its
  ports. Every domain then *specializes* the block it realizes
  (`ThermoblockWaterSubsystem :> System::WaterSubsystem`), so the system level
  never references a domain. Composing concrete domain parts at system level
  would invert every one of those arrows.
- **Cross-domain `port def`s live at system level, beside the blocks that
  expose them.** A port owned by one domain would let it change a contract
  another depends on. A port used *inside* one domain stays in that domain —
  `Mechanics::CoffeePort` carries beans from hopper to grinder and is nothing
  the rest of the machine can see. (The AUV has no such private port: every
  interface it declares crosses a boundary.)
- **Domain packages never reference each other.** Each one parses and renders
  on its own against the architecture, with the other domains absent — a test
  asserts it. Note that no per-domain *view* is committed: rendering one
  produces a strict subset of the full logical view with no node of its own,
  because file selection cannot subset `system.sysml` down to the blocks a
  single domain realizes. That is the strongest argument for the `view def` /
  `viewpoint` item below.

An earlier revision put the interfaces in a separate `interfaces.sysml` to break
a `System ↔ Mechanics` cycle. The realization convention removes that cycle at
the source, so the extra package bought nothing and was folded back in.

Note that the package diagram shows *dependency*, which runs opposite to
derivation: requirements are authored first and depend on nothing, so design
points at them (`Mechanics → Requirements`) even though requirements drive
design. The derivation direction is carried by `satisfy` traces on the logical
view, not by this one. Do not "fix" the arrows to read as a V-model flow —
that would require requirements to reference design, which destroys their
independence.

Scenario packages are deliberately left off the development view. A scenario is
an instance of behaviour, not an architectural unit; its only import is the
package it exercises, and a project with twenty scenarios would put twenty leaf
nodes on the diagram all pointing at the same place. They belong on the process
view. This is a file-selection choice in `scripts/render-examples.ts`, not a
rule in the extractor.

Because `import` is not semantically enforced (see the deviations above), every
cross-package reference in that example is written as a qualified name. That is
load-bearing, not stylistic: bare names do not resolve across packages, so the
domain prefix on `System::DataPort` is what makes a domain crossing visible
in the source.

## Backlog

Grammar surface that parses but no diagram consumes yet: `perform`,
`interface def`, import wildcards. Kept deliberately as spec coverage.

Possible next steps, roughly by value:

- **`view def` / `viewpoint` elements** — SysML v2's first-class replacement
  for UML diagram kinds. Could drive both diagram scoping and frame headings
  from the model itself, replacing the file-selection convention.
- State machines, constraints/calculations, item defs (typed message
  payloads), verification cases.
- Sequence combined fragments (loop/alt) and return-message styling.
- Edge bundling or subsystem splitting for wide compositions — the example
  logical view is ~2990px wide because one part composes eleven others.
- Enforced import semantics, once the grammar is broad enough to need them.
