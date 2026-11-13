# Developing ortho

How ortho works inside, and why. For using it, see [README.md](README.md).

## Pipeline

```text
.sysml text
  → parse            hand-written parser, then name resolution     (src/parser/)
  → model            the tree, its references linked               (src/parser/ast.rs)
  → extract          one extractor per diagram type               (src/diagrams/)
  → layout           ELK, or ortho's own for a fixed shape        (src/layout/)
  → render           hand-written SVG serialization               (src/render/)
  → .svg file
```

ortho is written in Rust, one binary with no runtime. `src/main.rs` is the
`ortho render` command, which renders every view the model declares, and is
the only way in for users; the library in `src/lib.rs` is there for the tests
and `xtask/`, not as a public API. `src/model/views.rs` says which of the six
diagrams a view is and names its file, and `src/pipeline.rs` runs that
diagram's extractor, layout and renderer on what the view exposes.

- **The parser is ortho's own** (`src/parser/`): a lexer, a recursive-descent
  parser with one method per rule, and name resolution and checks after it.
  The grammar is written from the OMG specification, not copied from the
  EPL-licensed pilot implementation. Existing parsers were tried first:
  `sysml-v2-lsp` offers no parser API, and OpenSysML's does not expose what the
  views draw.
- **ELK**, the Eclipse Layout Kernel, lays out the implementation, physical and
  deployment views, whose nested boxes and ports need it. It is
  [elk-rs](https://github.com/openedges/elk-rs), a port of ELK to Rust compiled
  into the binary — no Java, no service. The other three have a fixed shape, so
  ortho lays them out itself.
- **No renderer dependency.** SVG is written as text: no browser, no DOM, and
  output that is deterministic and diffable.

## Parsing

**Name resolution** (`src/parser/scope.rs`) follows SysML v2. A name
is looked up where it is written, among the namespace's own members and
aliases, what a definition inherits through `:>`, what a usage's type gives
it, and what a namespace imports, then in each namespace around it, and last
among the top-level elements; each `::` segment after that is a member of the
one before, or a public import of it. A feature chain continues with dots,
each segment a feature of the one before: its own, its type's, or one of what
it subsets, references or redefines. An unnamed element answers to the name it
takes on, `part :>> engine` to engine and `perform a.b` to b (`effective_name`);
any element answers to its short name too, and a quoted name loses its quotes
as it is parsed (`unquote` in `grammar.rs`). An import's target is a
reference found the same way, which the implementation view follows for its
«import» arrows; one ortho does not have, such as the standard library's,
imports nothing (`validator.rs`). A dependency's ends are
references too, which the logical view follows for its dashed arrows, so a typo
in one fails.

**Grammar** (`src/parser/grammar.rs`, with the tokens in `lexer.rs`). What the
views draw has alternatives of its own, close to the spec's; every other
definition and usage shares one of three, which keep what they name as
written. All of them sit in one rule, `element`: its prefix is read once and
the keyword after it picks the alternative, which keeps choosing among them
cheap, also on a broken file. A file stops at its first syntax error, which
says where it is and what was expected there.

**Conformance** (`src/parser/validator.rs`). The grammar enforces what
it can. After linking, the validator checks the rules it cannot, all listed
in [LANGUAGE.md](LANGUAGE.md): no reserved word as a name, chains that start
at a usage, a `perform` of behaviour rather than structure, a part or actor
typed by a part definition, and a use case's subject before its actors. Six
rules are only warnings, which the CLI prints before carrying on:

- a top-level package named after a standard library package: ortho finds the
  model's, while the pilot may find the library's, so such a model renders
  here and may fail elsewhere;
- a function in scope that no module or part of a solution view performs
  (`unperformed_functions`, with what those views draw in `solution_owners`);
- a tree a logical view draws beside use cases that refines none
  (`unrefined_trees`);
- a use case a logical view draws that no tree refines
  (`unrefined_use_cases`);
- an `allocate` end that neither the implementation nor the physical view
  draws (`foreign_allocation_ends`);
- software that no deployment view allocates to a host (`unhosted_software`).

All but the first are rules of ortho's method between two views, found once
for the whole model (`Gaps`; see *Errors and warnings*). Told to fix an error,
an agent may delete the line the error points at; the model still loads, so
without these warnings its diagrams would look complete. A model may declare
any view, so one ortho cannot draw is skipped with the reason
(`why_undrawable`).

**Errors and warnings.** The standard says when a model is well-formed, not
when it is complete: `perform`, `allocate` and `#refinement` are all optional,
and a view is a filter on the one model (§9.2.20), so a model with only an
implementation view, for a cloud solution, is valid SysML v2. ortho keeps the
two apart:

- **An error** is what the standard forbids, or a name that resolves to
  nothing. The model does not load, and nothing is drawn.
- **A warning** is a rule of ortho's method: two views that both exist and
  disagree. The model renders anyway, so a systems engineer can work in any
  order, and an agent knows what to fix.
- **A view that is missing** is neither. Every view is optional; a check
  between two views applies only once the model declares both.

| When the model declares… | ortho warns of… |
|---|---|
| use cases and a logical view | a tree drawn beside use cases that refines none, and a use case drawn beside trees that none refines |
| a logical view, and an implementation or physical view | a function in scope that no module or part those views draw performs |
| a deployment view | an `allocate` end that is not a module of an implementation view or a part of a physical view, and a module no host runs |

A function is in scope when a logical view draws it and, in a model with use
cases, its tree refines one. A catalogue of functions kept beside the product's
trees needs no solution until a use case asks for it. SysML v2's own way to say
which functions one product of a range addresses is variability, `variation`
and `variant` (§7.6), which ortho parses but does not yet read.

## Layout and rendering

- **Text is measured without a browser.** `src/render/text_metrics.rs` embeds
  the widths of Arial and DejaVu Sans, which the SVG's font list draws with on
  Windows, Linux and Typst. `measure_text()` takes the wider for each character,
  so text fits whichever is drawn and nobody installs a font. Boxes are sized
  with it, never with character counts.
- **Edges arrive in root coordinates.** ELK reports an edge relative to the
  node that contains it, which for a line from a box's own port to its child
  is that box. `src/layout/elk.rs` sets `org.eclipse.elk.json.edgeCoords: ROOT`;
  without it the edges on the implementation and physical views move.
- **Containers fit their contents and their name.** A container gets its
  name's width as a minimum and centres narrower contents. Layering downwards,
  ELK reads a container's minimum size and content alignment with their axes
  swapped, so `elk.rs` gives them swapped.
- **Packages no import reaches are gridded by ortho.** ELK's `layered` puts
  boxes with no edge between them side by side, so twelve packages that import
  nothing from each other stood in one row, 4522 px across. A package that no
  edge or port reaches into, at any depth, is laid out by `grid_of` in
  `elk.rs` instead: its contents in a grid, in declared order, each package in
  it a grid of its own. ELK sees one box of the grid's size. Packages an
  import joins are still layered by ELK, which needs the arrows.
- **Packages that all import one package are a block.** ELK layers them side
  by side too: nine stood 3412 px across. More than three siblings joined by
  one line each to the same box, the same way, are one box to ELK
  (`bundle_node` in `elk.rs`): ortho grids them in declared order, with as many
  columns as bring the block nearest half again as wide as the page, and gives
  the box a port for each line. A line leaves a box in the bottom row straight
  down, and one from a row above runs down the gap beside its column; ELK
  routes it on from the port. Pinning siblings to layers with hidden edges
  instead set them out in a staircase.
- **Declared order is kept.** ELK reorders boxes it is free to place; the
  layered views set `considerModelOrder` (`NODES_AND_EDGES`, and
  `MODEL_ORDER` for the parts with no edge between them) on the root and on
  every container, so siblings stand in the order the model declares them
  wherever no crossing forbids it. elk-rs names these options
  `org.eclipse.elk.alg.layered.…`, not ELK's `org.eclipse.elk.layered.…`.
- **Deployment is packed, not layered.** With no edges, ELK's `layered` put
  every box in one row and `box` reorders by size, so the view uses
  `rectpacking`, one level at a time, since packing cannot lay out a
  hierarchy in one pass.
- **The physical view draws instances, not types.** It walks down from the
  exposed part, inherited parts and connections included, and
  resolves each connection from the part that declares it, so a type fitted
  twice is two boxes, each wired on its own. Drawing definitions merged the
  drone's two radios into one.
- **The logical view is laid out by ortho** (`src/layout/tree.rs`). A
  family of leaves is listed vertically off a spine; any other family is
  spread in a row off a bus, the parent over its middle child, or over the
  middle of the row when there is none. ELK put every leaf in one row: 3322 px
  across for 27 functions. A use case the view exposes stands in a row above,
  centred over the functions that depend on it, and takes no part in the tree.
- **The use case view is laid out by ortho** (`src/layout/use_case.rs`), each
  use case as SysML v2 draws one (Part 1, Table 23, §8.2.3.25): its «subject»
  above it, primary actors to its left and secondary ones to its right, and the
  use cases it includes below it, joined by «include» arrows at right angles.
  The subject and the actors are a use case's own parameters, so each group
  draws its own; groups stand in rows as wide as landscape A4, in declared
  order. One row of use cases sharing one subject and one set of actors grew a
  line per use case, and those crossed. ELK, layering left to right, spread the
  view out with long lines across it.
- **Realization, APIs and allocation are compartments.** An edge from a
  function to its realizer would need both on one diagram — a seventh view —
  so the part's box lists them instead; `padding_of` adds the compartments'
  height so nested parts start below them. A module's API is a *ports*
  compartment too: a square on the border is for a port a wire lands on. A
  host lists what is allocated to it in its *relationships* compartment,
  since software drawn inside it would be its member.
- **Labels** for ports, wires and «import» are real ELK labels, so ELK
  reserves room for them; port labels are painted after every edge.
  Keywords get guillemets; names do not.
- **Sequence diagrams bypass ELK** (`src/render/sequence.rs`): gaps
  between lifelines widen only where a label needs it. A message runs from
  lifeline to lifeline, since SysML v2 draws no activation bars.
- **The notation** is described in [VIEWS.md](VIEWS.md). The frame's tab shows
  the view's definition, which is optional there, only for a view without a
  name. A box gets its corners and keyword from what it draws (`box_of`).
- **Drawn for landscape A4** (`src/render/page.rs`). An SVG shrinks to fit its
  page, and its text with it, so `ortho render` warns of a view whose names
  would print smaller than 6 pt on landscape A4, 277 × 190 mm inside 10 mm
  margins. Names are 12 px, which is 9 pt at full size, so a diagram may be
  half as big again as the page, about 1570 × 1077 px, and still read.
- **Themes are colour only** (`src/render/theme.rs`). Every coordinate is
  fixed before a theme is consulted, and a test asserts both themes place
  everything alike. A theme adds nothing it does not paint, so the light
  diagrams stay byte-for-byte stable.

## One model file per view

The examples keep one model file per view, and **a view file names nothing
from another view**, except:

```text
logical                   →  use case
implementation            →  logical
physical                  →  logical, use case
deployment                →  implementation, physical
process                   →  implementation, use case
```

Every dependency points back towards what users need: a function tree names
the use case it refines, a component names its functions, the product
specializes the use cases' subject, and a scenario's people are their actors,
never the other way round. A view draws only what it exposes, so the files it refers to supply
names, and boxes only where it exposes what they declare.

These rules are the examples' only: [any link can be left out](VIEWS.md). The
example tests load each view with only the files it depends on, so a name from
any other fails. Both ends of an allocation are named once: one
`part sw { ... }` tree of software, in the deployment file beside the
allocations that use it, and one usage of the whole product, which also heads
the physical view. The implementation file has no package around all the
software: each layer is a package, the APIs another, and the view stands
outside them.

## Working on the code

You need Rust; `rust-toolchain.toml` pins the version, which `rustup`
installs on the first `cargo` command. The repository's own tasks are written
in Rust too, in `xtask/`, and run as `cargo xtask <task>`; `cargo xtask` alone
lists them.

```sh
cargo build                         # target/debug/ortho
cargo run -- render examples/coffee-machine -o out
cargo test                          # every test but OMG's models
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo xtask render-examples         # regenerate the examples' and tests/views/' diagrams/
cargo xtask deck                    # build the pitch deck into marketing/
cargo xtask omg-models              # fetch OMG's models, which needs Git
cargo test -- --ignored             # count how many of OMG's models load
cargo xtask library-names           # list the standard library's names
cargo xtask vscode-grammar          # the extension's grammar, LANGUAGE.md and example copies
```

CI (`.github/workflows/ci.yml`) builds and tests on Linux, Windows and
macOS, runs fmt and clippy, tests the VS Code extension, counts OMG's
models, fails if a fresh render differs from the committed diagrams or the
VS Code grammar from a fresh one, and only then, on `main`, publishes the
deck to GitHub Pages.

- **The grammar** is `src/parser/grammar.rs`, with its keywords in `lexer.rs`.
  A new keyword goes in `KEYWORDS`, which the deck highlights from too, and is
  then reserved: it can no longer be a name. Run `cargo xtask vscode-grammar`
  after, so the VS Code extension colours it.
- **The VS Code extension** is in `editors/vscode/`. AI agents will write
  most of the models, so it helps people check and review them rather than
  type them: rename, completion and other editing help are left to the agents
  and to SysIDE, since ortho's checks catch what an agent misses. Its
  [README](editors/vscode/README.md) is its page on the Marketplace, written
  for systems engineers deciding to install it, so how to work on it is here.
  Press F5 with the repository, or `editors/vscode`, open in VS Code: a
  second window opens on the coffee machine example with the extension
  loaded, and picks up your changes when you reload it. Run this way, it
  carries no ortho and runs `ortho` from the PATH, so set **Ortho: Path** to
  `target/debug/ortho` after `cargo build`. Its version is ortho's, since each
  package carries that ortho, and a test fails if `package.json` and
  `Cargo.toml` differ. Its grammar,
  `syntaxes/sysml.tmLanguage.json`, is written from `KEYWORDS`, never by hand,
  its `ai/LANGUAGE.md`, which the AI instructions carry, is copied from
  LANGUAGE.md, its `examples/`, which *Open Example* copies, from each
  example's `.sysml` files, and its LICENSE and `icon.png` from the
  repository's, since `vsce` packs only what is in the extension's folder:
  run `cargo xtask vscode-grammar` after changing any of them. Its
  CHANGELOG.md, which the Marketplace shows, is written by hand.
  Its code is plain JavaScript with no dependencies and no build step:
  `src/extension.js` holds the commands, the preview panel and the Problems,
  and `media/` the preview's page. What needs no VS Code is tested with Node
  (`npm test` in `editors/vscode`, with `ORTHO_BIN` set to run the real ortho
  too): `src/ortho.js`, which runs ortho and reads what it prints, and
  `src/instructions.js` and `src/examples.js`, the AI instructions and the
  examples offered. The README's screenshot, in `images/`, is left out of the
  package: the Marketplace shows it from GitHub, by an absolute URL, since
  `vsce` makes a relative one point at the repository's root folder. The URL
  names the commit that added the image, not `main`, so a package built
  before its branch is merged shows it too; a new screenshot needs the URL
  moved to its own commit, once that commit is pushed. Anything that changes
  what `ortho render` prints, its `Wrote` lines or its `file line n: message`
  problems, must keep those tests passing.
- **The examples** cover two different domains, so a rendering change that
  only suits one shows up. Regenerate their SVGs and look at them in a browser
  whenever rendering changes. They carry no requirements, since no view would
  show them.
- **The views written by hand** in `tests/views/` draw what no example does,
  such as unnamed functions and a deployment of nested hosts. Their diagrams
  are committed and checked like the examples', so a change to what they draw
  shows up in review.
- **OMG's models** are the example, training and validation models of OMG's
  release, EPL-2.0 and so fetched, not committed. `tests/omg_models.rs`
  counts how many load, so the count in it changes only on purpose. It is
  ignored until asked for with `cargo test -- --ignored`.
- **The standard library's names** are listed in `src/model/library.rs`,
  generated by `cargo xtask library-names` from the library `cargo xtask
  omg-models` fetches, and `validator.rs` tells a library name from a typo by
  them.
- **The deck** inlines the committed example diagrams, so render before
  `cargo xtask deck`; a missing diagram fails the build. The test count is
  filled in from `cargo test -- --list`, but the table comparing the two
  examples' sizes is typed by hand.
- **ELK** is elk-rs, pinned to an exact version in `Cargo.toml`, since a new
  one may move boxes. After an update, `cargo xtask render-examples` shows
  what moved. `tests/elk.rs` checks two small graphs against what elkjs, the
  JavaScript ELK the TypeScript version used, made of them.

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

## Releasing

`.github/workflows/release.yml` builds a binary for Linux (x86-64 and ARM,
linked statically), Windows (x86-64) and macOS (Apple silicon and Intel). Each
is run on the examples where the runner can run it, then packed with the
README and licence. For a tag, it publishes them as a GitHub release, with
their SHA-256 sums:

```sh
# Set the version in Cargo.toml and editors/vscode/package.json, commit, then:
git tag v0.2.0
git push origin v0.2.0
```

Without Git at hand, run the Release workflow on `main` from the Actions
tab, with *publish* ticked: it tags that commit `v` and the version in
`Cargo.toml` itself. Either way, the tag has to be `v` and the version in
`Cargo.toml`, or nothing is published. Each archive is named after its target alone, such as
`ortho-x86_64-unknown-linux-musl.tar.gz`, so
`releases/latest/download/<archive>` always finds the newest. A pull request
that changes the build, such as `Cargo.toml` or the workflow, builds every
binary without publishing.

The same workflow packs the VS Code extension once per system VS Code
names (`linux-x64`, `alpine-arm64`, `win32-x64`, `darwin-arm64` and so on),
each package carrying that system's binary in `bin/`, and tests the Linux
one against the ortho inside it. The two Mac packages are unpacked on Mac
runners, Apple silicon and Intel, and tested the same way, after the
extension's own `makeExecutable` rather than a `chmod`; nothing is published
unless they pass. The packages are the run's artifacts, not
part of the GitHub release: the Marketplace and Open VSX give each user the
package for their system.

After the GitHub release, the workflow publishes the same version on
[crates.io](https://crates.io/crates/ortho), for `cargo install ortho` and
`cargo binstall ortho`; binstall fetches the release's archive, which is why
crates.io comes second. A version on crates.io can be yanked but never
replaced, so a release that fails there needs a new version. Every run also
packs the crate as crates.io would get it (only `src/`, the README and the
licence) and builds it with the Rust that `rust-version` in `Cargo.toml`
names, so raising that floor is a deliberate change.

crates.io gets a short-lived token from GitHub through *trusted publishing*,
set up for `ola-coding/ortho` and `release.yml` in the crate's settings on
crates.io. The workflow still takes a token from the secret
`CARGO_REGISTRY_TOKEN` if one is set, which only the first publish needed;
there is none now.

The extension's packages go out after the GitHub release too, from the
`marketplace` and `open-vsx` jobs. A version on either can never be
published again, so a fix needs a new version; `--skip-duplicate` lets a
run that failed part-way be run again. The VS Code Marketplace takes
GitHub's short-lived token through its own *trusted publishing*: a policy
for `ola-coding/ortho` and `release.yml` on the publisher `ola-coding`, at
marketplace.visualstudio.com/manage, with no secret. Open VSX, for Cursor
and VSCodium, takes it the same way, through a trusted publisher for
`ola-coding/ortho` and `release.yml` registered on the extension
`ola-coding.ortho` at open-vsx.org/user-settings/trusted-publishers, which
only an owner of the namespace can do. Open VSX registers one only for an
extension that already exists, so a new extension's first version needs a
namespace token in the secret `OVSX_PAT`, read by a job that passes it, and
the secret deleted afterwards.

Both stores answer *Published* before a package can be installed: they
check it first, for some minutes. For 0.3.0, the Marketplace dropped the
seven packages uploaded while it checked the first, the one that created
the extension, though it answered *Published* for each. So each job then
waits, by `.github/scripts/await-extension.sh`, until its store serves the
package for every system, and fails after 30 minutes if one never comes:
run that job again, and `--skip-duplicate` publishes only what is missing.

## The move to Rust

ortho was written in TypeScript, on Langium and elkjs, and needed Node. It was
ported to Rust in November 2026 for a single binary that needs nothing else,
one stage at a time, each checked against what the TypeScript produced at that
stage before the next was started:

- **Parser:** identical trees on OMG's 252 models, our 17 files, 2,589 tricky
  constructs and 11,298 broken copies.
- **Linking:** every reference found Langium's target, with the same
  diagnostics, on the same models and 4,393 small workspaces.
- **Rendering:** byte-identical SVGs from the recorded layouts, both themes.
- **Views:** all six, for the examples and for 2,028 views made of every
  element of OMG's models and ours that a view could expose. Those without ELK
  were byte-identical; those with it identical but for rounding in the last
  digits, since elk-rs takes a few sums through a float as ELK's Java does.
  elk-rs orders two lines into one nested box the other way round in one of
  OMG's models, the Room Model, and so moves a box.

The committed diagrams were rendered again by the Rust version, which changed
one number in the last digits. Two things the TypeScript did were fixed after
the move: `end a [1];` is now an end named `a` with multiplicity 1, where it
was a syntax error, and an unnamed use case's subject error names the use
case it takes its name from, or none, where it said `'undefined'`.

## Backlog

Done, one line each; the commits and the sections above hold the detail:

- [x] **Raise the Node floor** to 24; CI tests 24 and 26.
- [x] **Align the logical view's stem** with the middle child.
- [x] **Align with the SysML v2 spec:** LANGUAGE.md, a stricter grammar.
- [x] **Realization through `perform`**, as *perform actions* compartments.
- [x] **Dark glass theme** for slides.
- [x] **APIs on the software modules**, drawn as *ports* compartments.
- [x] **Update the TypeScript version's dependencies.**
- [x] **Declare each view with the spec's view definitions.**
- [x] **Imports that make names visible:** `Pkg::*` and `Pkg::Element`.
- [x] **Read standard SysML v2:** all 252 of OMG's models load.
- [x] **Render every view a model declares**, each to an SVG named after it.
- [x] **Follow SysML v2's graphical notation.**
- [x] **Text that fits its box on Windows, Linux and Typst.**
- [x] **Tie the logical view to the use cases:** each function tree refines one.
- [x] **Warn of a package named after a standard library package.**
- [x] **Update the models** to the stricter reading of SysML v2.
- [x] **Update the colouring:** one flat blue tint on a white body.
- [x] **Move to Rust;** see [The move to Rust](#the-move-to-rust).
- [x] **Publish the first release,** `v0.1.0`, checked on each system.
- [x] **Warn of a diagram too big for landscape A4** (`src/render/page.rs`).
- [x] **Packages on several rows, in declared order.**
- [x] **Draw the use case view as SysML v2 does,** in rows, no line crossing.
- [x] **Keep wide diagrams readable on A4;** wrapping tree rows was dropped.
- [x] **A clear repository page.**
- [x] **Publish on crates.io** as `ortho`; see [Releasing](#releasing).
- [x] **A `--watch` option** that renders again on every save.
- [x] **Warn of a function nothing performs** (`unperformed_functions`).
- [x] **Warn of a tree that refines no use case** (`unrefined_trees`).
- [x] **Every function in scope has a solution;** see *Errors and warnings*.
- [x] **A deployment view adds nothing of its own** (`foreign_allocation_ends`).
- [x] **Refuse a part typed by behaviour or data.**
- [x] **Warn of software that runs nowhere** (`unhosted_software`).
- [x] **Warn of a use case no tree refines** (`unrefined_use_cases`).
- [x] **Suggest the name meant** in a chain (`chain_hint`); not for plain names.
- [x] **A VS Code extension** in `editors/vscode/`, with ortho inside, on the
  Marketplace and Open VSX; see *Working on the code* and *Releasing*.

Open, in priority order:

- [ ] **A Zed extension,** in `editors/zed/`, on the VS Code extension's
  principle: it helps check and review models, not type them. Zed gives
  extensions no panel or web view, only open proposals (zed-industries/zed
  #21208 and #53403), so there is no live preview: `ortho render --watch` in
  Zed's terminal writes the diagrams, and Zed opens them as images.
  - [ ] `ortho lsp`: ortho as a language server over stdio, with the problems
    of the whole model folder on open, change and save, and go-to-definition
    from the names it resolves; no completion or rename. Neovim, Helix, Emacs
    and JetBrains' editors can use it too.
  - [ ] Highlighting from a Tree-sitter grammar: Nomograph's tree-sitter-sysml
    (gitlab.com/nomograph/tree-sitter-sysml) if its licence and coverage
    fit, or else a small grammar of keywords written from `KEYWORDS`, as the
    VS Code grammar is.
  - [ ] The extension: a Rust crate built to WebAssembly that starts
    `ortho lsp`, with the ortho on the PATH or else the release archive for
    the user's system, which it downloads.
  - [ ] Find out whether Zed's image viewer shows an SVG again when `--watch`
    rewrites it.
  - [ ] Publish it by a pull request to zed-industries/extensions, and add
    that to [Releasing](#releasing).
- [ ] **Be found by SysML v2 users:** an entry in each SysML v2 awesome list
  (jgsystemsconsulting/awesome-sysml-v2, mycr0ft/awesome-sysml,
  daltskin/SysML-v2-Resources, jgsystemsconsulting/awesome-mbse) and a post
  in the SysML Forum, made from the maintainer's own account. Tag and
  describe ortho as SysML v2, never plain SysML, which reads as v1.
- [ ] **Item flows on the physical view** (`flow`): what passes between ports,
  not only that they connect. ortho parses every flow already.
  - [ ] Resolve a flow's ends and its `of` type, kept as written today, as
    connection ends are, so a typo gets *did you mean*; `of` must name an
    item definition.
  - [ ] A flow between two connected ports puts an arrowhead and `of Water`
    on that line; a flow with no `connect` draws a line of its own, as the
    spec draws a flow (§8.2.3).
  - [ ] Flows in the examples: water, steam, grounds and milk in the coffee
    machine, power and video in the drone. Their port defs already say which
    item goes which way.
  - [ ] 38 of OMG's model files use flows, some between action parameters:
    they must keep loading once the ends are resolved.
- [ ] **Requirements as a table ortho writes,** and on the parts that satisfy
  them, rather than a seventh diagram. They are standard SysML,
  `requirement <'R1'> brewTime { doc /* ... */ }` and
  `satisfy brewTime by machine.brewUnit;`, so SysIDE's requirements table
  edits the same files, and ortho needs no editor of its own.
  - [ ] A `GridView`, SysML's standard tabular view, rendered as a Markdown
    file beside the SVGs: each requirement's ID, name and text, and what
    satisfies it. GitHub, VS Code and Zed all show it; the VS Code preview
    gets a tab for it.
  - [ ] The requirements a part satisfies, in a compartment on its box, like
    *perform actions*.
  - [ ] Resolve what `satisfy` names, kept as written today, as for flows: an
    error with *did you mean* for a name that names nothing, and a warning
    for a requirement nothing satisfies.
  - [ ] Requirements in the examples.
