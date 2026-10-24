# Developing ortho

How ortho works inside, and why. For using it, see [README.md](README.md).

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

`src/cli.ts` renders the view the first file declares, and is the only way
in; there is no programmatic API. `src/model/views.ts` says which of the six
diagrams a view is, and `src/pipeline.ts` runs that diagram's extractor,
layout and renderer on what the view exposes.

- **Langium** does cross-reference resolution and scoping, the bulk of a SysML
  parser. The grammar is written from the OMG specification, not copied from
  the EPL-licensed pilot implementation.
- **elkjs**, the Eclipse Layout Kernel in plain JavaScript, lays out nested
  nodes with port-aware routing — no binary, no service.
- **No renderer dependency.** SVG is written as text: no browser, no DOM, and
  output that is deterministic and diffable.

## Parsing

**Name resolution** (`src/parser/sysml-scope.ts`) follows SysML v2. A name
is looked up where it is written, among the namespace's own members, what a
definition inherits through `:>` and what a package imports, then in each
namespace around it, and last among the top-level packages; each `::`
segment after that is a member of the one before, or a public import of it.
An import's path is found the same way, and the implementation view uses
that lookup for its «import» lines. A feature chain continues with dots, each
segment a feature of the previous one's type, inherited members included.

**Conformance** (`src/parser/sysml-validator.ts`). The grammar enforces what
it can. After linking, the validator checks the rules it cannot, all listed
in [LANGUAGE.md](LANGUAGE.md): no reserved word as a name, chains that start
at a usage, and a use case's subject before its actors — each found by running
the examples through the OMG pilot implementation — and a view that is one
ortho draws, exposing what that view can show.

## Layout and rendering

- **Text is measured without a browser.** `src/render/text-metrics.ts` embeds
  Arial/Helvetica advance widths, and boxes are sized with `measureText()`,
  never character counts. Arial leads the SVG font stack; the fallback, Segoe
  UI, is narrower, so an error leaves room rather than overflowing.
- **Edges arrive in root coordinates.** ELK reports an edge relative to the
  node that contains it, which for a line from a box's own port to its child
  is that box. `elk-layout.ts` sets `org.eclipse.elk.json.edgeCoords: ROOT`
  (elkjs 0.12); without it the edges on the implementation and physical views
  move.
- **Containers fit their contents and their name.** Deployment,
  implementation and physical nest boxes rather than drawing edges. A
  container gets its name's width as a minimum and centres narrower contents.
  `node3d` pads its top and right by `NODE_DEPTH`, since its depth edge is
  drawn inside its bounds.
- **Deployment is packed, not layered.** With no edges, ELK's `layered` put
  every box in one row and `box` reorders by size, so the view uses
  `rectpacking`, one level at a time, since packing cannot lay out a
  hierarchy in one pass.
- **The physical view draws instances, not types.** It walks down from the
  exposed part, inherited parts and connections included, and
  resolves each connection from the part that declares it, so a type fitted
  twice is two boxes, each wired on its own. Drawing definitions merged the
  drone's two radios into one.
- **The logical view is laid out by ortho** (`src/layout/tree-layout.ts`). A
  family of leaves is listed vertically off a spine; any other family is
  spread in a row off a bus, the parent over its middle child, or over the
  middle of the row when there is none. ELK put every leaf in one row: 3322 px
  across for 27 functions.
- **Use case lines are straight** (`src/layout/straight-edges.ts`). ELK places
  the shapes, then each line is redrawn as one segment. Its end slides round
  an ellipse 7.5° at a time until the line clears every shape by 4 px; lines
  meet an actor in the order they leave it, so they never cross there. When
  no sliding helps, the actor moves along its column, 4 px at a time, up to
  60 px, or further only to avoid crossing a line already drawn. Lines
  between use cases go first, since nudging cannot move them;
  a line nothing clears keeps ELK's route. A secondary actor's edge is
  reversed only as a layering hint.
- **Ellipse clipping.** ELK ends edges at bounding boxes, so the last segment
  is extended to the curve. A point already on or inside the ellipse is not
  clipped, since floating point would pick the far intersection, and the
  adjustment is capped at one radius.
- **Realization and APIs are compartments.** An edge from a function to its
  realizer would need both on one diagram — a seventh view — so the part's
  box lists them instead; `paddingOf` adds the compartments' height so nested
  parts start below them. A module's API is a *ports* compartment too: a
  square on the border is for a port a wire lands on.
- **Labels** for ports, wires, «include» and «import» are real ELK labels, so
  ELK reserves room for them; port labels are painted after every edge.
  Keywords get guillemets; names do not.
- **Sequence diagrams bypass ELK** (`src/render/sequence-renderer.ts`): gaps
  between lifelines widen only where a label needs it, and an activation runs
  from a message's arrival until that lifeline next sends — a heuristic that
  suits linear scenarios.
