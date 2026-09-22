---
layout: ../../layouts/DocsLayout.astro
title: Documentation
kicker: Overview
description: What FoxDev Studio is, how it is put together, and where each part of it is written down.
---

FoxDev Studio is an IDE and a runtime for Visual FoxPro 9 applications. It opens the projects,
forms, menus, programs and tables you already have and runs them, on a 64-bit machine, with no
conversion step. It is built from three things:

- **The IDE**, an Electron application with a React interface: the project explorer, the form
  and menu designers, the properties window, a CodeMirror editor, the Command Window, an Output
  window and a debugger.
- **The virtual machine**, a compiler and a bytecode interpreter written in Rust and compiled to
  WebAssembly. It is the same module in the IDE, in a shipped application, and in the headless
  command-line runner, so one program means one thing everywhere.
- **The host**, the part of the application that owns the world outside the machine: files,
  tables, dialogs, COM objects, 64-bit libraries in process, 32-bit libraries in a process of
  their own, and the sockets a `FoxScript.Http` server listens on.

## How the pages are arranged

| Page | What it covers |
| --- | --- |
| [Getting started](/docs/getting-started) | Installing the IDE, opening the sample, running a form, importing a Visual FoxPro project, building an executable. |
| [Required tooling](/docs/tooling) | What a machine needs to run the IDE, and what it needs to build the IDE, the VM and the native pieces from source. |
| [The virtual machine](/docs/vm) | Fibers, host requests, the scheduler, why nothing in the VM blocks, and what crosses the WebAssembly boundary. |
| [Bytecode](/docs/bytecode) | The module container, the constant pool, function prototypes, lambda captures, and what real programs compile to, with the disassembler. |
| [Instruction reference](/docs/instructions) | All 199 instructions with their operands and stack effects, generated from the VM source so it cannot drift. |
| [The new keywords](/docs/foxscript) | `LAMBDA` and `ENDLAMBDA`, the `FoxScript` namespace, and the two value types Visual FoxPro has not got. |
| [The HTTP API](/docs/http-api) | The server a program can open, the Node side that holds the sockets, and the reasons for each decision. |

## The one rule everything follows

Where Visual FoxPro 9 has an answer, the product's answer wins. Behaviour is settled by running
`vfp9.exe` and matching what it does, not by reading a reference page and guessing; the tests
carry the answers the product gave, and a change that drifts from one fails.

Where the product has no answer - a lambda, a JSON value, an HTTP server - the specification is
written down first and the tests are the specification. That is what makes FoxScript a superset
and not a dialect: every program that runs in the product keeps running here and keeps meaning
what it meant.

Two documents in the repository are worth knowing about before assuming something is missing:

- **`docs/language-reference.md`** lists every built-in function with its argument count, every
  property of every control with its default, and - for the things that cannot run yet - why. It
  is generated from the runtime's own registries, and a test fails when it falls out of step.
- **`docs/language-coverage.md`** maps every element of the Visual FoxPro 9 language reference
  to one of five states: runs, accepted and ignored, answered on purpose, refused by name, or
  unknown. Today 1,534 of its 1,722 elements are exercised by a test, and three are unknown.

## Where things stand

The IDE shell, the designers, the language and the data engine are done and used daily. Reports
are the milestone under way. `FoxScript` - lambdas, the namespace, the HTTP server and the JSON
bridge - is built and specified in the pages here. Tables past two gigabytes work today through
64-bit offsets, with a container format of their own still to come.
