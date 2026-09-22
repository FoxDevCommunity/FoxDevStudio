---
layout: ../../layouts/DocsLayout.astro
title: Bytecode
kicker: The runtime
description: The module the compiler writes and the VM reads - the container, the constant pool, function prototypes and lambda captures - and what real programs compile to, shown with the disassembler.
---

The bytecode is Visual FoxPro's p-code analogue: the serialisable output of the compiler and
the only input of the VM. It is a **stack machine with a deliberately fat instruction set**.
Where a small machine would spell `REPLACE` out as twenty loads and stores, this one has a
`ReplaceField` instruction, so the interpreter loop is one `match`, a statement boundary is an
explicit instruction, and a disassembly reads like the program it came from.

This page is the format at version 9 and a tour of what the compiler makes of ordinary
programs. Every listing on it is the output of `foxvm disasm`. The
[instruction reference](/docs/instructions) lists all 199 instructions with their operands and
stack effects, generated from the VM's source.

## From source to module

<div class="diagram-frame">
<div class="diagram-bar"><b>The pipeline</b><span>/</span><span>one crate, four stages, two places to stop</span></div>
<svg class="diagram" viewBox="0 0 680 250" role="img" aria-label="Five boxes in a row: source text, the lexer producing tokens, the parser producing an AST, the compiler producing a Module, and the VM running it. Diagnostics leave the lexer, parser and compiler and go to the editor linter, which is where foxvm check stops. The Module is encoded as bytes into an .fxa bundle, or loaded straight into the VM.">
  <defs>
    <marker id="bc-tip" viewBox="0 0 10 10" refX="9" refY="5" markerWidth="6.5" markerHeight="6.5" orient="auto-start-reverse"><path d="M0 0 L10 5 L0 10 z" class="dg-tip"></path></marker>
  </defs>
  <rect class="dg-box" x="16" y="40" width="88" height="52" rx="5"></rect>
  <text class="dg-name" x="60" y="62" text-anchor="middle">Source</text>
  <text class="dg-sub" x="60" y="79" text-anchor="middle">.prg, a method</text>
  <line class="dg-line" x1="104" y1="66" x2="150" y2="66" marker-end="url(#bc-tip)"></line>
  <rect class="dg-box" x="152" y="40" width="88" height="52" rx="5"></rect>
  <text class="dg-name" x="196" y="62" text-anchor="middle">Lexer</text>
  <text class="dg-sub" x="196" y="79" text-anchor="middle">lexer.rs</text>
  <line class="dg-line" x1="240" y1="66" x2="286" y2="66" marker-end="url(#bc-tip)"></line>
  <text class="dg-flow" x="264" y="56" text-anchor="middle">tokens</text>
  <rect class="dg-box" x="288" y="40" width="88" height="52" rx="5"></rect>
  <text class="dg-name" x="332" y="62" text-anchor="middle">Parser</text>
  <text class="dg-sub" x="332" y="79" text-anchor="middle">parser.rs, ast.rs</text>
  <line class="dg-line" x1="376" y1="66" x2="422" y2="66" marker-end="url(#bc-tip)"></line>
  <text class="dg-flow" x="400" y="56" text-anchor="middle">AST</text>
  <rect class="dg-box" x="424" y="40" width="88" height="52" rx="5"></rect>
  <text class="dg-name" x="468" y="62" text-anchor="middle">Compiler</text>
  <text class="dg-sub" x="468" y="79" text-anchor="middle">compiler.rs</text>
  <line class="dg-line" x1="512" y1="66" x2="558" y2="66" marker-end="url(#bc-tip)"></line>
  <text class="dg-flow" x="536" y="56" text-anchor="middle">Module</text>
  <rect class="dg-box-lead" x="560" y="40" width="104" height="52" rx="5"></rect>
  <text class="dg-name" x="612" y="62" text-anchor="middle">The VM</text>
  <text class="dg-sub" x="612" y="79" text-anchor="middle">vm.rs</text>
  <line class="dg-line" x1="196" y1="92" x2="196" y2="134" marker-end="url(#bc-tip)"></line>
  <line class="dg-line" x1="332" y1="92" x2="332" y2="134" marker-end="url(#bc-tip)"></line>
  <line class="dg-line" x1="468" y1="92" x2="468" y2="134" marker-end="url(#bc-tip)"></line>
  <rect class="dg-box" x="152" y="136" width="364" height="62" rx="5"></rect>
  <text class="dg-name" x="334" y="156" text-anchor="middle">Diagnostics, with line and column</text>
  <text class="dg-sub" x="334" y="173" text-anchor="middle">the editor linter and foxvm check stop here;</text>
  <text class="dg-sub" x="334" y="188" text-anchor="middle">the parser recovers and reads on</text>
  <path class="dg-line" d="M 540 92 L 540 218 L 558 218" marker-end="url(#bc-tip)"></path>
  <rect class="dg-box" x="560" y="200" width="104" height="36" rx="5"></rect>
  <text class="dg-name" x="612" y="216" text-anchor="middle">encode()</text>
  <text class="dg-sub" x="612" y="230" text-anchor="middle">bytes in an .fxa</text>
  <text class="dg-flow" x="16" y="214">every program takes these four stages: a .prg, a form method,</text>
  <text class="dg-flow" x="16" y="228">a menu command, a Command Window line, EVALUATE() text,</text>
  <text class="dg-flow" x="16" y="242">or the line a macro produced</text>
