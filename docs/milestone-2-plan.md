# FoxDev Studio Milestone 2 Plan: FoxPro compiler + WebAssembly runtime

## Context

Milestone 1 shipped the IDE shell and designers (repo `D:\Projects\foxdevstudio`, moved from WSL to Windows on 2026-09-07). Methods are stored as FoxPro source on every object, the preview renders forms with Fluent controls, but nothing executes: `fire()` in `RuntimeContext.tsx` only logs. Milestone 2 makes the language real: a compiler and a runtime that run `resources/samples` (HelloWorld form with `THISFORM.pgfMain.Page1.lblGreeting.Caption = cMsg`, `main.prg` with `DO FORM` + `READ EVENTS`, `Main.fxm` with `CLEAR EVENTS` and `MESSAGEBOX`).

VFP itself is "compiler to p-code + runtime DLL + exe stub". The user chose the analogue:

| Topic | Decision (confirmed by user 2026-09-07) |
|---|---|
| Runtime | **WebAssembly VM written in Rust** (wasm-bindgen), driven from the React renderer over a JS bridge. The compiler lives in the same crate so the IDE gets diagnostics from it and `&macro`/`EVALUATE()` can compile at runtime. |
| Execution | **Bytecode interpreter** (serializable IR, VFP p-code analogue) designed for breakpoints/stepping later. |
| Distribution | **Runtime player + packaged project**: a `.fxa` bundle (compiled project) loaded by a player mode of the Electron app; "Build Executable" emits a folder with `<AppName>.exe` + `resources/app.fxa`. |
| Data engine | **None in M2.** Define the data-session seam only; USE/SELECT/SQL etc. compile to a "not available" diagnostic. |

