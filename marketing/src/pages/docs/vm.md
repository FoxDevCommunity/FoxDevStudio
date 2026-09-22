---
layout: ../../layouts/DocsLayout.astro
title: The virtual machine
kicker: The runtime
description: A compiler and a bytecode interpreter in Rust, compiled to WebAssembly, built so that a FoxPro program can stop for a dialog without stopping the window it is in.
---

Visual FoxPro compiled a program to p-code and shipped a runtime that executed it. FoxDev
Studio is the same arrangement made again: `crates/foxvm` is a lexer, an error-recovering
parser, a bytecode compiler and an interpreter, written in Rust and compiled to WebAssembly
with `wasm-bindgen`. One module runs your code in the IDE, in a shipped application, and in the
headless `foxvm` runner. The editor lints through the same compiler, so what it underlines and
what the runtime refuses cannot drift apart.

This page is about the machine. [Bytecode](/docs/bytecode) is about what it runs.

## Fibers

Every running program, form method, menu command and Command Window line is a **fiber**: a
value stack, a stack of frames, the `TRY` handlers its frames have installed, and the
bookkeeping for one pending host request. Several fibers can be parked at once - a program in
`READ EVENTS`, a `Click` method running in another, a Timer firing in a third - and the
scheduler in the host decides which one steps next.

A frame owns its `LOCAL` slots, its `PRIVATE` variables, `THIS`, its `WITH` stack and the
argument list it was called with. A `LOCAL` is a numbered slot and is never visible to a callee.
A `PRIVATE` or an undeclared name is dynamic: `LoadName` walks the privates of the current
frame, then of every caller, then the `PUBLIC` globals, which is what dynamic scoping has always
meant in this language.

A call is not a Rust call. `run` is one loop over the frame stack; a call pushes a frame and
goes round again, and a return pops one. Nothing recurses into the interpreter, which is what
makes the next section possible.

## The VM never blocks

The wasm exports are **not re-entrant**: calling one while another is on the stack throws. The
whole runtime is built around that, and it turns out to be the design rather than a constraint.

When a program needs something from the world outside - a message box, a modal form, the next
page of a table, a COM call, a file - the VM does not call out and wait. It **yields a host
request** and stops. `step` answers with one of three things:

```text
Done    { value, nodefault }     the fiber finished, and this is what it returned
Error   { error, stack }         it failed, and here is the error and where it was
Suspend { request }              it needs the host: perform this, then resume me
```

<div class="diagram-frame">
<div class="diagram-bar"><b>One MESSAGEBOX()</b><span>/</span><span>what crosses the boundary, and in which order</span></div>
<svg class="diagram" viewBox="0 0 680 384" role="img" aria-label="Two lifelines, the scheduler on the left and the VM on the right. The scheduler calls step; the VM answers Suspend with a MessageBox request; the scheduler shows the dialog while the VM is off the stack, and may run a nested fiber there; the scheduler calls resume with the button pressed, then step again; the VM answers Done with the value the program returned.">
  <defs>
    <marker id="vm-tip" viewBox="0 0 10 10" refX="9" refY="5" markerWidth="6.5" markerHeight="6.5" orient="auto-start-reverse"><path d="M0 0 L10 5 L0 10 z" class="dg-tip"></path></marker>
    <marker id="vm-tip-lead" viewBox="0 0 10 10" refX="9" refY="5" markerWidth="6.5" markerHeight="6.5" orient="auto-start-reverse"><path d="M0 0 L10 5 L0 10 z" class="dg-tip-lead"></path></marker>
  </defs>
  <rect class="dg-box" x="40" y="18" width="200" height="44" rx="5"></rect>
  <text class="dg-name" x="140" y="37" text-anchor="middle">The scheduler</text>
  <text class="dg-sub" x="140" y="53" text-anchor="middle">JavaScript, in the renderer</text>
  <rect class="dg-box-lead" x="440" y="18" width="200" height="44" rx="5"></rect>
  <text class="dg-name" x="540" y="37" text-anchor="middle">The VM</text>
  <text class="dg-sub" x="540" y="53" text-anchor="middle">Rust, in WebAssembly</text>
  <line class="dg-rule" x1="140" y1="62" x2="140" y2="372"></line>
  <line class="dg-rule" x1="540" y1="62" x2="540" y2="372"></line>
  <line class="dg-line" x1="140" y1="92" x2="538" y2="92" marker-end="url(#vm-tip)"></line>
  <text class="dg-code" x="340" y="86" text-anchor="middle">step(fiber)</text>
  <rect class="dg-box-lead" x="532" y="92" width="16" height="40" rx="2"></rect>
  <text class="dg-sub" x="556" y="116">runs until MESSAGEBOX()</text>
  <line class="dg-line" x1="540" y1="132" x2="142" y2="132" marker-end="url(#vm-tip)"></line>
  <text class="dg-code" x="340" y="126" text-anchor="middle">Suspend { request: MessageBox { text, buttons } }</text>
  <rect class="dg-box" x="132" y="132" width="16" height="108" rx="2"></rect>
  <text class="dg-sub" x="156" y="150">shows the dialog; the VM is off the stack</text>
  <text class="dg-sub" x="156" y="166">the window paints, other forms respond</text>
  <line class="dg-split" x1="156" y1="180" x2="440" y2="180"></line>
  <text class="dg-flow" x="156" y="196">a Timer may fire here: a nested fiber,</text>
  <text class="dg-flow" x="156" y="210">stepped to completion before this one resumes</text>
  <line class="dg-split" x1="156" y1="220" x2="440" y2="220"></line>
  <text class="dg-sub" x="156" y="236">the user presses a button</text>
  <line class="dg-line-lead" x1="140" y1="256" x2="538" y2="256" marker-end="url(#vm-tip-lead)"></line>
  <text class="dg-code" x="340" y="250" text-anchor="middle">resume(fiber, 7)</text>
  <text class="dg-flow" x="340" y="272" text-anchor="middle">the 7 is pushed where the call's result goes</text>
  <line class="dg-line" x1="140" y1="304" x2="538" y2="304" marker-end="url(#vm-tip)"></line>
  <text class="dg-code" x="340" y="298" text-anchor="middle">step(fiber)</text>
  <rect class="dg-box-lead" x="532" y="304" width="16" height="36" rx="2"></rect>
  <text class="dg-sub" x="556" y="326">runs to the end</text>
  <line class="dg-line" x1="540" y1="348" x2="142" y2="348" marker-end="url(#vm-tip)"></line>
  <text class="dg-code" x="340" y="342" text-anchor="middle">Done { value: .T. }</text>
  <text class="dg-flow" x="340" y="368" text-anchor="middle">nothing above ever called into the VM while it was on the stack</text>
