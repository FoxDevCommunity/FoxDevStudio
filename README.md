# FoxDev Studio

A modern Visual FoxPro style IDE built with Electron, React and Fluent UI.

Milestone 1 delivers the IDE shell and the visual designers:

- **Form Designer** with a VFP-like toolbox, drag/resize/marquee selection, grid snapping, page frames, grids, containers, clipboard and undo/redo.
- **Properties Window** (All / Data / Methods / Layout / Other) with typed editors and VFP property names and defaults.
- **Method editor** (CodeMirror) opened by double-clicking a control or an event.
- **Run Form** (Ctrl+E) preview rendered with live Fluent UI controls; fired events show in the Output panel.
- **Menu Designer** with a live menu-bar preview.
- **Project Explorer** over JSON project files (`.fxproject`), forms (`.fxf`), menus (`.fxm`) and programs (`.prg`).

Milestone 2 adds the language: a FoxPro compiler and bytecode VM written in Rust and compiled to WebAssembly, plus the runtime that puts live forms on screen.

- **Compiler and VM** (`crates/foxvm`): lexer, error-recovering parser, bytecode compiler and a fiber-based interpreter with 151 built-in functions. The editor lints through the same compiler.
- **Run** a form (Ctrl+E), a program (Ctrl+D) or the project main (F5). Forms appear on the **Screen** tab and their methods execute: `THISFORM.pgfMain.Page1.lblGreeting.Caption = cMsg` updates the label as you would expect.
- **MESSAGEBOX**, **INPUTBOX** and **WAIT WINDOW** park the running program until answered, without freezing the UI.
- **Menus run**: `DO Main.fxm` installs the menu bar and its items execute their command or procedure text.
- **Command Window** (Ctrl+F2) and an **Output** window, with an event trace toggle.
- **Build App...** compiles the project to a `.fxa` bundle; **Build Executable...** wraps that bundle in a copy of the installed runtime, producing a folder with `<Name>.exe` that runs the application with no IDE.

The **data engine** has started: `USE` opens a table, `SELECT` chooses a work area, `GO` and `SKIP`
move the record pointer, and a field reads by name (`custno`, `customer.custno`, and `m.custno` for the
memory variable of the same name), memo fields included. The host owns the bytes and the VM owns the
meaning: the main process seeks and reads, and records are decoded in the VM a page at a time.
`SCAN`/`ENDSCAN` and `LOCATE`/`CONTINUE`/`FOUND()` walk a table, with the scopes (ALL, REST,
NEXT n, RECORD n) and FOR/WHILE clauses; `SET DELETED ON` makes every movement step over deleted
records. **SELECT-SQL** runs: a select list with expressions, `*`, `AS` names and the aggregates
COUNT/SUM/AVG/MIN/MAX; several sources joined by a WHERE clause or by INNER and LEFT OUTER JOIN;
WHERE, GROUP BY, ORDER BY, DISTINCT and TOP n; uncorrelated subqueries in `IN`, `NOT IN`,
`= ANY`, `= SOME` and `EXISTS`; and `INTO CURSOR` or `INTO ARRAY`, where the array may be a
property such as `THISFORM.aSamples`. A query is
compiled to nested loops over the same primitives a hand-written SCAN uses, so it reads a table
larger than memory the same way. A `.dbf` or `.dbc` opened in the IDE shows in a table browser.

Tables are written as well as read: `REPLACE` (with the scopes and FOR/WHILE), `APPEND BLANK`,
`DELETE` and `RECALL`. A record is a fixed run of bytes, so a change never moves anything - the
new field bytes go where the old ones were and the record goes back where it came from - and
APPEND sends the header's new record count with it. The browser edits the same way: double-click
a cell, or click a record number to mark it deleted.

Still to come, and reported by name: `SEEK` and `.cdx` indexes, `SET FILTER`, `HAVING`, writing
memo fields, and `ControlSource`/`RowSource` binding.

**[docs/language-reference.md](docs/language-reference.md)** lists every built-in function, with its
argument count and, for the ones that cannot run yet, why. It also lists every property of every
control with its default. It is generated from the two registries by `npm run docs:reference`, and a
test fails if it falls out of step, so it is worth consulting before assuming something is missing.

**[docs/not-supported.md](docs/not-supported.md)** is the shorter and more final list: what is refused
on purpose rather than merely unbuilt, and why. Today it is four sample forms built around Microsoft
Agent, Windows Messenger and the Media Player control.

## Table size

Visual FoxPro stops at 2 GB per table and per memo file, because it uses signed 32-bit file
offsets internally. FoxDev is designed to go past that: the data engine keeps every byte-level
operation in the main process behind a seek-and-read request, with 64-bit offsets throughout, and
never loads a table into memory. Verified on Node 24 by writing and reading a record at a
5.2 GB offset.

