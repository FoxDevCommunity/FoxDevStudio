---
layout: ../../layouts/DocsLayout.astro
title: Required tooling
kicker: Start here
description: What a machine needs to run FoxDev Studio, and what it needs to build the IDE, the virtual machine and the native pieces from source.
---

## To run the IDE

Nothing beyond the package. The installer carries Electron, the compiled WebAssembly module and
the native pieces, and the runtime a built application ships with is the same set of files.

Two of the pieces are Windows-only, because what they reach only exists there, and on another
platform the runtime says so by name instead of failing as something else:

| Piece | What it is for | Where it works |
| --- | --- | --- |
| `foxole.node` | COM automation: `CREATEOBJECT("Excel.Application")` and everything like it, through the `foxole` addon. | Windows |
| `fllhost.exe` | `SET LIBRARY TO`: a 32-bit process that holds a Visual FoxPro `.fll` and answers calls from the 64-bit runtime. | Windows, and only where the Visual C++ 7.1 runtimes beside it can load. |
| `koffi` | `DECLARE ... DLL`: a 64-bit library called in the same process. | Everywhere the library itself runs. |

Everything else - the compiler, the VM, the data engine, the designers, the HTTP server - is the
same on every platform.

## To build from source

The repository is `github.com/FoxDevCommunity/FoxDevStudio`. The build is driven by npm scripts,
and each native piece is rebuilt only when its sources are newer than its output, so a checkout
that has not touched Rust builds nothing in Rust.

### Always

| Tool | Version | Why |
| --- | --- | --- |
| Node.js | 24 | The IDE is Electron 44 and the scripts are ES modules. The data engine's 64-bit offsets were verified on Node 24. |
| npm | 11 | Ships with Node. `npm install` fetches `wasm-pack` as a package, so there is nothing else to fetch for the wasm build. |
| Rust | stable, with the `wasm32-unknown-unknown` target | `rust-toolchain.toml` pins the channel and names the target; `rustup` reads it and installs both. The crate edition is 2024. |

### On Windows, for the native pieces

| Tool | Why |
| --- | --- |
| Visual Studio with the "Desktop development with C++" workload, x86 and x64 tools | `fllhost.exe` is built for x86 with `cl.exe`, because every Visual FoxPro library is a 32-bit image whatever the machine is. `foxole.node` is a Rust cdylib built for x64. The build finds Visual Studio through `vswhere` and asks `vcvarsall.bat` for the x86 environment, so no shell setup is needed. |
| Visual FoxPro 9 | Optional. Where `vfp9.exe` is present, the tests can ask it for the answers the runtime is measured against, and the library host can be built against Microsoft's own API samples. Without it the checked-in answers are used. |

Without Visual C++, the two native builds print what they are skipping and exit cleanly. The
IDE, the VM and every test that does not need COM or an `.fll` run as before. GitHub's Windows
runner has the C++ tools, so the nightlies and releases carry both native pieces; the Visual
C++ 7.1 runtimes the library host ships beside it are tracked in the repository, because no
runner has a Visual FoxPro to take them from.

### The commands

```bash
npm install
npm run dev          # Electron with hot reload
npm test             # Vitest + jsdom: the primary verification, no display needed
npm run typecheck
npm run lint
npm run build        # production bundles into out/
npm run package      # installers into release/ (Linux targets under WSL2)
npm run build:wasm   # rebuild the Rust VM (automatic when the crate changes)
npm run test:rust    # cargo test for the compiler and VM
npm run check        # typecheck + lint + vitest + cargo test + the argument-form check
```

`npm run dev`, `npm run build` and `npm test` run `ensure-native` first, which is the three
`ensure-*` scripts in turn: wasm, then the COM addon, then the library host.

### What the wasm build does

`scripts/build-wasm.mjs` runs `wasm-pack build crates/foxvm --target web --release` with the
crate's `wasm` feature on, which turns on the `wasm-bindgen` exports. The module it produces is
then inlined as base64 into `src/wasm/foxvm/generated/foxvm_wasm.ts`, and the renderer
instantiates it with `initSync` rather than fetching it. That is deliberate: a packaged
application loads its pages over `file://`, and the tests run under jsdom, and neither can fetch
a `.wasm` from a URL. The release profile is `opt-level = "s"` with LTO and a single codegen
unit, and `wasm-opt` is off.

The native `cargo test` does not turn the feature on, so the same crate is tested as ordinary
Rust and shipped as wasm.

## The command-line runner

`crates/foxvm-cli` builds `foxvm`, a headless runner over the same crate:

```text
foxvm run file.prg       # run a program with a mock host: no forms, no dialogs
foxvm check file.prg     # compile and report diagnostics, as the editor's linter does
foxvm disasm file.prg    # print the bytecode, one instruction per line
foxvm audit dir          # compile every program under a folder and report what does not
```

It is the quickest way to see what the compiler makes of a program, and `disasm` is the
companion to the [bytecode page](/docs/bytecode).

## The layout, for orientation

```text
crates/foxvm                Rust: lexer, parser, compiler, bytecode, VM, builtins, data engine, wasm exports
crates/foxvm-cli            the headless runner
crates/foxole               the COM addon (Windows)
src/wasm/foxvm              the compiled module, inlined as base64
src/shared/runtime          React-free: object model, scheduler, host contract, bundle format, HTTP service
src/main                    Electron main process: path-guarded file access, dialogs, sockets, native pieces
src/preload                 the sandboxed bridge exposing the typed FoxDevApi
src/renderer/src            the React IDE: designer/, editor/, explorer/, shell/, runtime/, player/
resources/native            foxole.node and fllhost.exe, unpacked from the asar because both need a real file
tests                       Vitest suites mirroring src; goldens measured against vfp9.exe
docs                        the language reference and coverage map, both generated
```