</svg>
</div>

The host performs the request while the VM is off the stack and then calls `resume` with the
answer (or `resume_error` with an error number and message, which the fiber's own `TRY` and
`ON ERROR` machinery then sees). There are 73 kinds of request, from `SetProp` and `CallMethod`
through `DoForm`, `CreateObject`, `LoadClassLib`, the file and table requests, to the SQL
pass-through and the library calls.

Three things fall out of this:

- **A modal dialog blocks the program, not the window.** `MESSAGEBOX()` is a request; the fiber
  is parked until the host answers, and the rest of the IDE - the Command Window, other forms,
  the debugger - carries on.
- **Nested events run in the order FoxPro ran them.** Because the VM is off the stack while a
  request is performed, the host may dispatch a nested event as a fiber of its own and drive it
  to completion before the outer fiber resumes. That is how `SetFocus()` fires `GotFocus`, how
  `Init` runs while a form is still being built, and how a Timer fires while a program is
  parked in `READ EVENTS`.
- **A built-in cannot run FoxPro itself.** A built-in handed a script, or a lambda, returns
  `RunScript` or `CallFunction` and lets the VM push the frame, because the code it would run
  may stop for the host half way through. `EXECSCRIPT()` is the whole of the first; a built-in
  handed a function value is the whole of the second.

### Reads are the one exception

A property read is synchronous. `THISFORM.Caption` in an expression cannot yield, because an
expression that stopped half way would not be an expression. So the VM is constructed with a
small object of **imports** - `get_prop`, `get_member`, `object_class`, `now`, `random`,
`mouse`, `resolve_program` and a few more - that the host answers immediately, without the VM
leaving the stack. Everything that changes the world goes the other way, as a request.

## The scheduler

`src/shared/runtime/scheduler.ts` is the host's half of the bargain. It steps a fiber,
performs what the fiber yields, resumes it, and keeps going until the fiber is done, has failed,
or is parked waiting for something that will arrive later (a dialog's answer, a form's
`Unload`, the next request on a socket).

Its rules are simple and every part of the runtime leans on them:

- **One fiber runs at a time, to completion or to a park.** Two `Click` methods do not
  interleave; two HTTP requests do not interleave.
- **An event that arrives while a fiber is on the JS stack is queued**, and the queue drains
  in arrival order the moment the stack unwinds. Starting a fiber there would be a re-entrant
  call.
- **A handler that blocks holds the queue.** A long `SELECT`, a `MESSAGEBOX`, a `WAIT WINDOW`
  with no timeout stops every other event until it is done. This is the price of the
  single-threaded model, and it is written down here rather than left to be discovered.