</svg>
</div>

The compiler produces a `Module`. In the IDE it is handed straight to `load_module`; **Build
App** encodes it to bytes and stores them base64 in the `.fxa` bundle, and the player decodes
them again. Nothing about a module depends on which road it took.

## The container

A module is a byte string:

```text
"FXVM"            4 bytes, the magic
version           u16, little-endian; 9 today
body              the Module, encoded with postcard
```

A module with the wrong magic is refused as not a FoxVM module. A module with another version
is refused with "Module was built with bytecode version N; this runtime expects 9. Rebuild the
project." - the format is not versioned for compatibility, it is versioned so that a stale
`.fxa` says so plainly. An `.fxa` bundle also carries the crate version that built it, for the
same reason.

`postcard` is the serde format for the body: compact, schemaless, and the crate the rest of the
runtime already depends on rather than a wire format of our own. Two consequences follow that
matter to anyone reading the format. An enum variant is encoded by its **position** in the enum,
so an instruction can be added at the end but never moved; that is why a few instructions sit
away from their family in the source and the reference shows them under the family instead. And
integers are varints, so a small constant index or slot number costs one byte.

## The module

<div class="diagram-frame">
<div class="diagram-bar"><b>What a Module holds</b><span>/</span><span>and what each part is indexed by</span></div>
<svg class="diagram" viewBox="0 0 680 300" role="img" aria-label="A module drawn as a stack of sections: the header with name and kind; the constant pool, indexed by Const; the name table, indexed by LoadName and IndexOrCall; the member table, indexed by GetMember and CallMethod; the function prototypes, indexed by calls and MakeLambda; the methods map from object path and event to a function; and the tables for classes, queries, cursors and DLL declarations.">
  <rect class="dg-box" x="24" y="16" width="300" height="34" rx="5"></rect>
  <text class="dg-name" x="40" y="38">name, kind</text>
  <text class="dg-sub" x="330" y="38">Program | Form | Snippet | Expression</text>
  <rect class="dg-box" x="24" y="56" width="300" height="34" rx="5"></rect>
  <text class="dg-name" x="40" y="78">consts</text>
  <text class="dg-code" x="140" y="78">Vec&lt;Constant&gt;</text>
  <text class="dg-sub" x="330" y="78">Const(i) pushes one; class property values live here too</text>
  <rect class="dg-box" x="24" y="96" width="300" height="34" rx="5"></rect>
  <text class="dg-name" x="40" y="118">names</text>
  <text class="dg-code" x="140" y="118">Vec&lt;String&gt;, upper-cased</text>
  <text class="dg-sub" x="330" y="118">variables, procedures, settings: LoadName, Do, SetCmd</text>
  <rect class="dg-box" x="24" y="136" width="300" height="34" rx="5"></rect>
  <text class="dg-name" x="40" y="158">members</text>
  <text class="dg-code" x="140" y="158">Vec&lt;String&gt;, as written</text>
  <text class="dg-sub" x="330" y="158">properties and methods: GetMember, SetMember, CallMethod</text>
  <rect class="dg-box-lead" x="24" y="176" width="300" height="34" rx="5"></rect>
  <text class="dg-name" x="40" y="198">funcs</text>
  <text class="dg-code" x="140" y="198">Vec&lt;FuncProto&gt;</text>
  <text class="dg-sub" x="330" y="198">funcs[0] is main; calls and MakeLambda name one by index</text>
  <rect class="dg-box" x="24" y="216" width="300" height="34" rx="5"></rect>
  <text class="dg-name" x="40" y="238">methods</text>
  <text class="dg-code" x="140" y="238">"OBJPATH.EVENT" -&gt; func</text>
  <text class="dg-sub" x="330" y="238">a form's Click finds its function here; ".INIT" is the form</text>
  <rect class="dg-box" x="24" y="256" width="300" height="34" rx="5"></rect>
  <text class="dg-name" x="40" y="278">classes, queries, cursors, dlls</text>
  <text class="dg-sub" x="330" y="278">DEFINE CLASS, SQL plans, CURSOR columns, DECLARE DLL</text>
