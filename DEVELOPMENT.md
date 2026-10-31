# Developing ortho

How ortho works inside, and why. For using it, see [README.md](README.md).

## Pipeline

```text
.sysml text
  → parse            Langium-generated parser + custom scoping   (src/parser/)
  → model            resolved AST, cross-references linked        (src/generated/)
  → extract          one extractor per diagram type               (src/diagrams/)
  → layout           elkjs, or ortho's own for a fixed shape      (src/layout/)
  → render           hand-written SVG serialization               (src/render/)
  → .svg file
```

`src/cli.ts` renders every view the model declares, and is the only way in;
there is no programmatic API. `src/model/views.ts` says which of the six
diagrams a view is and names its file, and `src/pipeline.ts` runs that
diagram's extractor, layout and renderer on what the view exposes.

- **Langium** does cross-reference resolution and scoping, the bulk of a SysML
  parser. The grammar is written from the OMG specification, not copied from
  the EPL-licensed pilot implementation. Existing parsers were tried first:
  `sysml-v2-lsp` offers no parser API, and OpenSysML's does not expose what the
  views draw.
- **elkjs**, the Eclipse Layout Kernel in plain JavaScript, lays out the
  implementation, physical and deployment views, whose nested boxes and ports
  need it — no binary, no service. The other three have a fixed shape, so ortho
  lays them out itself.
- **No renderer dependency.** SVG is written as text: no browser, no DOM, and
  output that is deterministic and diffable.

## Parsing

**Name resolution** (`src/parser/sysml-scope.ts`) follows SysML v2. A name
is looked up where it is written, among the namespace's own members and
aliases, what a definition inherits through `:>`, what a usage's type gives
it, and what a namespace imports, then in each namespace around it, and last
among the top-level elements; each `::` segment after that is a member of the
one before, or a public import of it. A feature chain continues with dots,
each segment a feature of the one before: its own, its type's, or one of what
it subsets, references or redefines. An unnamed element answers to the name it
takes on, `part :>> engine` to engine and `perform a.b` to b (`effectiveName`);
any element answers to its short name too, and a quoted name loses its quotes
as it is parsed (`sysml-value-converter.ts`). An import's target is a
reference found the same way, which the implementation view follows for its
«import» arrows; one ortho does not have, such as the standard library's,
imports nothing (`sysml-document-validator.ts`). A dependency's ends are
references too, which the logical view follows for its dashed arrows, so a typo
in one fails.

**Grammar** (`grammar/sysml.langium`). What the views draw has alternatives
of its own, close to the spec's; every other definition and usage shares one
of three, which keep what they name as written. All of them sit in one rule,
`Element`: its prefix is read once and the keyword after it picks the
alternative, which keeps choosing among them cheap, also on a broken file.
Error recovery is off, so a file stops at its first syntax error; recovering
from one made a single broken file take seconds.

**Conformance** (`src/parser/sysml-validator.ts`). The grammar enforces what
it can. After linking, the validator checks the rules it cannot, all listed
in [LANGUAGE.md](LANGUAGE.md): no reserved word as a name, chains that start
at a usage, a `perform` of behaviour rather than structure, and a use case's
subject before its actors. One rule is only a warning, which the CLI prints
before carrying on: a top-level package named after a standard library package.
ortho finds the model's, while the pilot may find the library's, so such a model
renders here and may fail elsewhere. A model may declare any view, so one ortho
cannot draw is skipped with the reason (`whyUndrawable`).

## Layout and rendering

- **Text is measured without a browser.** `src/render/text-metrics.ts` embeds
  the widths of Arial and DejaVu Sans, which the SVG's font list draws with on
  Windows, Linux and Typst. `measureText()` takes the wider for each character,
  so text fits whichever is drawn and nobody installs a font. Boxes are sized
  with it, never with character counts.
- **Edges arrive in root coordinates.** ELK reports an edge relative to the
  node that contains it, which for a line from a box's own port to its child
  is that box. `elk-layout.ts` sets `org.eclipse.elk.json.edgeCoords: ROOT`
  (elkjs 0.12); without it the edges on the implementation and physical views
  move.
- **Containers fit their contents and their name.** A container gets its
  name's width as a minimum and centres narrower contents.
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
  across for 27 functions. A use case the view exposes stands in a row above,
  centred over the functions that depend on it, and takes no part in the tree.
- **The use case view is laid out by ortho** (`src/layout/use-case-layout.ts`):
  the subject above its use cases, as OMG draws it, and the actors below them.
  OMG draws one use case, its actors beside it; with several sharing both, only
  opposite sides keep every line clear of every box. Each use case stands
  between those it includes, so an «include» joins neighbours. ELK, layering
  left to right, spread the view out with long lines across it.
- **Realization, APIs and allocation are compartments.** An edge from a
  function to its realizer would need both on one diagram — a seventh view —
  so the part's box lists them instead; `paddingOf` adds the compartments'
  height so nested parts start below them. A module's API is a *ports*
  compartment too: a square on the border is for a port a wire lands on. A
  host lists what is allocated to it in its *relationships* compartment,
  since software drawn inside it would be its member.
- **Labels** for ports, wires and «import» are real ELK labels, so ELK
  reserves room for them; port labels are painted after every edge.
  Keywords get guillemets; names do not.
- **Sequence diagrams bypass ELK** (`src/render/sequence-renderer.ts`): gaps
  between lifelines widen only where a label needs it. A message runs from
  lifeline to lifeline, since SysML v2 draws no activation bars.