- **Cancelling a session settles everything.** Every parked fiber is aborted, every queued
  event is answered with "nothing ran", every open server is closed.

A host event is a function value and the arguments to call it with: `scheduler.raise({ func,
args })`. That is the seam a Timer, a menu choice and an HTTP request all cross; see
[the HTTP API](/docs/http-api) for the one that arrives from a socket.

## The data engine

The host owns the bytes and the VM owns the meaning. A `USE` asks the main process to open the
file; a record is read by a seek-and-read request with a 64-bit offset; a page of records is
decoded inside the VM; a change goes back as the field bytes at the offset they came from. A
table is never loaded into memory, which is why one can be larger than memory, and larger than
the two gigabytes Visual FoxPro stops at.

`SCAN`, `LOCATE`, `SEEK` against a `.cdx` tag, `SET FILTER`, `SET RELATION` and the SQL
`SELECT` all go through the same primitives. A query is compiled to a `QueryPlan` - nested loops
over the sources, with joins, `WHERE`, `GROUP BY`, `HAVING`, `ORDER BY`, `DISTINCT`, `TOP n`,
uncorrelated subqueries and `INTO CURSOR` or `INTO ARRAY` - and runs as bytecode over the same
`SqlBegin`, `SqlRow`, `SqlEnd` instructions a hand-written loop would, so it reads a table
larger than memory the same way a `SCAN` does.

## Values

The VM's `Value` is what Visual FoxPro's is - character, numeric (with the width it was written
in, so `? 001` prints `  1`), currency, date, datetime, logical, null, an array, an object
handle - plus the two FoxScript adds: a **function** (a lambda, `VARTYPE()` "F") and a
**JSON value** (`VARTYPE()` "J"). Both are described under [the new keywords](/docs/foxscript).

An object is a handle. The host allocates handles from 1 upwards and reserves `0x7fff_ffff` for
the application object; the VM's own objects - the `FoxScript` namespace and the servers it
makes - come from `0xF000_0000` upwards, and a test asserts the two ranges never meet. A value
crossing the boundary travels as JSON: an object as its handle, a function as its id, a JSON
value as its text, a date as an ISO string.

## What crosses the boundary

The `FoxVm` wasm class is the whole surface the host sees. Its exports fall into five groups:

| Group | Exports |
| --- | --- |
| Compiling | `compile_program`, `compile_form`, `compile_snippet`, `compile_expression`, `check`; each answers bytes plus diagnostics with line and column. |
| Loading and starting | `load_module`, `start` (a function by name), `start_method` (a form event), `start_class_method`, `start_function` (a lambda by id, which is what a host event is), `class_definitions`. |
| Driving | `step`, `resume`, `resume_error`, `abort`, `abort_all`, `route` (which lambda answers an HTTP request), `menu_chosen`. |
| The debugger | `set_breakpoint`, `clear_breakpoints`, `breakpoints`, `set_step_mode`, `frames`, `frame_variables`, `evaluate`, `evaluate_in`, `call_stack`. |
| The environment | `set_global`, `get_global`, `set_setting`, `get_setting`, `version`, and the DBF helpers the table browser uses: `read_dbf_header`, `decode_dbf_page`, `encode_dbf_field`. |

The module itself is inlined into the renderer as base64 and instantiated with `initSync`,
because a packaged application loads its pages over `file://` and the tests run under jsdom,
and neither can fetch a `.wasm` from a URL.

## Errors, and how they are numbered

An error unwinds to the innermost `TRY` handler of the fiber, whichever frame installed it:
frames above it are popped, the value stack is cut back to where it was at `TryPush`, and the
fiber continues at `CATCH` or, with only a `FINALLY`, at that block with the error pending for
`EndFinally`. Without a handler, `ON ERROR` text is compiled as a snippet and run in a new
frame, and the failing frame continues at its next statement when it returns. Otherwise the
fiber ends with `Error`.

Error numbers are Visual FoxPro's, measured: a missing program is error 1, an unknown member
1925, a read-only property 1743. FoxScript's own failures are numbered from 3001, a long way
above anything the product uses, so a program can tell one from the other by the number alone.

## Runtime compilation

`&macro`, `EXECSCRIPT()`, `EVALUATE()` and a Command Window line are compiled on the spot,
against the slot names of the function they run in, and executed in an **inline frame**: a
frame with no storage of its own that forwards every local, private, `WITH` and argument access
to its owner. Slot numbers in the compiled code therefore line up with the owner's, a snippet
can declare a `PRIVATE` the owner then sees, and compiled snippets are cached per owner
function and text.