</svg>
</div>

The four kinds say what `funcs[0]` is:

| Kind | What it holds |
| --- | --- |
| `Program` | A `.prg`: `funcs[0]` is the implicit main, the rest are its `PROCEDURE`s and `FUNCTION`s. |
| `Form` | One function per method with source, listed in `methods` under `"PGFMAIN.PAGE1.LBLGREETING.CLICK"` or `".INIT"` for the form itself. No main. |
| `Snippet` | A menu command, an `EXECSCRIPT()` body, a Command Window line: `funcs[0]` is the body. |
| `Expression` | `EVALUATE()`, `&macro`, a `SKIP FOR` condition: `funcs[0]` pushes one value and returns it. |

### Constants

```text
Num(f64, chars, decimals)   a number and the width it was written in
Money(i64)                  $12.34 as written, in ten-thousandths
Str(String)
Date(Option<i32>)           days since 1970-01-01; None is the empty date
DateTime(Option<f64>)       seconds since 1970-01-01; None is the empty datetime
Bool(bool)                  only class property values need these; code pushes True/False
Null
Array(Vec<Constant>)        DIMENSION aRGB[3] in a class body
```

A numeric constant remembers how it was written because Visual FoxPro does: `? 001` prints
three characters, and the width has to travel with the value for that to be true. In the
listings below, `Num(10.0, 2, 0)` is the `10` of `FOR i = 1 TO 10`: two characters, none of
them past the point.

### Function prototypes

```text
FuncProto
  name          upper-cased; MAIN for a program body, OBJPATH.EVENT for a method
  display_name  as error messages and PROGRAM() show it
  nparams       declared parameters, which occupy local slots 0..nparams
  locals        slot -> upper-cased name, so runtime-compiled code can find a local by name
  def_line      the source line LINENO(1) counts from
  captures      for a lambda: (from, to) slot pairs; empty for everything else
  code          the instructions
```

### Classes

`DEFINE CLASS ... ENDDEFINE` becomes a `ClassProto`: the name and parent as written, the
property values constant-folded, the `ADD OBJECT` members, and the methods as `("Init", func)`
or `("image1.Click", func)`. Inheritance is **not** flattened: a proto carries only what its own
declaration says, and the host walks the chain when it instantiates.

## A frame

Every function call pushes a frame, and the frame is where the instructions' operands point.

