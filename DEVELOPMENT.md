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

ortho is one Rust binary with no runtime. Users reach it only through the
commands in `src/main.rs`: `ortho render` renders every view the model
declares, `ortho lsp` gives an editor the same checks and diagrams
(`src/lsp/`), and `ortho init` writes what a project starts with
(`src/init/`). The library in `src/lib.rs` serves the tests and `xtask/`; it
is not a public API. `src/model/views.rs` says which of the six diagrams a
view is and names its file, and `src/pipeline.rs` runs that diagram's
extractor, layout and renderer on what the view exposes. A `GridView` of
requirements is no diagram: it is written as Markdown, with no layout
(`src/diagrams/requirements.rs`, `src/render/requirements.rs`).

- **The parser is ortho's own:** a lexer, a recursive-descent parser with one
  method per rule, then name resolution and checks. It is written from the OMG
  specification, not copied from the EPL-licensed pilot implementation.
  Existing parsers fell short: `sysml-v2-lsp` offers no parser API, and
  OpenSysML's does not expose what the views draw.
- **ELK** lays out the implementation, physical and deployment views, whose
  nested boxes and ports need it. It is
  [elk-rs](https://github.com/openedges/elk-rs), ELK ported to Rust and
  compiled in: no Java, no service. The other three views have a fixed shape,
  so ortho lays them out itself.
- **No renderer dependency.** SVG is written as text, so the output is
  deterministic and diffable.

## Parsing

**Name resolution** (`src/parser/scope.rs`) follows SysML v2. A name is
looked up where it is written: among the namespace's members and aliases,
what a definition inherits through `:>`, what a usage's type gives it, and
what the namespace imports; then in each namespace around it; last among the
top-level elements. Each `::` segment is a member, or a public import, of the
one before. A feature chain continues with dots, each segment a feature of
the one before: its own, its type's, or one it subsets, references or
redefines. An unnamed element answers to the name it takes on, `part :>>
engine` to `engine` and `perform a.b` to `b` (`effective_name`); any element
answers to its short name, and a quoted name loses its quotes as it is
parsed. Imports, dependency ends and flow ends are references the views
follow, so a typo in one fails, with *did you mean* for a segment of a chain
(`chain_hint`). An import of what ortho does not have, such as the standard
library, imports nothing.

**Grammar** (`src/parser/grammar.rs`, tokens in `lexer.rs`). What the views
draw has rules of its own, close to the spec's; every other definition and
usage shares one of three, which keep what they name as written. All sit in
one rule, `element`: its prefix is read once and the keyword after it picks
the rule, which stays cheap on a broken file. A file stops at its first
syntax error, which says where it is and what was expected.

**Conformance** (`src/parser/validator.rs`). After linking, the validator
checks what the grammar cannot, all listed in [LANGUAGE.md](LANGUAGE.md): no
reserved word as a name, chains that start at a usage, a `perform` of
behaviour rather than structure, a `satisfy` of a requirement, a part or
actor typed by a part definition, and a use case's subject before its actors.

**Errors and warnings.** The standard says when a model is well-formed, not
when it is complete: `perform`, `allocate` and `#refinement` are optional,
and a view is a filter on the one model (§9.2.20). ortho keeps three cases
apart:

- **An error** is what the standard forbids, or a name that resolves to
  nothing. The model does not load, and nothing is drawn.
- **A warning** is a rule of ortho's method: two views that both exist and
  disagree. The model renders anyway, so work can go in any order, and an
  agent knows what to fix. Without them, an agent told to fix an error could
  delete the line it points at, and the diagrams would look complete.
- **A missing view** is neither. Every view is optional; a check between two
  views applies once the model declares both.

The warnings are found once for the whole model (`Gaps`):

| When the model declares… | ortho warns of… |
|---|---|
| a use case view and a logical view | a use case that no function refines (`unrefined_use_cases`) |
| use cases and trees on one logical view | a tree there that refines none (`unrefined_trees`) |
| a logical view, and an implementation or physical view | a function in scope that no module or part those views draw performs (`unperformed_functions`) |
| a deployment view | an `allocate` end that neither view draws (`foreign_allocation_ends`), and software no host runs (`unhosted_software`) |
| a GridView of requirements, and an implementation or physical view | a requirement it lists that nothing satisfies, itself or through a requirement it is in (`unsatisfied_requirements`) |
| a GridView of requirements | a requirement it lists without an ID, or with another's (`unidentified_requirements`) |
| any package | a top-level package named after a standard library package, which other tools may resolve to the library's |

A function is in scope when a logical view draws it and, in a model with use
cases, its tree refines one. A view ortho cannot draw is skipped with the
reason (`why_undrawable`). Variability, `variation` and `variant` (§7.6),
SysML v2's way to say which functions one product of a range addresses, is
parsed but not yet read.

## Layout and rendering

- **Text is measured without a browser.** `src/render/text_metrics.rs` embeds
  the widths of Arial and DejaVu Sans, which the SVG's font list draws with on
  Windows, Linux and Typst, and `measure_text()` takes the wider for each
  character, so text fits whichever is drawn. Boxes are sized with it, never
  with character counts.
- **ELK's quirks** (`src/layout/elk.rs`): edges come relative to the node
  that contains them, so `org.eclipse.elk.json.edgeCoords: ROOT` is set, or
  the lines on the implementation and physical views move. Layering
  downwards, ELK reads a container's minimum size and content alignment with
  their axes swapped, so they are given swapped. elk-rs names the layered
  options `org.eclipse.elk.alg.layered.…`, not ELK's
  `org.eclipse.elk.layered.…`.
- **Declared order is kept:** `considerModelOrder` is set on the root and on
  every container, so siblings stand in the order declared wherever no
  crossing forbids it.
- **Boxes ELK would set in one row are gridded by ortho.** ELK's `layered`
  sets boxes with no line between them side by side: twelve packages stood
  4522 px across. A package that no line reaches into is a grid in declared
  order (`grid_of`), one box to ELK. More than three siblings joined the same
  way to the same box are one block too (`bundle_node`), gridded about half
  again as wide as the page, with a port for each line.
- **Deployment is packed, not layered** (`rectpacking`, one level at a time):
  with no lines, `layered` put every box in one row.
- **The physical view draws instances, not types.** It walks down from the
  exposed part, inherited parts and connections included, so a type fitted
  twice is two boxes, each wired on its own.
- **The logical view is laid out by ortho** (`src/layout/tree.rs`): a family
  of leaves listed down a spine, any other family in a row off a bus, the
  parent over its middle child. ELK put 27 functions in one row, 3322 px
  across. Use cases the view exposes stand in a row above the functions that
  depend on them.
- **The use case view is laid out by ortho** (`src/layout/use_case.rs`), as
  SysML v2 draws a use case (Part 1, Table 23, §8.2.3.25): its «subject»
  above, primary actors to the left, secondary ones to the right, and
  included use cases below, joined by «include» arrows. The subject and
  actors are the use case's parameters, so each use case draws its own and no
  lines cross; the groups fill rows as wide as landscape A4.
- **Realization, APIs and allocation are compartments.** A line from a
  function to its realizer would need both on one diagram, so a part lists
  its *perform actions*; a module's API is a *ports* compartment, while a
  square on the border is a port a wire lands on; a host lists what is
  allocated to it under *relationships*. `padding_of` makes room for them
  above nested parts.
- **Labels** for ports, wires and «import» are ELK labels, so ELK keeps room
  for them; port labels are painted after every line. Keywords get
  guillemets, names do not.
- **Flow names are placed after the layout** (`src/layout/flow_labels.rs`). A
  flow on a connection is an arrowhead on its line, with what flows beside it
  (§8.2.3.16). As ELK labels, the names took a layer each and pushed the
  survey drone off its page. Placed after, in free space clear of every box,
  port, name and line, they leave the layout as it is; an arrowhead may sit
  on a trunk lines share, never where another line crosses. Only when a name
  finds no place is the diagram laid out again with ELK keeping room for
  every name.
- **Sequence diagrams bypass ELK** (`src/render/sequence.rs`): the gaps
  between lifelines widen only where a label needs it, and there are no
  activation bars, since SysML v2 draws none.
- **The notation** is in [VIEWS.md](VIEWS.md). A box's corners and keyword
  come from what it draws (`box_of`). Each box carries the file and line that
  declare it, as `data-file` and `data-line`, so a viewer can open the line
  when the box is clicked.
- **Drawn for landscape A4** (`src/render/page.rs`): `ortho render` warns of a
  view whose 12 px names would print below 6 pt on A4 inside 10 mm margins,
  which allows about 1570 × 1077 px.
- **Themes are colour only** (`src/render/theme.rs`): every coordinate is
  fixed first, and a test asserts both themes place everything alike.

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
never the other way round. A view draws only what it exposes, so the files it
refers to supply names, not boxes.

These rules are the examples' only: [any link can be left out](VIEWS.md), and
the web shop, which has no hardware, has no physical or deployment view. The
example tests load each view with only the files it depends on, so a name
from any other fails. Both ends of an allocation are named once: one
`part sw { ... }` tree of software, in the deployment file beside the
allocations, and one usage of the whole product, which also heads the
physical view.

## Working on the code

You need Rust; `rust-toolchain.toml` pins the version, which `rustup`
installs on the first `cargo` command. The repository's own chores are Rust
too, in `xtask/`, run as `cargo xtask <task>`; `cargo xtask` alone lists
them.

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
cargo xtask zed-grammar             # the Zed highlighting, and the copies the extension and ortho init carry
```

**Changes reach `main` only by pull request.** The rule set *Protect main*,
with no bypass, forbids deleting or force-pushing `main` and merges a pull
request only once CI (`.github/workflows/ci.yml`) passes: the tests on Linux,
Windows and macOS; fmt, clippy and the generated grammars and copies up to
date; the committed diagrams matching a fresh render; OMG's models counted;
the VS Code extension's tests; and the Zed extension built, with its queries
checked against the grammar it pins. On `main`, CI then publishes the deck to
GitHub Pages. Dependabot opens grouped update pull requests each month for
the Rust crates, the Zed extension and the GitHub Actions; it leaves out the
ELK crate, since a new version moves every diagram. Vulnerabilities are
reported privately, as `SECURITY.md` says.

- **A new keyword** goes in `KEYWORDS` in `lexer.rs`, and is then reserved:
  it can no longer be a name. Run `cargo xtask vscode-grammar` and `cargo
  xtask zed-grammar` after, so both editors colour it.
- **The VS Code extension** (`editors/vscode/`) helps people check and review
  models, which agents will mostly write, so rename, completion and other
  editing help are left out. Its [README](editors/vscode/README.md) is its
  Marketplace page, for systems engineers. Press F5 for a second window on
  the coffee machine with the extension loaded; run this way it carries no
  ortho, so set **Ortho: Path** to `target/debug/ortho`. Its version is
  ortho's, which a test checks. Its grammar, `ai/`, `examples/`, LICENSE and
  icon are copies made by `cargo xtask vscode-grammar`, since `vsce` packs
  only the extension's folder; its CHANGELOG is written by hand. It is plain
  JavaScript with no dependencies and no build step: `src/extension.js` holds
  the commands, the preview and the Problems, and `media/` the preview's
  page. `npm test` (with `ORTHO_BIN` set, against the real ortho) tests what
  needs no VS Code, including how `ortho render`'s `Wrote` lines and `file
  line n: message` problems are read, so keep those stable. The README's
  screenshot is linked from GitHub at the commit that added it, so a new one
  needs the URL moved once its commit is pushed.
- **`ortho lsp`** (`src/lsp/`) is a language server over stdio, its JSON-RPC
  written by hand on `serde_json`. A folder's `.sysml` files are one model:
  every change checks it, with the open files as typed, and reports every
  file. It offers go to definition and nothing else. `tests/lsp.rs` drives it
  as an editor and a browser would.
- **The live preview** (`src/lsp/preview.rs`, its page `preview.html`) is a
  page `ortho lsp` serves on `127.0.0.1`, as Typst's Tinymist does, since Zed
  gives an extension no panel. The diagrams stay in memory until *Save all*
  is pressed. The page waits on `state` for each newer render and posts
  `open` when a box is clicked, which the server passes to the editor as
  `window/showDocument`. The page opens in the browser when a model file is
  opened and none is open; the port comes from the project's path, so a page
  left open finds the server again. It follows the model last opened, edited
  or saved, and closes with the last model file where the editor says so;
  Zed says so only when another file opens, so there it stays. Only this
  machine's browser is served: another host name, or a post from another
  page, is refused.
- **`ortho init`** (`src/init/`) writes the AI instructions, and with
  `--example` an example model, so a project needs no editor to start. It
  puts the instructions together as the VS Code extension does, from the same
  files, which `cargo xtask zed-grammar` copies into `src/init/assets/`,
  since crates.io gets only `src/`. Instructions changed by hand are kept
  unless `--force`; AGENTS.md is written only where there is none.
- **The Zed extension** (`editors/zed/`, its own Cargo workspace) is a Rust
  crate Zed builds to WebAssembly, which starts `ortho lsp` (`src/ortho.rs`),
  and the Tree-sitter queries in `languages/sysml/`. It parses with
  Nomograph's tree-sitter-sysml (MIT), pinned in `extension.toml`;
  `highlights.scm` is written by `cargo xtask zed-grammar` from the VS Code
  grammar's keyword groups, so both editors colour alike. The language is
  named *SysML v2*, as Spec42 names it, so both run on the same files. To try
  it, run *zed: install dev extension* and pick `editors/zed` (Zed on Windows
  builds on Windows, so copy the folder there from WSL), and set
  `lsp.ortho.binary.path` to `target/debug/ortho`.
- **The AI instructions**, a Claude Code skill, GitHub Copilot instructions
  and an `AGENTS.md`, are written once, in LANGUAGE.md, VIEWS.md and `ai/`:
  `WORKFLOW.md` how to build, check and review a model, `MISTAKES.md` the
  mistakes agents make with what ortho prints for each, `AGENTS.md` the short
  file. Only putting them together belongs to each way in.
  `tests/agent_files.rs` checks that each mistake draws the message shown and
  that its fix loads cleanly.
- **The examples** cover three domains, so a change that suits only one
  shows. Render them and look at them in a browser whenever rendering
  changes. `tests/views/` holds views written by hand that draw what no
  example does; their diagrams are committed and checked too.
- **OMG's models**, EPL-2.0, are fetched, not committed. `tests/omg_models.rs`
  counts how many load, now all 252, so the count changes only on purpose.
  **The standard library's names**, generated from them into
  `src/model/library.rs`, let `validator.rs` tell a library name from a typo.
- **The deck** inlines the committed diagrams, so render before `cargo xtask
  deck`; the table comparing the examples' sizes is typed by hand.
- **ELK** is pinned to an exact version in `Cargo.toml`, since a new one may
  move boxes; after an update, `cargo xtask render-examples` shows what moved.
  `tests/elk.rs` checks two small graphs against what elkjs, the JavaScript
  ELK, makes of them.

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
- Give one `SysMLInteractive.process(text)` session each example's files in
  dependency order (use case, logical, implementation, physical, deployment,
  process) and print `getIssues()` after each. Give each block of LANGUAGE.md
  and each test fixture `process(text, false)`, which leaves it out of the
  index, so none leans on another's names. Give it a broken model too: a check
  that cannot fail proves nothing.

## Releasing

`.github/workflows/release.yml` builds ortho for Linux (x86-64 and ARM,
linked statically), Windows (x86-64) and macOS (Apple silicon and Intel),
runs each on the examples where the runner can, and packs it with the README
and licence. For a tag `v` plus the version in `Cargo.toml`, it publishes
them as a GitHub release with their SHA-256 sums. Set the version in
`Cargo.toml` and `editors/vscode/package.json` by a pull request, then tag
the merged commit:

```sh
git tag v0.5.0
git push origin v0.5.0
```

Or run the Release workflow on `main` from the Actions tab with *publish*
ticked, which tags the commit itself. Each archive is named after its target
alone, such as `ortho-x86_64-unknown-linux-musl.tar.gz`, so
`releases/latest/download/<archive>` always finds the newest. A pull request
that changes the build builds everything without publishing.

The same workflow packs the VS Code extension once for each system VS Code
names, each package carrying that system's ortho, and tests the Linux
package and, on Mac runners, both Mac ones, after the extension's own
`makeExecutable`. Nothing is published unless they pass. Then, after the
GitHub release:

- **crates.io**, second, since `cargo binstall ortho` fetches the release's
  archive. A version there can be yanked but never replaced. Every run also
  packs the crate as crates.io would get it and builds it with the Rust that
  `rust-version` names.
- **The VS Code Marketplace and Open VSX**, where a version can never be
  published again; `--skip-duplicate` lets a failed run be run again. Both
  answer *Published* before a package can be installed, and for 0.3.0 the
  Marketplace dropped packages uploaded while it checked the first, so each
  job waits (`.github/scripts/await-extension.sh`) until its store serves
  every package, and fails after 30 minutes.

All three take GitHub's short-lived token by *trusted publishing*, so the
repository holds no publishing secret: crates.io in the crate's settings, the
Marketplace in a policy on the publisher `ola-coding`, and Open VSX as a
trusted publisher on `ola-coding.ortho`. Open VSX registers one only for an
extension that exists, so a new extension's first version needs a namespace
token in the secret `OVSX_PAT`, deleted afterwards.

**The Zed extension** is published by a pull request to
[zed-industries/extensions](https://github.com/zed-industries/extensions),
which carries this repository as a Git submodule and names the folder and
version in its `extensions.toml`:

```toml
[ortho]
submodule = "extensions/ortho"
path = "editors/zed"
version = "0.1.0"
```

A new version moves the submodule to the commit that sets it in
`editors/zed/extension.toml` and `Cargo.toml`. The version is the
extension's own, since it carries no ortho: it downloads ortho's latest
release, so a release of ortho reaches Zed users without one. A change to
`ortho lsp` the extension relies on is released first. Try the extension at
the commit submitted, with no `ortho` on the PATH, so it downloads ortho as a
user's would. The branch is made in `ola-coding/extensions`, with
`extensions.toml` and `.gitmodules` kept sorted. Zed's
[AI policy](https://github.com/zed-industries/extensions/blob/main/AI_POLICY.md)
lets no agent contribute: an agent may prepare the branch, but a person, who
has signed Zed's CLA, opens the pull request and writes its description and
every reply. A pull request left unanswered for three weeks is closed.

## Backlog

Done:

- [x] **One Rust binary**, released from 0.1.0 to 0.4.0 on GitHub, crates.io,
  the VS Code Marketplace and Open VSX.
- [x] **SysML v2 as the spec writes it:** a grammar from the spec, all 252 of
  OMG's models, the spec's view definitions and graphical notation, imports,
  and *did you mean* in chains. Metadata in a relationship's body and a body
  as an expression came from an outside report (#38).
- [x] **Six views**, each to an SVG named after it, readable on landscape A4,
  with text that fits on Windows, Linux and Typst, a flat blue tint, and a
  dark theme for slides.
- [x] **Checks between views**, as warnings: see *Errors and warnings*.
- [x] **Compartments** for realization, APIs and allocation.
- [x] **Item flows on the physical view** (§8.2.3.16). Their ends and payload
  resolve; `of` names any definition, as §7.16.1 lets a payload be anything.
  A segment after an action, such as a send action's `receiver` in OMG's
  models, stays unchecked, since it comes from the standard library.
- [x] **Ways in:** `--watch`, the VS Code extension, `ortho lsp` with a live
  preview in the browser, `ortho init`, and the Zed extension, tried on
  Windows with the project in WSL and on Windows itself.
- [x] **Security:** `main` protected, Dependabot, and private vulnerability
  reporting.

Open, in priority order:

- [ ] **Publish the Zed extension:** zed-industries/extensions#8349, opened on
  2026-11-15, waits for review. Try it on macOS and on a Linux desktop. When
  Nomograph fixes `dependency A::b to C;`, which their grammar reads only with
  `from` (nomograph/tree-sitter-sysml#4, reported on 2026-11-17), move the pin
  and let CI parse the web shop's `logical.sysml` too.
- [ ] **Be found by SysML v2 users.** ortho is listed in mycr0ft/awesome-sysml
  and daltskin/SysML-v2-Resources; jgsystemsconsulting/awesome-sysml-v2#5 and
  jgsystemsconsulting/awesome-mbse#30 wait for review. Then a post in the
  SysML Forum, once Zed publishes the extension. Always say SysML v2, never
  plain SysML, which reads as v1.
- [ ] **Requirements as a specification ortho writes,** rather than a
  seventh diagram, from standard SysML:
  `requirement <'REQ-001'> brewTime { doc /* ... */ }`, `@Rationale` and
  `satisfy brewTime by machine.heater;`.
  - [x] A `GridView` rendered as Markdown beside the SVGs: a section for each
    requirement, with its ID, name, text and rationale, then a traceability
    table of what satisfies each.
  - [x] Resolve what `satisfy` names, with *did you mean*; warn of a
    requirement nothing satisfies, and of one without an ID or with another's.
  - [ ] Requirements in the three examples.
  - [ ] LANGUAGE.md, VIEWS.md and the AI instructions.
  - [ ] A tab for the table in the VS Code and browser previews.
  - [ ] Later: *Verified by* (`verify`), *Derived from* (`#derivation`) and
    status (`StatusInfo`).