- **The notation** is described in [VIEWS.md](VIEWS.md). The frame's tab shows
  the view's definition, which is optional there, only for a view without a
  name. A box gets its corners and keyword from what it draws (`boxOf`).
- **Themes are colour only** (`src/render/theme.ts`). Every coordinate is
  fixed before a theme is consulted, and a test asserts both themes place
  everything alike. A theme adds nothing it does not paint, so the light
  diagrams stay byte-for-byte stable.

## One model file per view

The examples keep one model file per view, and **a view file names nothing
from another view**, except:

```text
logical                   →  use case
implementation, physical  →  logical
deployment                →  implementation, physical
process                   →  implementation
```

Every dependency points back towards what users need: a function tree names
the use case it refines, and a component names its functions, never the other
way round. A view draws only what it exposes, so the files it refers to supply
names, and boxes only where it exposes what they declare.

These rules are the examples' only: [any link can be left out](VIEWS.md). The
example tests load each view with only the files it depends on, so a name from
any other fails. Both ends of an allocation are named once: one
`part sw { ... }` tree of software, and one usage of the whole product, which
also heads the physical view.

## Working on the code

```sh
npm install              # also builds dist/ (prepare script)
npm run build            # langium generate + tsc → dist/
npm run typecheck        # src, tests and scripts
npm test                 # vitest
npm run render:examples  # regenerate every example's diagrams/
npm run deck             # build the pitch deck into marketing/
npm run omg-models       # fetch OMG's models, which npm test counts
npm run library-names    # list the standard library's names
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
- **OMG's models** are the example, training and validation models of OMG's
  release, EPL-2.0 and so fetched, not committed. `tests/omg-models.test.ts`
  counts how many load, so the count in it changes only on purpose. Without
  them, as on CI, it is skipped.
- **The standard library's names** are listed in `src/model/library.ts`,
  generated from the library `npm run omg-models` fetches, and
  `sysml-document-validator.ts` tells a library name from a typo by them.
- **The deck** inlines the committed example diagrams, so render before
  `npm run deck`; a missing diagram fails the build. The test count is filled
  in from `vitest list`, but the table comparing the two examples' sizes is
  typed by hand.

**Repeating the OMG pilot implementation check** from
[LANGUAGE.md](LANGUAGE.md#how-this-is-checked), without conda or Jupyter:

- Download `jupyter-sysml-kernel` from conda-forge. The `.conda` file is a zip
  holding a `.tar.zst`, which holds
  `share/jupyter/kernels/sysml/jupyter-sysml-kernel-<version>-all.jar` and the
  standard library in `sysml.library/`. It needs Java 21.
- Load the library with `SysMLInteractive.loadLibrary`. On Windows that fails,
  because the library's folder names contain spaces that reach the file system
  still URL-encoded: copy the library with the spaces replaced, and read its
  three folders from `jshell` with
  `SysMLInteractive.readAll(folder, true, ".kerml")` for the kernel libraries
  and `".sysml"` for the other two.
- Give one `SysMLInteractive.process(text)` session each example's six files
  in dependency order — use case, logical, implementation, physical,
  deployment, process — and print `getIssues()` after each. Give each block of
  LANGUAGE.md and each test fixture `process(text, false)`, which leaves it out
  of the index, so none leans on another's names. Give it a broken model too: a
  check that cannot fail proves nothing.

## Backlog

In priority order:

- [x] **Raise the Node floor** to 24; CI tests 24 and 26.
- [x] **Align the logical view's stem** with the middle child.
- [x] **Align with the SysML v2 spec:** [LANGUAGE.md](LANGUAGE.md), a stricter
  grammar and a validator.
- [x] **Realization through `perform`**, drawn as *perform actions*
  compartments.
- [x] **Dark glass theme** for slides.
- [x] **APIs on the software modules**, drawn as *ports* compartments.
- [x] **Update the dependencies:** Langium 4.4, Commander 15, Vitest 5,
  elkjs 0.12 and TypeScript 7.
- [x] **Declare each view with the spec's view definitions.**
- [x] **Imports that make names visible:** `Pkg::*` and `Pkg::Element`, public
  ones passed on.
- [x] **Read standard SysML v2:** all 252 of OMG's example, training and
  validation models load.
- [x] **Render every view a model declares**, each to an SVG named after it.
- [x] **Follow SysML v2's graphical notation.**
- [x] **Text that fits its box on Windows, Linux and Typst.**
- [x] **Tie the logical view to the use cases:** each function tree refines one.
- [x] **Warn of a package named after a standard library package.**
- [x] **Update the models** to the stricter reading of SysML v2: software usages
  named after their modules, `include x`, `interface` usages, and scenarios as
  an `occurrence def` of `item def` messages.
- [ ] **Prepare for NPX.** Publish ortho on npm, so it runs without the repo.
  First clean up `package.json`, the README and whatever else publishing
  needs. The name `ortho` is taken on npm, so publish under a scope, such as
  `@ola-coding/ortho`.
- [ ] **A `--watch` option** that renders again on every save, so an editor's
  SVG preview shows the diagram beside the text.
- [ ] **Requirements on the parts that satisfy them**, in a compartment like
  *perform actions* rather than a seventh view.
- [ ] **Item flows on the physical view** (`flow`): what passes between ports,
  not only that they connect.