<div class="diagram-frame">
<div class="diagram-bar"><b>One frame, and the value stack beside it</b><span>/</span><span>what LoadLocal, LoadName and LoadThis reach</span></div>
<svg class="diagram" viewBox="0 0 680 318" role="img" aria-label="On the left, a frame: its locals as numbered slots with the parameters first, then the declared locals, then a lambda's captures; below them the privates map, THIS, the WITH stack and the argument list. On the right, the fiber's value stack, growing upward, with the frame's stack base marked. LoadLocal reads a slot by number; LoadName walks this frame's privates, then each caller's, then the globals.">
  <rect class="dg-box" x="24" y="16" width="404" height="286" rx="6"></rect>
  <text class="dg-name" x="40" y="38">Frame</text>
  <text class="dg-sub" x="90" y="38">module 0, func 1, pc 4, line 9</text>
  <text class="dg-sub" x="40" y="62">locals: one slot each, .F. until written</text>
  <rect class="dg-box-plain" x="40" y="70" width="60" height="30" rx="3"></rect>
  <text class="dg-code" x="70" y="89" text-anchor="middle">0 PN</text>
  <rect class="dg-box-plain" x="106" y="70" width="60" height="30" rx="3"></rect>
  <text class="dg-code" x="136" y="89" text-anchor="middle">1 I</text>
  <rect class="dg-box-plain" x="172" y="70" width="70" height="30" rx="3"></rect>
  <text class="dg-code" x="207" y="89" text-anchor="middle">2 TOTAL</text>
  <rect class="dg-box-plain" x="248" y="70" width="74" height="30" rx="3"></rect>
  <text class="dg-code" x="285" y="89" text-anchor="middle">3 #FOR..</text>
  <rect class="dg-box-lead" x="328" y="70" width="84" height="30" rx="3"></rect>
  <text class="dg-code" x="370" y="89" text-anchor="middle">4 N (capt.)</text>
  <text class="dg-flow" x="40" y="118">params first, then LOCALs, then the compiler's</text>
  <text class="dg-flow" x="40" y="132">temporaries, then a lambda's captured slots.</text>
  <text class="dg-flow" x="40" y="146">A callee never sees them.</text>
  <rect class="dg-box-plain" x="40" y="160" width="180" height="30" rx="3"></rect>
  <text class="dg-code" x="50" y="179">privates: { X: 5 }</text>
  <text class="dg-sub" x="230" y="179">PRIVATE and undeclared: dynamic</text>
  <rect class="dg-box-plain" x="40" y="196" width="180" height="30" rx="3"></rect>
  <text class="dg-code" x="50" y="215">this: handle 12</text>
  <text class="dg-sub" x="230" y="215">LoadThis; THISFORM comes from it</text>
  <rect class="dg-box-plain" x="40" y="232" width="180" height="30" rx="3"></rect>
  <text class="dg-code" x="50" y="251">with_stack: [ 12 ]</text>
  <text class="dg-sub" x="230" y="251">PushWith / LoadWith / PopWith</text>
  <rect class="dg-box-plain" x="40" y="268" width="372" height="22" rx="3"></rect>
  <text class="dg-code" x="50" y="283">args, stack_base, stmt_sp, catch, finally</text>
  <text class="dg-name" x="450" y="38">Value stack</text>
  <text class="dg-sub" x="540" y="38">shared by the fiber</text>
  <rect class="dg-box-plain" x="450" y="52" width="206" height="26" rx="3"></rect>
  <text class="dg-code" x="460" y="69">Number(2, w1.0)</text>
  <text class="dg-flow" x="622" y="69">top</text>
  <rect class="dg-box-plain" x="450" y="80" width="206" height="26" rx="3"></rect>
  <text class="dg-code" x="460" y="97">Ref -&gt; caller's slot 0</text>
  <rect class="dg-box-plain" x="450" y="108" width="206" height="26" rx="3"></rect>
  <text class="dg-code" x="460" y="125">Str("customer")</text>
  <line class="dg-split" x1="450" y1="140" x2="656" y2="140"></line>
  <text class="dg-flow" x="450" y="156">stack_base: cut back to here</text>
  <text class="dg-flow" x="450" y="170">on return or unwind</text>
  <rect class="dg-box" x="450" y="180" width="206" height="26" rx="3"></rect>
  <text class="dg-sub" x="460" y="197">the caller's values, untouched</text>
  <text class="dg-flow" x="450" y="232">LoadLocal(2)   slot 2 of this frame</text>
  <text class="dg-flow" x="450" y="248">LoadName(0)    privates here, then</text>
  <text class="dg-flow" x="450" y="262">               each caller's, then PUBLIC</text>
  <text class="dg-flow" x="450" y="278">Ref(Local(0))  a cell over slot 0,</text>
  <text class="dg-flow" x="450" y="292">               for a @ argument</text>
</svg>
</div>

A `LOCAL` is a slot and is never visible to a callee. A `PRIVATE` or an undeclared name goes in
the frame's `privates` map and is found by walking the callers, which is what dynamic scope has
always meant here. `#FOR_END1` and `#FOR_STEP2` in the listings below are the compiler's own
slots: a `FOR` loop's bounds are evaluated once, as Visual FoxPro evaluates them, and have to
be kept somewhere a program cannot name.