- **Headings** carry the view name and the first input's root package; every
  input path is printed small in the corner, so a diagram names its sources.
- **Themes are colour only** (`src/render/theme.ts`). Every coordinate is
  fixed before a theme is consulted, and a test asserts both themes place
  everything alike. A theme adds nothing it does not paint, so the light
  diagrams stay byte-for-byte stable.
- The renderer still draws generalization and composition markers, though no
  view uses them now.

## One model file per view

Each view is rendered from its own file, linked with the files it refers to
(the [README](README.md#the-six-views) lists them). The rule behind that is
that **a view file names nothing from another view**, except:

```text
implementation, physical  →  logical
deployment                →  implementation, physical
process                   →  implementation
```

Every dependency points towards what the system does: a component names its
functions, never the other way round. A view draws only what it exposes, so
the files it refers to supply names, never boxes. The example tests enforce
three conventions:

- `logical.sysml` names no component.
- `implementation.sysml` and `physical.sysml` never name each other;
  `deployment.sysml` only maps one onto the other.
- Both ends of an allocation are named once: one `part sw { ... }` tree of
  software, and one usage of the whole product, which also heads the physical
  view.

Use case and logical must also parse and render on their own.

## Working on the code

```sh
npm install              # also builds dist/ (prepare script)
npm run build            # langium generate + tsc → dist/
npm run typecheck        # src, tests and scripts
npm test                 # vitest
npm run render:examples  # regenerate every example's diagrams/
npm run deck             # build the pitch deck into marketing/
```

CI typechecks and tests on Node 24 and 26, fails if a fresh render differs
from the committed diagrams, and only then publishes the deck to GitHub
Pages.

- **Grammar changes** need `npm run langium:generate` (part of `build`).
  `src/generated/` is committed, so a fresh clone typechecks before building.
- **Never add `chevrotain` as a direct dependency.** Langium pins its range
  (`~13.2.0` in Langium 4.4); a direct one lets npm hoist a version that
  breaks `langium generate` with `Error: non exhaustive match`.
- **The examples** cover two different domains, so a rendering change that
  only suits one shows up. Regenerate their SVGs and look at them in a browser
  whenever rendering changes. They carry no requirements, since no view would
  show them.
- **The deck** inlines the committed example diagrams, so render before
  `npm run deck`; a missing diagram fails the build. The test count is filled
  in from `vitest list`, but the table comparing the two examples' sizes is
  typed by hand.

**Repeating the OMG pilot implementation check** from
[LANGUAGE.md](LANGUAGE.md#how-this-is-checked), without conda or Jupyter:

- Download `jupyter-sysml-kernel` from conda-forge (0.62.0 was used). The
  `.conda` file is a zip holding a `.tar.zst`, which holds
  `share/jupyter/kernels/sysml/jupyter-sysml-kernel-<version>-all.jar` and the
  standard library in `sysml.library/`. It needs Java 21.
- On Windows, `SysMLInteractive.loadLibrary` fails, because the library's
  folder names contain spaces that reach the file system still URL-encoded.
  Copy the library with the spaces replaced, and read its three folders from
  `jshell` with `SysMLInteractive.readAll(folder, true, ".kerml")` for the
  kernel libraries and `".sysml"` for the other two.
- Give one `SysMLInteractive.process(text)` session each example's six files
  in dependency order — use case, logical, implementation, physical,
  deployment, process — and print `getIssues()` after each.

## Backlog

In priority order:

- [x] **Raise the Node floor** to 24, the current LTS; CI tests 24 and 26.
- [x] **Align the logical view's stem** with the middle child.
- [x] **Align with the SysML v2 spec:** [LANGUAGE.md](LANGUAGE.md), a stricter
  grammar and a validator; both examples pass the OMG pilot implementation.
- [x] **Realization through `perform`**, drawn as *perform actions*
  compartments.
- [x] **Dark glass theme** for slides.
- [x] **APIs on the software modules**, drawn as *ports* compartments.
- [x] **Update the dependencies:** Langium 4.4, Commander 15, Vitest 5,
  elkjs 0.12 and TypeScript 7; npm audit is clean.
- [x] **Declare each view with the spec's view definitions**: each file
  declares a `GeneralView`, `InterconnectionView` or `SequenceView`, which
  names the heading and exposes the content; `-d` is gone.
- [x] **Imports that make names visible:** `Pkg::*` and `Pkg::Element`, public
  ones passed on; the examples rely on them.
- [ ] **Prepare for NPX.** Publish ortho on npm, so it runs without the repo.
  First clean up `package.json`, the README and whatever else publishing
  needs. The name `ortho` is taken on npm.