Verified constraints that shape the design:
- `src/renderer/index.html` CSP is `script-src 'self'`, and wasm needs `'wasm-unsafe-eval'`. Production loads via `file://`, so the `.wasm` cannot be `fetch`ed; it is inlined as base64 and instantiated with `initSync`.
- wasm-bindgen exported structs are **not re-entrant** (calling an export while one is on the stack throws). jsdom fires `focus` synchronously, so `SetFocus()` -> `GotFocus` handler would re-enter. Therefore: **property reads are synchronous imports; every side effect is a yielded host request** handled by JS while the VM is off the stack. This also gives MESSAGEBOX / modal DO FORM / READ EVENTS without threads or SharedArrayBuffer.
- `HelloWorld.fxp` main is the **form**, not `main.prg`: "Run Main" must do `DO FORM <main>` + implicit `READ EVENTS`.
- `src/shared` is compiled by both node and web tsconfigs and eslint-banned from React/Electron: object model, scheduler, bundle and host types go there; wasm loader and React bindings stay in the renderer.
- Toolchain on this machine: Rust 1.98 (MSVC target only, no wasm32 target, no wasm-bindgen CLI), Node 24 via fnm, npm. Phase 0 installs the missing pieces (needs the user's consent at execution time).
- `tests/preview/preview.test.tsx` asserts "code does not run yet" behaviour; it is replaced, not patched.

## Architecture

```
Cargo.toml (workspace), rust-toolchain.toml
crates/foxvm/            Rust lib+cdylib: lexer, ast, parser, compiler, bytecode, value, vm, builtins/, host, wasm
crates/foxvm-cli/        Rust bin: `foxvm run file.prg` headless (golden tests, debugging)
scripts/build-wasm.mjs   wasm-pack build -> src/wasm/foxvm/generated/ + base64 module; ensure-wasm.mjs (pretest/predev/prebuild)
src/wasm/foxvm/          loader.ts (memoized initSync) + generated/ (gitignored)
src/shared/runtime/      React-free: values.ts, host.ts, objectModel.ts, scheduler.ts, programSource.ts, bundle.ts, dataSession.ts (seam), eventBus.ts (kept as trace channel)
src/renderer/src/runtime/  vmBridge.ts, session.ts (zustand), RuntimeDesktop.tsx, FormWindow.tsx, RuntimeControl.tsx, controls/ (moved from preview/runtime), dialogs/, useRuntimeObject.ts, projectSource.ts
src/renderer/src/player/   PlayerApp.tsx + src/renderer/player.html (player mode entry)
src/main/services/buildService.ts, playerService.ts
```

Runtime data flow: DOM event -> control renderer -> `desktop.dispatch(obj, 'Click')` -> `vm.start_method(...)` returns a fiber -> `Scheduler.drive` loops `vm.step(fiber)`; each `HostRequest` is performed by JS against the object model / dialogs / files, then `vm.resume(fiber, value)`. React subscribes to `RuntimeObject` instances via `useSyncExternalStore`.

### Rust crate (`crates/foxvm`)

- `Cargo.toml`: `crate-type = ["cdylib","rlib"]`; deps `wasm-bindgen` (exact-pinned), `serde`, `serde-wasm-bindgen`, `postcard`, `js-sys`; feature `wasm` gates the bindings so native `cargo test` needs no wasm tooling; `[profile.wasm-release] opt-level="s", lto=true, panic="abort"`.
- `lexer.rs`: `lex(src) -> Result<Vec<Token>, Diagnostic>`; handles `*`/`NOTE`/`&&` comments, `;` continuation, `"..." '...' [...]` strings, `{^2024-01-31}` dates, `.T. .F. .NULL. .AND. .OR. .NOT.`, `&`.
- `ast.rs` / `parser.rs`: `parse_program`, `parse_method(src, implicit_params)`, `parse_expression`; **error-recovering** (resync at line starts) so the linter shows several errors; `Diagnostic { line, col, end_line, end_col, message, severity }`. Unknown command verbs become a diagnostic ("not supported in FoxDev runtime"), not a silent no-op.
- `compiler.rs`: `compile_program(src, name)`, `compile_form(name, methods: [{object_path, event, params, source}])` (one module per form, methods table keyed `"OBJPATH.EVENT"`), `compile_snippet` (menu command/procedure text, EXECSCRIPT), `compile_expression` (EVALUATE, `&macro`, skipFor). Method bodies get the event's `EventMeta.params` as implicit LPARAMETERS.
- `bytecode.rs`: `Module { name, kind, consts, names (upper-cased), funcs: Vec<FuncProto>, methods }`, `FuncProto { name, params, locals (slot->name, for dynamic lookups), code: Vec<Instr>, lines: (pc,line) run-length, handlers }`; postcard encoding behind a `FXVM` + u16 version header; version mismatch -> "rebuild". Stack machine with a fat `Instr` enum: `Stmt(line)` at every statement boundary (line tracking + future breakpoints), `Const/True/False/Null/Pop/Dup`, `LoadLocal/StoreLocal(slot)`, `LoadName/StoreName(nidx)` (dynamic scope), `DeclLocal/DeclPrivate/DeclPublic/Release`, `Dim/LoadIndex/StoreIndex/IndexOrCall`, arithmetic/comparison ops (`Eq` obeys SET EXACT, `ExactEq` for `==`, `Contains` for `$`), `Jump*` incl. keep-variants for AND/OR/IIF, `ForInit/ForTest/ForNext` (bounds evaluated once), `CallBuiltin(bid,argc)`, `CallUser`, `Do`, `ArgRef` (by-ref `@`), `Return/ReturnValue`, `LoadThis/LoadThisForm/LoadScreen/GetMember/SetMember/CallMethod/PushWith/PopWith/LoadWith`, `Macro/ExecLine`, `Print/WaitWindow/ReadEvents/ClearEvents/DoForm/Quit/SetCmd/NoDefault`, `TryPush/TryPop/Throw/RaiseError`.
- `value.rs`: `Value { Null, Logical, Number(f64), Str(Rc<str>), Date, DateTime, Object(Handle), Array }`, `vartype`, `compare(a,b,exact)`, `is_empty`, display formatting per `Settings` (EXACT, DECIMALS, DATE, CENTURY).
- `vm.rs`: `Vm { modules, globals (PUBLIC), fibers, settings, breakpoints }`; `Fiber { frames, stack, state, pending, nodefault }`; `Frame { module, func, pc, locals, privates, this, with_stack, args }`. API: `load_module`, `start(module, func, this, args) -> FiberId`, `start_method(module, obj_path, event, this, args) -> Option<FiberId>`, `step(host, fiber) -> Step::{Done{value,nodefault} | Error(RtError) | Suspend(HostRequest)}`, `resume(fiber, value)`, `resume_error(fiber, RtError)` (host failure raised as a VFP error at the current line, catchable), `abort`, `abort_all`, `call_stack`. Scoping: LOCAL/LPARAMETERS are slots; PRIVATE/PARAMETERS and implicit privates live in per-frame maps; `LoadName` walks frame -> callers -> globals; unknown -> error 12 "Variable 'X' is not found". `RtError { code (VFP numbers where they exist: 12, 1734, 1925...), message, program, line }`.
- `builtins/{string,numeric,datetime,system,array,object,file}.rs`: `fn(&mut Vm, &mut dyn Host, &mut Fiber, args) -> Result<BuiltinResult::{Value|Suspend(HostRequest)}>` in a static map with arity specs (arity errors are compile diagnostics).
- `host.rs`:
  ```rust
  pub trait Host {  // synchronous, side-effect free; JS imports or MockHost in tests
      fn get_prop(&mut self, obj: Handle, name: &str) -> Result<Value, RtError>;   // Err(1734) when unknown; answers Name/Parent/Class/BaseClass too
      fn get_member(&mut self, obj: Handle, name: &str) -> Result<Member, RtError>; // Child(Handle) | Prop | Method | None
      fn object_class(&mut self, obj: Handle) -> Option<String>;                    // None = released object
      fn output(&mut self, text: &str, newline: bool);
      fn now(&mut self) -> (i32, f64); fn random(&mut self) -> f64;
      fn resolve_program(&mut self, name: &str) -> Option<u32>;                     // DO <prog>: JS returns a loaded module id or None -> LoadProgram request
  }
  pub enum HostRequest {  // side effects: yielded, never nested inside a Rust frame
      SetProp{obj,name,value}, CallMethod{obj,name,args}, DoForm{name,args,modal,name_var,linked,want_result}, ReleaseObject{obj},
      MessageBox{text,flags,title,timeout}, InputBox{..}, WaitWindow{text,nowait,timeout,clear},
      ReadEvents, ClearEvents, Quit, Cancel, DoMenu{name}, GetFile{..}, PutFile{..},
      FileRead{path}, FileWrite{path,text,append}, FileExists{path}, LoadProgram{name}, Break{line} /* reserved */,
      Data(DataRequest) /* reserved seam, unimplemented */
  }
  ```
- `wasm.rs` (`#[cfg(feature="wasm")]`): `#[wasm_bindgen] struct FoxVm` with `new(reads: JsValue)`, `load_module(&[u8]) -> u32`, static `compile_program/compile_form/compile_snippet/check(src, kind) -> JsValue {bytes, diagnostics}`, `start`, `start_method`, `step -> JsValue {state:'done'|'error'|'suspend', ...}`, `resume`, `resume_error`, `abort`, `abort_all`, `version()`. `JsHost` implements `Host` through `#[wasm_bindgen(method, structural)]` externs on the `HostReads` object, so the vitest mock is a plain object.
- **Marshalling**: plain `JsValue` primitives for values (`{ $obj: n }` for handles, `{ $date }`/`{ $dt }` for dates, `{ $arr }` for arrays), `serde-wasm-bindgen` for structured messages (requests, step results, diagnostics, compile inputs) so the TS `HostRequest` union mirrors the Rust enum, `Uint8Array` only for bytecode. Rationale: property traffic is strings/numbers/booleans (exactly `PropValue`); JSON would add parse/stringify per host call; typed-array framing is premature for a UI-bound interpreter.

### JS shared layer (`src/shared/runtime/`)

- `host.ts`: `VmValue`, `HostReads`, `HostRequest` (TS mirror), `HostRequestHandler { perform(req, ctx): VmValue | Promise<VmValue> }`, `VmLike` (the subset of `FoxVm` the scheduler needs, so it is testable with a fake).
- `scheduler.ts`: `Scheduler.drive(fiber)` loops `step`; a sync `perform` result resumes in the same loop; a Promise (MessageBox, InputBox, modal DoForm, WAIT WINDOW without NOWAIT, GetFile) is awaited. Because the wasm is off the stack during `perform`, `perform` may synchronously `dispatch` nested VFP events (ProgrammaticChange from SetProp, GotFocus from SetFocus, Load/Init during DoForm, Destroy/Unload during Release) and they run to completion before the outer resume: VFP's nested-handler model without threads. Only one fiber executes at a time; any number may be parked. `ReadEvents` parks the fiber and increments depth; `ClearEvents` sets a flag honoured when the current fiber finishes (VFP resumes the READ EVENTS program only after the handler returns), nested READ EVENTS unwind LIFO; in the player, main fiber done with depth 0 -> `onQuit`. Modal `DoForm` resolves on the form's release with `Unload`'s return value (`DO FORM ... TO var`). Errors -> `hooks.onError` returning Cancel (abort all, release forms) or Ignore (resume at next `Stmt`). Timers: `TimerService` (injectable `setInterval` for fake timers) dispatches only when not executing, else queues one tick; keeps firing while parked. `vmBridge` asserts `!inWasm` around every export call so re-entrancy is an explicit test failure.
- `objectModel.ts`: `RuntimeObject { handle, name, type, node, parent, children, get/has/set(name, value, 'program'|'interactive'), child(name), form(), path(), bindElement(el), setFocus(), subscribe/version, alive }`; `FormInstance extends RuntimeObject { module, modal, result, release() }`; `Desktop` (the `_SCREEN` analogue, one per session): `forms`, `handles`, `createForm(doc, module, opts)` (Load -> Init children-before-parent -> Show -> Activate), `releaseForm(form, {queryUnload})` (QueryUnload with NODEFAULT cancel -> Destroy parent-first -> Unload -> unregister), `dispatch` (injected by the session; logs to `eventBus` when Trace is on), `timers`, `menu`, `log`. `Desktop` implements `HostReads` (intrinsics `Name/Parent/Class/BaseClass/ControlCount/PageCount/ActivePage`, member classification from registry descriptors plus a method list `SetFocus/Refresh/Release/Show/Hide/Move`) and the `HostRequestHandler` for object requests; dialog/file requests go to a `PlatformServices` interface (IDE: Fluent dialogs + `getApi().files`; tests: scripted answers).
- `programSource.ts`: `interface ProgramSource { getProgram(name), getForm(name), getMenu(name) }`; IDE implementation compiles from open (in-memory) documents or disk with a content-hash cache; bundle implementation reads the `.fxa`. Note in the UI: Run Form uses unsaved in-memory state (differs from VFP, matches M1's Refresh expectation).
- `bundle.ts`: `AppBundle { $schema:'foxdev-app', version:1, name, vmVersion, builtAt, main:{kind,name}, programs, forms:{doc, bytecode(base64)}, menus }` with zod; `packBundle(project, sources)` aborts on diagnostics with an Output listing; `parseBundle(text)`.
- `dataSession.ts`: `interface DataSessionHost` with the M3 method surface, no implementation; every data command/function compiles to the "not available" diagnostic.

### Renderer (`src/renderer/src/runtime/`)

- `vmBridge.ts`: `loadFoxVm()` memoized (base64 -> `initSync`), `createVm(reads)`, `VmLike` adapter.
- `session.ts` (zustand): `{ status: idle|running|waiting|error, desktop, scheduler, output, start(kind:'form'|'program'|'main', target), cancel(), execute(text) }`; one session at a time, starting another asks to cancel via `dialog.message`.
- `RuntimeDesktop.tsx`: new document kind `{ kind: 'desktop' }` (single instance, replaces `preview`) hosting `FormWindow`s (Left/Top/AutoCenter), the app menu bar when a menu is installed (reuses `menu-designer/MenuPreview.tsx` with `onChoose -> session.execute(item.result.text)`, `skipFor` evaluated on open), modal overlay, `MessageBoxDialog`/`InputBoxDialog` (Fluent `Dialog`), `WaitWindowToast`, `ProgramErrorDialog` (program/line, Cancel/Ignore/Edit -> opens the method/program at the line via a new `goToLine` prop on `CodeEditor`).
- `FormWindow.tsx` replaces `preview/FormPreview.tsx` (title bar, working close box -> `releaseForm({queryUnload:true})`, client area).
- `controls/index.tsx` is `preview/runtime/index.tsx` moved: `RuntimeProps { obj, props, renderChildren }`, `useEvents(obj)` dispatches through the desktop, value writes become `obj.set('Value', v, 'interactive')`, PageFrame `ActivePage` lives in the object, focusable controls register via `ref={obj.bindElement}`, `Valid` returning `.F.` refocuses. `RuntimeControl({ obj })` uses `useRuntimeObject` (`useSyncExternalStore`).
- Removed: `preview/RuntimeContext.tsx`, `preview/FormPreview.tsx`, document kind `preview`. `eventBus.ts` stays as the trace channel.
- IDE-wide Output panel: bottom pane in `IdeLayout.tsx` (extend `SplitPane` with a `bottom` slot) fed by the session store; shows `?` output, errors, and event trace when enabled.

### IDE commands (`shell/commands/index.ts`, `MenuBar.tsx` Program menu)

`program.run` Run Form Ctrl+E (now executes), `program.doProgram` Do Program Ctrl+D (active .prg), `program.runMain` Run Main F5 (form or program main), `program.cancel` Cancel Program Shift+F5, `program.build` Build App... Ctrl+Shift+B (.fxa via save dialog), `program.buildExe` Build Executable..., `view.output` toggle, `view.commandWindow` Command Window Ctrl+F2 toggle, `program.traceEvents` checked toggle. **Command Window** (`runtime/CommandWindow.tsx`, docked under the Output panel): a one-line CodeMirror input; Enter compiles the line as a snippet through `session.execute(text)` and runs it, `?` output and errors land in the Output panel, up/down arrows walk a history kept in the session store. Works without a running program (it starts an idle session) and while one is parked in READ EVENTS. `StatusBar` shows Running/Waiting for events/Ready. Explorer context menu gains Run on forms/programs.

Editor: add `@codemirror/lint` and `@codemirror/autocomplete`; `editor/foxproService.ts` implements `LanguageService` with `[foxproLanguage, linter(async -> check()), autocompletion(keywords+functions, later form members)]`; `CodeEditor` gains `languageContext: { kind:'program'|'method', params? }` (implicit LPARAMETERS don't lint as unknown); the lint source awaits `loadFoxVm()` so editors mounted before load still work. `setLanguageService(foxproService)` at App start.

### Player and Build Executable

- Same binary, two modes. `src/renderer/player.html` -> `player/main.tsx` -> `PlayerApp.tsx` (FluentProvider + full-window `RuntimeDesktop`, `getApi().player.getBundle()` -> `session.start('main')`, no shell/designer imports). `electron.vite.config.ts` renderer gets multi-input `{ index, player }`. `src/main/index.ts`: bundle path from `FOXDEV_PLAY`, `--play <file>`, or `resources/app.fxa` next to `app.asar`; then `createMainWindow({ page:'player.html', title })`, `guard.allowDir(dirname(bundle))`, IPC `player:getBundle`. Verify dev URL `${ELECTRON_RENDERER_URL}/player.html`; fallback `?mode=player` on index.html.
- `build:exe` IPC (`src/main/services/buildService.ts`): requires `app.isPackaged`; `fs.cp` the install dir to `<outDir>/<AppName>/`, write `resources/app.fxa`, rename the exe. A separate slim runtime-only build and an end-user installer are M3. `electron-builder.yml` adds a zip/portable target and `.fxa` association.

### Language subset for M2

First cut: comments, `;` continuation, case-insensitive identifiers/keywords with 4-letter abbreviations; literals (strings in three quote styles, numbers incl. hex, `.T./.F./.NULL.`, `{^date}`, `{}`); VFP operator precedence, string `+`/`-`, `$`, `=` vs `==` (SET EXACT), NULL propagation; `=`, `STORE`, `LOCAL/PRIVATE/PUBLIC` (with arrays), `LPARAMETERS/PARAMETERS`, `IF/ELSE/ENDIF`, `DO CASE`, `DO WHILE`, `FOR/ENDFOR|NEXT [STEP]`, `EXIT`, `LOOP`, `RETURN`, `PROCEDURE/FUNCTION`, `DO proc [WITH] [IN]`, user functions incl. `@` by-ref, `? / ??`, `WAIT WINDOW [NOWAIT] [TIMEOUT] [CLEAR]`, `DO FORM name [NAME var [LINKED]] [WITH] [TO var]`, `READ EVENTS`, `CLEAR EVENTS`, `RELEASE`, `QUIT`, `CANCEL`, `WITH/ENDWITH`, `DIMENSION/DECLARE`, `=expr` and bare method calls, `NODEFAULT`, `SET EXACT|DECIMALS|CENTURY|DATE|TALK|SAFETY|ESCAPE` (others warn, no-op), `THIS/THISFORM/_SCREEN`, member access and method calls, `a[1]`/`a(1)`, `IIF`, `#DEFINE`, `TEXT/ENDTEXT [TEXTMERGE]`.
Round 2 (Phase 6): `FOR EACH`, `TRY/CATCH/FINALLY/THROW`, `&macro` (expression and whole line), `EVALUATE()`, `EXECSCRIPT()`, `ON ERROR` + `ERROR()/MESSAGE()/LINENO()/PROGRAM()`, `ON KEY LABEL` (form KeyPress), `DO menu`.
Built-ins first cut: string (`ALLTRIM LTRIM RTRIM TRIM UPPER LOWER PROPER LEN SUBSTR LEFT RIGHT AT ATC RAT PADL PADR PADC SPACE REPLICATE STRTRAN STUFF CHRTRAN CHR ASC STR VAL TRANSFORM OCCURS ISALPHA ISDIGIT ISBLANK EMPTY INLIST BETWEEN IIF EVL NVL ISNULL TYPE VARTYPE GETWORDCOUNT GETWORDNUM STREXTRACT`), numeric (`INT ROUND ABS MAX MIN MOD SQRT CEILING FLOOR RAND`), date (`DATE DATETIME TIME YEAR MONTH DAY DOW CDOW CMONTH DTOC CTOD DTOS TTOC CTOT TTOD SECONDS GOMONTH`), system (`MESSAGEBOX INPUTBOX VERSION OS SET PCOUNT PARAMETERS PROGRAM SYS(2015,3)`), arrays (`ALEN ASCAN ADEL AINS ASORT ACOPY`), files (`FILE FILETOSTR STRTOFILE GETFILE PUTFILE JUSTFNAME JUSTSTEM JUSTEXT JUSTPATH FORCEEXT ADDBS`), objects (`CREATEOBJECT("Empty") ADDPROPERTY PEMSTATUS`).
Deferred with a clear diagnostic: all data commands/functions, `DEFINE CLASS`, `NEWOBJECT` with class libs, SQL, reports, `_VFP`, `DECLARE DLL`, `ON SHUTDOWN`, `KEYBOARD`, exact decimal arithmetic (Numeric is f64; DECIMALS is display-only), Currency, Varbinary, code pages. Also fix `foxproKeywords.ts`: move `SETFOCUS/REFRESH/RELEASE/SHOW/HIDE` out of the functions list.

## Toolchain and build (Windows)

- One-time, at execution start (ask first): `rustup target add wasm32-unknown-unknown`; `npm i -D wasm-pack` (npm package ships the Windows binary and fetches the wasm-bindgen CLI matching the pinned crate). Fallback: `cargo install wasm-bindgen-cli --version =<pinned> --locked` and drive `cargo build --target wasm32-unknown-unknown --profile wasm-release` + `wasm-bindgen --target web` from the script.
- `scripts/build-wasm.mjs` (`npm run build:wasm`): `wasm-pack build crates/foxvm --target web --release --out-dir ../../src/wasm/foxvm/generated --out-name foxvm`, remove wasm-pack's `.gitignore`, write `generated/foxvm_wasm.ts` with the base64 module. `scripts/ensure-wasm.mjs` rebuilds only when `crates/foxvm/src/**` or `Cargo.lock` is newer; wired as `pretest`, `predev`, `prebuild`. `generated/` is gitignored (committing artifacts is the documented alternative for a Rust-less checkout).
- `package.json`: `test:rust` = `cargo test --workspace`, `check` = typecheck + lint + test + test:rust. `tsconfig.web.json` includes `src/wasm/**`; eslint ignores `generated`; vitest coverage adds `src/shared/runtime/**` and `src/renderer/src/runtime/**`; `rust-toolchain.toml` pins stable with the wasm32 target.
- CSP in `index.html` and `player.html`: `script-src 'self' 'wasm-unsafe-eval'`.

## Phases (each ends green on `npm run check`)

**Phase 0: Toolchain + wasm smoke.** Workspace `Cargo.toml`, `rust-toolchain.toml`, `crates/foxvm` exporting `version()` and `add()`, build scripts, `src/wasm/foxvm/loader.ts`, CSP, `.gitignore`, package scripts. Tests: one `cargo test`; `tests/wasm/load.test.ts` (`loadFoxVm()` resolves, `version()` equals the crate version); FOXDEV_SCREENSHOT run still boots with wasm loaded in an `App.tsx` effect.

**Phase 1: Lexer, parser, diagnostics, editor lint.** `lexer.rs`, `ast.rs`, `parser.rs`, `diagnostics.rs`, `wasm.rs::check`; `editor/foxproService.ts`, `CodeEditor` `languageContext`, `App.tsx` `setLanguageService`. Tests: `crates/foxvm/tests/parse_*.rs` inline snapshots for every first-cut construct plus recovery cases (`IF` without `ENDIF` reports the IF line; unknown verb; continuation; nested procedures); all sample sources parse with zero diagnostics; `tests/editor/lint.test.tsx` renders `ProgramDocument` with `IF x` and finds a lint marker; `tests/wasm/check.test.ts` asserts diagnostic shape.

**Phase 2: Compiler, VM core, builtins, CLI.** `compiler.rs`, `bytecode.rs`, `value.rs`, `vm.rs`, `builtins/*`, `host.rs` trait + `MockHost` (records output, scripted dialog answers, small object tree), `crates/foxvm-cli`. Tests: golden programs `crates/foxvm/tests/programs/*.prg` + `.expected` (precedence, SET EXACT, NULL, LOCAL vs PRIVATE dynamic scope, PCOUNT, by-ref, recursion, DO CASE, loops with EXIT/LOOP, FOR bounds evaluated once, arrays, dates, STR/TRANSFORM/VAL, TEXT/ENDTEXT, `#DEFINE`, error cases with expected VFP code and line); bytecode round-trip; `RtError.line` correctness; `tests/wasm/run.test.ts` compiles `? "hi"` through the bridge with a fake `HostReads` and sees `output` called.

**Phase 3: Host requests, fibers, scheduler.** Suspend/resume in `vm.rs`, request-emitting builtins (MESSAGEBOX, INPUTBOX, WAIT, READ/CLEAR EVENTS, DO FORM), `src/shared/runtime/{host,scheduler,values}.ts`, `runtime/vmBridge.ts`. Tests: Rust `tests/fibers.rs` (MESSAGEBOX suspends and the resumed value flows into `IF MESSAGEBOX(..) = 6`; READ EVENTS parks and another fiber's CLEAR EVENTS resumes it after that fiber ends; nested fiber while another is suspended; abort mid-suspend; `resume_error` is catchable); `tests/runtime/scheduler.test.ts` with a fake VM (sync requests in one tick, promise requests awaited, nested dispatch inside `perform`, timer tick queued while running, cancel drops pending resumptions).

**Phase 4: Object model + form method compilation (no React).** `objectModel.ts`, `programSource.ts`, `compile_form`, `start_method`, registry lookups. Tests: Rust `tests/form_methods.rs` (methods table, implicit params, `THISFORM.a.b.Caption = x` -> GetMember/GetMember/SetMember against MockHost); `tests/runtime/objectModel.test.ts` with the real wasm on `HelloWorld.fxf`: `createForm` runs Init and SetFocus targets `txtName`; set `txtName.Value='Jorge'` interactively, `chkLoud=true`, dispatch `cmdSayHi.Click` -> `lblGreeting.Caption === 'HELLO, JORGE!'`; `cmdClose.Click` releases with Destroy/Unload order; ProgrammaticChange only on program sets; Timer dispatch under fake timers; `Parent`/`Name`; unknown property -> 1734 with the right line.

**Phase 5: React integration + IDE commands (acceptance).** `runtime/{session,RuntimeDesktop,FormWindow,RuntimeControl,controls,dialogs,useRuntimeObject,projectSource,CommandWindow}`, `documentsStore` (`desktop` replaces `preview`), `DocumentArea`, `IdeLayout`/`SplitPane` bottom Output, `OutputPanel`, `StatusBar`, `MenuBar`, commands, `ProjectExplorer` Run, `CodeEditor.goToLine`, delete `preview/`. Tests: `tests/runtime/helloWorld.test.tsx` (memory api seeded with the sample files; open project; `runCommand('program.runMain')`; dialog "Hello, World" appears; `txtName` focused; type Jorge, check Shout, click Say Hi -> `HELLO, JORGE!`; Close -> gone, status Ready; `program.doProgram` on `main.prg` -> "Waiting for events"; `execute('CLEAR EVENTS')` ends it); `tests/runtime/dialogs.test.tsx` (MESSAGEBOX Yes/No resumes 7 on No; WAIT WINDOW toast; INPUTBOX); `tests/runtime/errors.test.tsx` ("Variable 'Y' is not found", `Program: cmdSayHi.Click Line: 1`, Cancel ends session, Edit opens the method tab); `tests/runtime/commandWindow.test.tsx` (type `? 1 + 1` and Enter -> Output shows `2`; `DO FORM HelloWorld` opens the form; up arrow recalls the previous line; a syntax error shows in Output without a dialog); `tests/preview/preview.test.tsx` rewritten as `tests/runtime/controls.test.tsx` (every ControlType renders; Visible/Enabled honoured; trace lines when enabled).

**Phase 6: Language round 2 + menus.** Parser/compiler/VM additions, `builtins/system.rs` (EVALUATE/EXECSCRIPT/ON ERROR), desktop menu bar, `menuRuntime.ts` (setup/cleanup, skipFor, hotkeys). Tests: golden programs for FOR EACH, TRY/CATCH, macros, ON ERROR; `tests/runtime/menu.test.tsx` (`DO Main.fxm` shows File/Help; About -> MESSAGEBOX "HelloWorld 1.0"; Exit -> CLEAR EVENTS ends the session; modal `DO FORM x TO r` returns Unload's value).

**Phase 7: Bundle, player, Build Executable.** `bundle.ts`, `bundleSource.ts`, `program.build`, `player.html` + `player/*`, multi-input vite config, main-process mode switch and `player:getBundle`/`build:exe` IPC (channels, api, preload, memoryApi), `buildService.ts`, `electron-builder.yml`, README. Tests: `tests/shared/bundle.test.ts` (round trip; diagnostics abort; sample project bundle has form/menu/program); `tests/player/player.test.tsx` (`PlayerApp` with the packed sample: form appears, Say Hi works, Close with no READ EVENTS -> `onQuit`); `tests/main/build.test.ts` (fake install dir: copies, writes `resources/app.fxa`, renames exe, refuses when not packaged); `FOXDEV_PLAY=<bundle> FOXDEV_SCREENSHOT=...` shows the form.

**Phase 8: Polish.** Member autocompletion from the active form, breakpoint API exposed (`set_breakpoint`, `Suspend(Break)` -> pause with Continue), VFP error-code table review, README and memory update.

## Verification

- Per phase: `npm run check` (typecheck, lint, vitest, `cargo test`). Targeted: `cargo test -p foxvm`, `npx vitest run tests/wasm`, `tests/runtime`, `tests/editor`, `tests/player`.
- End-to-end without a screen: the Phase 5 HelloWorld test is the acceptance test (type a name, click Say Hi, assert the label; Close releases the form; main.prg parks in READ EVENTS; CLEAR EVENTS ends it).
- Real window: `FOXDEV_OPEN=resources/samples/HelloWorld.fxp FOXDEV_SCREENSHOT=<png> npm run dev` after Phase 5, and `FOXDEV_PLAY=<bundle>` after Phase 7; a manual pass on Windows: F5, interact, Cancel, Build App, Build Executable, run the produced exe.
- Headless language checks: `cargo run -p foxvm-cli -- run crates/foxvm/tests/programs/<x>.prg`.

## Risks and mitigations

- **wasm in vitest/jsdom**: Node's `WebAssembly`/`TextEncoder` stay available; base64 loader avoids fs and fetch. If cross-realm `encodeInto` errors appear, alias Node's `TextEncoder` in `tests/setup.ts`.
- **CSP / file:// / asar**: solved by inlining the module (+33% size, ~1.5 MB string); `?url` streaming can replace it later behind the same `loadFoxVm()`.
- **Re-entrancy**: eliminated structurally (reads sync, effects yielded) and asserted in `vmBridge`; parked promises check `session.generation` before resuming after cancel.
- **Marshalling cost**: heavy string work stays in Rust; member chains are cheap integer-returning `getMember` imports; cache child handles per (handle, name) if profiling ever demands.
- **wasm-bindgen crate/CLI drift**: exact-pin the crate; wasm-pack fetches the matching CLI; `ensure-wasm` prints both versions on mismatch.
- **Fidelity creep**: the first-cut list is the contract; everything else is a clear diagnostic backed by a golden test that expects it.
- **PathGuard**: file built-ins go through existing IPC and surface VFP errors 1/1102 on denial; the player allows only the bundle dir and userData.
- **Two renderer entries**: verify electron-vite multi-page dev URLs in Phase 7; fallback is `?mode=player`.


## Milestone 3 outline (not planned in detail yet)

Recorded here so the M2 seams are built with it in mind; M3 gets its own plan when M2 ships.

- **DBF/FPT/CDX reader** in Rust inside the foxvm crate (`crates/foxvm/src/dbf/`): table header, field descriptors, memo blocks, deleted flag, code pages; exposed to JS through the same wasm module. This is the shared foundation for both items below.
- **VFP project importer**: open a `.pjx` (project table + `.pjt` memo) and convert the referenced `.scx/.sct` forms, `.vcx/.vct` class libraries and `.mnx/.mnt` menus into `.fxf`/`.fxm` JSON next to the originals, writing a `.fxp` for the project; `.prg` files are referenced as-is. The M1 schemas already keep VFP property names and defaults verbatim so records map one-to-one (`Properties` memo -> sparse props, `Methods` memo -> the methods map, `ObjName`/`Parent` -> the tree). Unsupported base classes become `Container` placeholders with a warning listed in Output. IDE surface: `File > Import VFP Project...`.
- **Data engine** behind the `DataSessionHost` seam from M2: work areas, `USE/SELECT/SCAN/SKIP/GO/SEEK/LOCATE/REPLACE/APPEND/DELETE`, `RECNO/RECCOUNT/EOF/BOF/FOUND`, `SET FILTER/ORDER/DELETED`, cursors, then `SELECT-SQL` (SQLite-in-wasm is the candidate). `ControlSource`/`RowSource` binding and a live `Grid`.
- **End-user installer** for built apps and a slim runtime-only player build.