## What programs compile to

Each listing is `foxvm disasm` on the program above it. Read `[before] -> [after]` with the
top of the stack on the right.

### A statement, a local, a print

```foxpro
LOCAL n
n = 3
? n * 2
```

```text
func 0 MAIN (demo) nparams=0 locals=["N"]
     0  Stmt(1)
     1  DeclLocal(0)
     2  Stmt(2)
     3  Const(0)                              ; Num(3.0, 1, 0)
     4  StoreLocal(0)
     5  Stmt(3)
     6  Print { newline: true, argc: 0 }
     7  LoadLocal(0)
     8  Const(1)                              ; Num(2.0, 1, 0)
     9  Mul
    10  Print { newline: false, argc: 1 }
    11  True
    12  EndOfCode
```

Every statement begins with `Stmt(line)`: it records the line for error messages and
`LINENO()`, and it is where a breakpoint hooks. `?` is two prints, the newline it starts with
and then the item, because `??` is the same instruction without the first. `True` then
`EndOfCode` is the `.T.` a routine answers when it runs off the end, which is what Visual FoxPro
answers, measured. `EndOfCode` is not `Return`: a written `RETURN` also has to leave a line a
macro put together, and the routine that line belongs to.

### IF, and a short-circuit AND

```foxpro
LOCAL a, b
a = 3
b = 4
IF a > 2 AND b < 5
    ? "both"
ELSE
    ? "not both"
ENDIF
```

```text
    10  LoadLocal(0)
    11  Const(2)
    12  Gt
    13  JumpIfFalseKeep(18)
    14  LoadLocal(1)
    15  Const(3)
    16  Lt
    17  And
    18  JumpIfFalse(24)
    19  Stmt(5)
    20  Print { newline: true, argc: 0 }
    21  Const(4)                              ; Str("both")
    22  Print { newline: false, argc: 1 }
    23  Jump(28)
    24  Stmt(7)
    25  Print { newline: true, argc: 0 }
    26  Const(5)                              ; Str("not both")
    27  Print { newline: false, argc: 1 }
    28  True
```

`a AND b` compiles to `a; JumpIfFalseKeep(end); b; And; end:`. `JumpIfFalseKeep` peeks rather
than pops, so when the left side is `.F.` it stays on the stack as the answer and the right
side is never evaluated; when it is `.T.` the right side is evaluated and `And` combines the
two, three-valued, because either may be `.NULL.`. Jump targets are absolute program counters.

### FOR, with its bounds evaluated once

```foxpro
LOCAL i, total
total = 0
FOR i = 1 TO 10 STEP 2
    total = total + i
ENDFOR
? total
```

```text
func 0 MAIN (loop) nparams=0 locals=["I", "TOTAL", "#FOR_END1", "#FOR_STEP2"]
     7  Const(1)                              ; Num(1.0, 1, 0)
     8  StoreLocal(0)
     9  Const(2)                              ; Num(10.0, 2, 0)
    10  StoreLocal(2)
    11  Const(3)                              ; Num(2.0, 1, 0)
    12  StoreLocal(3)
    13  LoadLocal(0)
    14  LoadLocal(2)
    15  LoadLocal(3)
    16  ForTest(27)
    17  Stmt(4)
    18  LoadLocal(1)
    19  LoadLocal(0)
    20  Add
    21  StoreLocal(1)
    22  LoadLocal(0)
    23  LoadLocal(3)
    24  Add
    25  StoreLocal(0)
    26  Jump(13)
    27  Stmt(6)
```

The end and the step go into two slots the program cannot name, so changing `total` inside the
body cannot change how many times it runs. `ForTest` takes `[var, end, step]` and jumps out
when the loop is finished, in either direction, which is why the step is on the stack too.

### DO WITH a by-reference argument, and a FUNCTION call

```foxpro
LOCAL n
n = 5
DO double WITH n
? n
? twice(4)

PROCEDURE double
    LPARAMETERS pn
    pn = pn * 2
ENDPROC

FUNCTION twice(x)
    RETURN x * 2
ENDFUNC
```