| | Visual FoxPro | FoxDev |
|---|---|---|
| Table file | 2 GB | 558 GB at 130-byte records, 4.4 TB at 1 KB records |
| Records per table | ~1 billion in practice | 4.29 billion (the DBF header's 32-bit count) |
| Memo file | 2 GB | 4 GB at block size 1, 274 GB at block size 64 |

The remaining ceilings are the DBF format's own 32-bit fields, not ours: the record count in the
header, and the memo block number that addresses the memo file in block-size multiples.

Two caveats worth knowing before you rely on this:

- **A table grown past 2 GB will no longer open in Visual FoxPro.** If you still work in both, that
  is a one-way door.
- **Memo files are capped at the block number times the block size**, so a table whose memo file
  uses a block size of 1 (real VFP files do; the samples in this repository are an example) is
  limited to 4 GB of memo data however large the table itself gets.

## Architecture

```
crates/foxvm         Rust: lexer, parser, compiler, bytecode, VM, builtins, wasm exports
crates/foxvm-cli     headless runner: foxvm run|check|disasm file.prg
src/wasm/foxvm       the compiled module, inlined as base64 and loaded with initSync
src/shared/runtime   React-free: object model, scheduler, host contract, bundle format
src/renderer/src/runtime  session, Screen tab, control renderers, dialogs, Command Window
src/renderer/src/player   the runtime player entry (player.html)
```

Every side effect the VM needs is *yielded* as a host request and performed while the VM is off the stack, then the fiber resumes. That is what lets a modal dialog block a FoxPro program without blocking the UI thread, and it keeps the wasm module free of re-entrancy.

## Development

```bash
npm install
npm run dev          # Electron with hot reload
npm test             # Vitest + jsdom (primary verification, no display needed)
npm run typecheck
npm run lint
npm run build        # production bundles into out/
npm run package      # installers into release/ (Linux targets under WSL2)
npm run build:wasm   # rebuild the Rust VM (runs automatically when the crate changes)
npm run test:rust    # cargo test for the compiler and VM
npm run check        # typecheck + lint + vitest + cargo test
```

Open the sample project from the welcome page: `resources/samples/HelloWorld.fxproject`.

### Continuous builds

Four workflows under `.github/workflows`:

- **CI** (`ci.yml`): every branch and pull request runs `npm run check` on Linux and Windows,
  and builds the site so a stale instruction reference is caught early. Nothing is packaged.
- **Nightly** (`nightly.yml`): every push to `main` packages the product and replaces the
  rolling **`nightly`** pre-release: the Windows installer, the Windows runtime zip, the
  `foxvm` command-line runner and the compiled VM as a wasm package, versioned
  `<version>-nightly.<date>.<sha>`. Windows only, because runner minutes are billed; run it
  by hand with `platforms` set to all three runners for a full set.
- **Release** (`release.yml`): a tag `v<version>` packages on Windows, Linux and macOS and
  publishes a release under that tag. The tag must match `package.json`, so cutting a release
  is: bump the version, commit, `git tag v0.1.0`, `git push origin v0.1.0`.
- **Site** (`site.yml`): a push to `main` touching `marketing/` deploys the website to
  Cloudflare with wrangler, as a Worker serving static assets. It needs the repository secrets
  `CLOUDFLARE_API_TOKEN` (Account > Workers Scripts > Edit) and `CLOUDFLARE_ACCOUNT_ID`.

The packages are unsigned. `scripts/version-check.mjs` is the tag check; `npm run package:ci`
is the packaging step the workflow runs after `npm run build`.

### Verifying without a screen

Tests run under jsdom and cover the designer interactions with pointer events. To capture the real window as a PNG:

```bash
FOXDEV_OPEN=$PWD/resources/samples/HelloWorld.fxproject FOXDEV_SCREENSHOT=/tmp/foxdev.png npm run dev
```

`FOXDEV_OPEN` (or a `.fxproject` command-line argument) opens a project and its main item at startup; `FOXDEV_SCREENSHOT` saves the window after `FOXDEV_SCREENSHOT_DELAY` ms (default 2500) and exits.

To run a built application instead of the IDE, pass a bundle: `FOXDEV_PLAY=/path/app.fxa npm run dev`, or `FoxDevStudio.exe --play app.fxa`. A packaged application carries its bundle at `resources/app.fxa` and starts in player mode automatically.

## File formats

All documents are JSON with a `$schema` and `version` field. Forms keep Visual FoxPro property names and store only values that differ from the defaults, so a later `.scx/.vcx` importer maps directly onto them. See `src/shared` for the schemas, and `resources/samples` for examples.

## Layout

- `src/main`: Electron main process (path-guarded file access, dialogs, recent projects).
- `src/preload`: sandboxed bridge exposing the typed `FoxDevApi`.
- `src/shared`: document schemas, control registry, path helpers; no Electron or React imports.
- `src/renderer/src`: React app: `designer/`, `preview/`, `editor/`, `menu-designer/`, `explorer/`, `shell/`, `stores/`.
- `tests`: Vitest suites mirroring `src`.

## The website

`marketing/` is the public site and the documentation, a separate Astro package: `cd marketing && npm install && npm run dev`. Its own README says how the pages are arranged and how to add one.