```text
func 0 MAIN (call) nparams=0 locals=["N"]
     6  Ref(Local(0))
     7  Do { name: 0, argc: 1, in_prog: false }     ; name 0: DOUBLE
    ...
    14  Const(1)                                    ; Num(4.0, 1, 0)
    15  IndexOrCall { name: 1, argc: 1 }            ; name 1: TWICE
func 1 DOUBLE (double) nparams=1 locals=["PN"]
     0  Stmt(9)
     1  LoadLocal(0)
     2  Const(2)
     3  Mul
     4  StoreLocal(0)
     5  True
     6  EndOfCode
func 2 TWICE (twice) nparams=1 locals=["X"]
     0  Stmt(13)
     1  LoadLocal(0)
     2  Const(2)
     3  Mul
     4  Return
```

`DO ... WITH` passes by reference, so the argument is `Ref(Local(0))`: a cell over the caller's
slot that the callee's `StoreLocal(0)` writes through, which is why `? n` prints 10. A function
call in an expression is `IndexOrCall`, the one question asked of a name: an array of that name
is subscripted, a function is called, and anything else looks for a program of that name. A
parameter is slot 0 of the callee; `nparams=1` says how many slots the arguments fill.

### TRY, CATCH TO, FINALLY

```foxpro
TRY
    x = 1 / 0
CATCH TO oErr
    ? oErr.Message
FINALLY
    ? "done"
ENDTRY
```

```text
     1  TryPush { catch: 9, finally: 17 }
     2  Stmt(2)
     3  Const(0)
     4  Const(1)
     5  Div
     6  StoreName(0)                          ; name 0: X
     7  TryPop
     8  Jump(17)
     9  CatchObject
    10  StoreName(1)                          ; name 1: OERR
    11  Stmt(4)
    12  Print { newline: true, argc: 0 }
    13  LoadField { area: Some(1), field: 0 } ; oErr.Message
    14  Print { newline: false, argc: 1 }
    15  TryPop
    16  Jump(17)
    17  Stmt(6)
    18  Print { newline: true, argc: 0 }
    19  Const(2)                              ; Str("done")
    20  Print { newline: false, argc: 1 }
    21  EndFinally
```

`TryPush` installs a handler naming both targets. When `Div` raises error 1307, the VM pops any
frames above the one that installed the handler, cuts the value stack back to where it was at
`TryPush`, and jumps to `catch`. `CatchObject` yields `CreateException` so the host can build the
`Exception` object, and `StoreName` puts it in `oErr`. Both paths end at `finally`, and
`EndFinally` re-raises whatever was still propagating when the block was entered - nothing, on
the paths shown. `oErr.Message` is `LoadField` and not `GetMember`, because a bare name followed
by a dot is only known at run time to be an object variable, an `m.` prefix, or a table alias.

### A lambda and its capture

```foxpro
LOCAL n, f
n = 1
f = LAMBDA(x)
    RETURN x + n
ENDLAMBDA
? f(10)
```

```text
func 0 MAIN (lambda) nparams=0 locals=["N", "F"]
     4  Const(0)                              ; Num(1.0, 1, 0)
     5  StoreLocal(0)
     6  Stmt(3)
     7  MakeLambda(1)
     8  StoreLocal(1)
     9  Stmt(6)
    10  Print { newline: true, argc: 0 }
    11  LoadLocal(1)
    12  Const(1)                              ; Num(10.0, 2, 0)
    13  IndexOrCallValue(1)
    14  Print { newline: false, argc: 1 }
func 1 #LAMBDA1 (lambda LAMBDA at line 3) nparams=1 locals=["X", "N", "F"]
     0  Stmt(4)
     1  LoadLocal(0)
     2  LoadLocal(1)
     3  Add
     4  Return
```

The lambda is an ordinary function of the module, `#LAMBDA1`, with a parameter in slot 0 and
the enclosing routine's two locals in slots 1 and 2; its proto's `captures` say `[(0, 1),
(1, 2)]`. `MakeLambda(1)` copies slot 0 and slot 1 of the current frame into a new function
value and pushes it. `IndexOrCallValue(1)` asks of a *value* what `IndexOrCall` asks of a name,
and when it is a function the VM pushes a frame for `#LAMBDA1`, writes the argument into slot 0
and the captured values into slots 1 and 2, and carries on round the loop. Nothing recurses.

<div class="diagram-frame">
<div class="diagram-bar"><b>Capture by value</b><span>/</span><span>the two moments a captured LOCAL is copied</span></div>
<svg class="diagram" viewBox="0 0 680 208" role="img" aria-label="Three boxes. The enclosing frame's locals, N holding 1 and F. An arrow labelled MakeLambda copies N into the function value's captured list. A second arrow labelled the call copies the function value's captured list into slots 1 and 2 of the lambda's own frame, whose slot 0 holds the argument 10. A note says that a later change to the outer N is not seen, and a change inside the lambda changes only its copy.">
  <defs>
    <marker id="cap-tip" viewBox="0 0 10 10" refX="9" refY="5" markerWidth="6.5" markerHeight="6.5" orient="auto-start-reverse"><path d="M0 0 L10 5 L0 10 z" class="dg-tip-lead"></path></marker>
  </defs>
  <rect class="dg-box" x="24" y="24" width="150" height="96" rx="5"></rect>
  <text class="dg-name" x="36" y="46">MAIN's frame</text>
  <rect class="dg-box-plain" x="36" y="58" width="64" height="26" rx="3"></rect>
  <text class="dg-code" x="68" y="75" text-anchor="middle">0 N = 1</text>
  <rect class="dg-box-plain" x="106" y="58" width="44" height="26" rx="3"></rect>
  <text class="dg-code" x="128" y="75" text-anchor="middle">1 F</text>
  <text class="dg-sub" x="36" y="106">slots 0 and 1</text>
  <line class="dg-line-lead" x1="174" y1="72" x2="262" y2="72" marker-end="url(#cap-tip)"></line>
  <text class="dg-flow" x="219" y="60" text-anchor="middle">MakeLambda(1)</text>
  <text class="dg-flow" x="219" y="92" text-anchor="middle">copies</text>
  <rect class="dg-box-lead" x="264" y="24" width="150" height="96" rx="5"></rect>
  <text class="dg-name" x="276" y="46">Function value #7</text>
  <text class="dg-code" x="276" y="66">module 0, func 1</text>
  <text class="dg-code" x="276" y="82">this: none</text>
  <text class="dg-code" x="276" y="98">captured: [1, .F.]</text>
  <text class="dg-sub" x="276" y="114">kept for the run</text>
  <line class="dg-line-lead" x1="414" y1="72" x2="502" y2="72" marker-end="url(#cap-tip)"></line>
  <text class="dg-flow" x="459" y="60" text-anchor="middle">f(10)</text>
  <text class="dg-flow" x="459" y="92" text-anchor="middle">copies</text>
  <rect class="dg-box" x="504" y="24" width="152" height="96" rx="5"></rect>
  <text class="dg-name" x="516" y="46">#LAMBDA1's frame</text>
  <rect class="dg-box-plain" x="516" y="58" width="44" height="26" rx="3"></rect>
  <text class="dg-code" x="538" y="75" text-anchor="middle">X=10</text>
  <rect class="dg-box-plain" x="566" y="58" width="38" height="26" rx="3"></rect>
  <text class="dg-code" x="585" y="75" text-anchor="middle">N=1</text>
  <rect class="dg-box-plain" x="610" y="58" width="30" height="26" rx="3"></rect>
  <text class="dg-code" x="625" y="75" text-anchor="middle">F</text>
  <text class="dg-sub" x="516" y="106">slots 0, 1 and 2</text>
  <text class="dg-flow" x="24" y="150">n = 2 after the lambda was made: it still adds 1, because slot 0 was copied at MakeLambda.</text>
  <text class="dg-flow" x="24" y="166">n = n + 10 inside the lambda: changes slot 1 of the lambda's frame, never MAIN's slot 0.</text>
  <text class="dg-flow" x="24" y="182">A PRIVATE is not in any slot, so it is not copied: it is looked up when the lambda runs.</text>
  <text class="dg-flow" x="24" y="198">THIS travels with the value, so a lambda written in a method keeps its object.</text>
</svg>
</div>

### SCAN FOR

```foxpro
USE customer
SCAN FOR country = "UK"
    ? company
ENDSCAN
USE
```

```text
     1  Const(0)                              ; Str("customer")
     2  Use { alias: None, named_alias: false, exclusive: false, online: false, in_area: false }
     3  OpenIndex
     4  Stmt(2)
     5  Go(Top)
     6  CallBuiltin { id: 145, argc: 0 }      ; EOF()
     7  JumpIfTrue(19)
     8  LoadName(0)                           ; name 0: COUNTRY
     9  Const(1)                              ; Str("UK")
    10  Eq
    11  JumpIfFalse(16)
    12  Stmt(3)
    13  Print { newline: true, argc: 0 }
    14  LoadName(1)                           ; name 1: COMPANY
    15  Print { newline: false, argc: 1 }
    16  Const(2)                              ; Num(1.0, 1, 0)
    17  Skip
    18  Jump(6)
    19  Stmt(5)
    20  Const(3)                              ; Str("")
    21  Use { alias: None, ... }
```

A `SCAN` is nothing special: `Go(Top)`, then a loop of `EOF()`, the `FOR` condition, the body,
`Skip 1`. `Use` yields a request to open the file and `OpenIndex` reads the structural `.cdx`
beside it if the header says one is there. A field is read by `LoadName`, the same instruction a
variable uses, because Visual FoxPro looks a bare name up the same way: a variable first, then a
field of the selected area. `USE` with an empty path closes the area.

### SELECT-SQL

```foxpro
SELECT custno, SUM(amount) AS total ;
    FROM orders ;
    GROUP BY custno ;
    INTO CURSOR c_top
```

```text
     1  SqlOpen { table: 0, alias: 0, named: false }
     2  SqlBegin(0)
     3  SelectSource(0)
     4  Go(Top)
     5  SelectSource(0)
     6  CallBuiltin { id: 145, argc: 0 }      ; EOF()
     7  JumpIfTrue(16)
     8  LoadName(1)                           ; CUSTNO, the group key
     9  LoadName(2)                           ; AMOUNT, for SUM
    10  LoadName(1)                           ; CUSTNO, the select list
    11  SqlRow(3)
    12  SelectSource(0)
    13  Const(1)
    14  Skip
    15  Jump(5)
    16  SqlEnd
```

A query is compiled to the same loop a hand-written `SCAN` uses: `Go(Top)`, `EOF()`, `Skip`.
What holds it together is `Module::queries[0]`, a `QueryPlan` that says what the three values
`SqlRow(3)` collects per record are for - a group key, an aggregate input, a column - and
`SqlEnd` finishes the grouping, applies `HAVING` and `ORDER BY`, and writes the cursor. Several
sources are nested loops with `SelectSource(n)` choosing which is current, and an outer join's
miss is `JoinMiss`. That a query is bytecode over the same primitives is why it can read a table
larger than memory the way a `SCAN` does.

### Objects and WITH

```foxpro
oForm.Caption = "Hi"
WITH oForm
    .Left = 10
    .Show()
ENDWITH
```

```text
     1  Const(0)                              ; Str("Hi")
     2  LoadName(0)                           ; OFORM
     3  SetMember(0)                          ; member 0: Caption
     4  Stmt(2)
     5  LoadName(0)
     6  PushWith
     7  Stmt(3)
     8  Const(1)
     9  LoadWith
    10  SetMember(1)                          ; member 1: Left
    11  Stmt(4)
    12  LoadWith
    13  CallMethod { name: 2, argc: 0 }       ; member 2: Show
    14  Pop
    15  PopWith
```

`SetMember` takes `[value, obj]` and yields `SetProp` to the host, which owns the object; the
fiber is resumed once the property is written and any `ProgrammaticChange` it fired has run.
`CallMethod` yields `CallMethod` and its result is pushed, so a call whose value is not wanted is
followed by `Pop`. `WITH` is a stack in the frame: `PushWith` saves the object, `LoadWith` reads
it for each `.member`, `PopWith` lets it go. Member names are stored as written and the host
compares them case-insensitively; variable names are upper-cased in the module because the
language is.

## Which instructions leave the machine

About a third of the instructions can yield a host request; the rest complete inside the VM. The
ones that yield are the ones whose effect is outside the machine's own memory: every object
write and method call, every file and table operation, every dialog, `DoForm`, `DoMenu`,
`CreateObject`, the DLL and library calls. An instruction that yields is resumed with a value
and either pushes it, discards it, or - for the data instructions, which can take several round
trips to read a record - rewinds the program counter and runs again with the reply in hand. The
[virtual machine page](/docs/vm) is about that loop; the
[instruction reference](/docs/instructions) says which request each one yields.
