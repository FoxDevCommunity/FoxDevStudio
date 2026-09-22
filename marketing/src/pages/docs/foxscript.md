---
layout: ../../layouts/DocsLayout.astro
title: The new keywords
kicker: FoxScript
description: LAMBDA and ENDLAMBDA, the FoxScript namespace, and the two value types Visual FoxPro has not got. Each rule was written down before it was coded, and each has a test that fails if it changes.
---

FoxScript is Visual FoxPro 9 with things added: a function value you can hand to something else
to run later, a standard library that reaches the machine the way a program written this decade
expects to, and tables that are not stopped by two gigabytes. It is a **superset**. Every
program that runs in the product keeps running here and keeps meaning the same thing, and that
is the rule every decision below answers to: *where the product already has an answer, the
product wins*.

The whole project's method is to ask `vfp9.exe`. FoxScript has no `vfp9.exe` to ask, so where
there is nothing to measure the specification is written down first and the tests are the
specification. Every numbered rule on this page has one.

## LAMBDA ... ENDLAMBDA

```foxpro
LOCAL f
f = LAMBDA(x, y)
    RETURN x * y
ENDLAMBDA
? f(6, 7)        && 42
```

### Why this spelling

Every block in the language is a word and its matching end: `IF`/`ENDIF`, `DO CASE`/`ENDCASE`,
`FOR`/`ENDFOR`, `SCAN`/`ENDSCAN`, `TRY`/`ENDTRY`, `WITH`/`ENDWITH`, `TEXT`/`ENDTEXT`,
`DEFINE CLASS`/`ENDDEFINE`, `PROCEDURE`/`ENDPROC`. A brace block would be the only C-shaped
thing in it, and braces are not free anyway: `{^2024-01-31}` is a date literal and `{}` an empty
one. The word form costs nothing and reads like the language.

`ENDLAMBDA` rather than a bare `END`, and that was measured rather than chosen: `END` at the
start of a statement is already the beginning of `END TRANSACTION`, and the product answers a
bare one with error 1221, "Command is missing required clause." `ENDLAMBDA` collides with
nothing.

### 1. When LAMBDA opens a lambda

`LAMBDA` is **not a reserved word**. Measured in `vfp9.exe`, `LAMBDA` and `ENDLAMBDA` are legal
variable names, field names and property names, so `LAMBDA = 5`, `DIMENSION lambda(3)`,
`lambda(1)` and `tst.lambda` all keep working.

`LAMBDA` opens a lambda only when it is followed by `(`, a possibly empty comma-separated list
of **plain names**, `)`, and then the end of the line. Anything else is an ordinary read of a
variable called `LAMBDA`. An array subscript is an expression; a parameter list is a list of
names; that tells the two apart without reserving the word. `LAMBDA` is matched as a whole word,
because it is read in expression position where `LAMB` has to stay available; `ENDLAMBDA`
abbreviates to four letters as every block terminator does.

### 2. The body

The body is the newline-terminated statements between the `LAMBDA(...)` line and `ENDLAMBDA`.
The expression parser stops treating a newline as the end of the statement for as long as the
body lasts, and recovers at the lambda's own `ENDLAMBDA` rather than swallowing the rest of the
file. A body that never closes is one error at the `LAMBDA`; a bad line inside a body is one
error, not a cascade. `ENDLAMBDA)` is the ordinary shape, because the lambda usually sits inside
an argument list that still has to close.

`ENDLAMBDA` outside a lambda body is not a keyword at all: `ENDLAMBDA = 7` is legal FoxPro,
measured, and a word that closes nothing must not stop a program that uses it as a name.

`LPARAMETERS` and `PARAMETERS` inside a body are a syntax error. The parameters are written in
`LAMBDA(...)` and a second declaration has nowhere to mean anything.

### 3. Capture

A name in the body is resolved in this order, and the order is the whole of the scope rule:

1. a parameter of the lambda;
2. a `LOCAL` of the lambda;
3. a `LOCAL` of an enclosing routine, which is **captured by value when the lambda is made**;
4. anything else, which stays dynamic and is resolved **when the lambda runs**, against the
   private chain and the globals live then.

The third rule is forced. A `LOCAL` is a slot in a frame, the frame is gone by the time a
handler runs, and FoxPro has no reference to a local to keep instead. So the lambda gets a slot
of its own and the value is copied into it at the moment `LAMBDA` is evaluated. Two things
follow, and both are tested: a later change to the outer local is not seen by the lambda, and an
assignment to the captured name inside the lambda changes the lambda's copy, not the outer
variable.

```foxpro
LOCAL n, f
n = 1
f = LAMBDA()
    n = n + 10
    RETURN n
ENDLAMBDA
n = 2
? f()      && 11: the lambda captured 1 when it was made
? n        && 2: the lambda's copy is its own
```

Every `LOCAL` of the enclosing routine is captured, not only the ones the body mentions. It
costs a copy of each and it is what makes the chain work at any depth: a lambda inside a lambda
captures from the middle one's frame, and the middle one has the outer routine's locals in it.

The fourth rule is what dynamic scope already means: a `PRIVATE` set up before the lambda runs is
what the lambda sees, whoever set it up. It is also how recursion works - by way of a `PRIVATE`
or a `PUBLIC`, never a `LOCAL`, because a `LOCAL` holding the lambda is captured before the
lambda exists and the copy would be `.F.`.

`THIS` is captured with the lambda, so a lambda written inside a method keeps its object.
`THISFORM` and `THISFORMSET` are worked out from the captured `THIS` when the lambda runs, which
is what they already are inside a method.

### 4. The value

`Value` gains a function: an id into a table the VM owns, holding the module, the function
index, the captured slot values and the captured `THIS`.

- `VARTYPE()` answers **"F"**. Measured, the letters `vfp9.exe` can return are C, N, Y, D, T,
  L, O, G, Q, X and U, so "F" is free; a Float field answers "N" there, so it is not that
  letter either. `TYPE()` answers "F" too, and the type name in an error message is "Function".
- `EVALUATE()`, `STORE`, an array element and an object property all hold one. A function value
  crossing to the host travels as its id, the way an object travels as its handle.
- `=` and `==` between two function values are true when they are the same function value,
  which mirrors objects, measured.
- A function value is kept for the life of the runtime, like a compiled procedure; `RELEASE`
  does not free one. That is a deliberate simplification, and the reason to make lambdas where a
  procedure would be written rather than inside a loop.

### 5. Calling one

`f(a, b)` where `f` holds a function value calls it. The name is read exactly as it is today: a
name holding an array is subscripted, a name holding a function is called, and a name holding
anything else still looks for a program of that name and still fails with error 1, "File 'f.prg'
does not exist.", measured.

Inside the body, a lambda is a procedure:

- missing arguments are padded with `.F.` (measured); `PCOUNT()` answers how many were passed;
- `RETURN` with no value returns `.T.` (measured), and so does running off the end;
- a `@` argument is by reference, because a lambda parameter is a parameter like any other.

Two shapes are worth knowing:

- **A lambda that is not in a name is called where it stands.** `laRoutes[1, 2](req, res)` and
  `make()(20)` both work. The rule is narrow on purpose: a `(` calls the value only after a
  subscript or a call, where the expression already ended in a bracket. Measured, a `(` there is
  error 36 in the product, so no program that runs there is read differently here.
- **A lambda kept in a property is read into a name first.** `oForm.Handler(x)` is a call of the
  method `Handler` in the product and stays one here, so `f = oForm.Handler` and then `f(x)` is
  the form that works. `oForm.aHandlers[1](x)` does work, because it ends in a bracket.

## The FoxScript namespace

One global, `FoxScript`, and everything hangs off it. It is answered by the VM rather than the
host, and the reason is `CursorToJson`: a cursor is in the VM's own data engine and the host has
no way to read one. The VM makes the values; the host makes the machine.

```foxpro
? FoxScript.Version                    && the runtime's version
oServer = FoxScript.Http.CreateServer()
oData   = FoxScript.Json.Parse('{"a": [1, 2, 3]}')
cText   = FoxScript.Data.CursorToJson("c_cust")
```

### The rules it follows

- **`FoxScript` is a name only when nothing else in scope answers to it.** A program that already
  has a variable, private or public called `FoxScript` keeps working and keeps meaning what it
  meant; giving the variable up gives the namespace back. The same rule keeps `LAMBDA` a
  variable name.
- **`VARTYPE()` answers "O".** It is an object to the programmer.
- **`AMEMBERS()` answers honestly**, every member marked native and every property read-only,
  which they are. `GETPEM()` reads one the way it reads any other, so the namespace can be
  discovered from the Command Window.
- **A program may not write to it.** A member it has raises 1743, "X is a read-only property.";
  a member it has not raises the product's own 1925, "Unknown member X.". Nothing reaches the
  host with a handle the host has never heard of.
- **FoxScript's own errors are numbered from 3001.** Visual FoxPro's numbers stop a long way
  below that, so a program can tell one of ours from one of the product's by the number alone.

### The members

| Object | Members |
| --- | --- |
| `FoxScript` | `Version`, `Http`, `Data`, `Json`. |
| `FoxScript.Http` | `CreateServer()`; see [the HTTP API](/docs/http-api). |
| `FoxScript.Data` | `CursorToJson(cAlias)`, `JsonToCursor(vJsonOrText, cAlias)`. |
| `FoxScript.Json` | `Parse`, `Stringify`, `Count`, `Keys`, `Has`, `Get`. |

`FoxScript.Fs`, `FoxScript.Process` and `FoxScript.Crypto` are later, and only where the product
has no answer already. `FoxScript.Crypto` in particular must not duplicate what a `.fll` does:
hosting libraries was done to keep the legacy road open.

## The JSON value

A JSON value is not a FoxPro type and needs one. `Empty` plus `ADDPROPERTY` is close and is not
it: no arrays, no null, no order. So `Value` gains a JSON value - an ordered map, arrays,
numbers, strings, booleans and null - built on `serde_json` with key order preserved, because
that is the crate for this and hand-rolling a parser is the thing the project's rules forbid.

- **`VARTYPE()` answers "J"**, chosen the way "F" was: free in the product. The type name in a
  message is "Json".
- **A member is read by name**, case-insensitively, because every other member read in this
  language is: `oData.name` and `oData.Name` are the same key. Where two keys differ only in
  case, the first in document order wins.
- **An element is read by subscript**, one-based like everything else: `oData[1]` and
  `oData(1)`. Outside the array is the product's own invalid-subscript error.
- **What comes out is a FoxPro value where JSON has one**: a string is character, a number is
  numeric, true and false are logical, null is `.NULL.`. An object or an array comes out as
  another JSON value, so reaching goes on: `oData.items[1].name`.
- **Two JSON values are equal when they hold the same thing.** There is nothing to be identical
  to, so `=` compares what is in them.
- **Crossing to the host it goes as its text**, so `res.Json(oData)` works as readily as
  `res.Json(cText)`.

`FoxScript.Json` has six members and no more:

| Member | Answers |
| --- | --- |
| `Parse(cText)` | The value, or error 3001 with the parser's own complaint. |
| `Stringify(vAny [, lPretty])` | The text. It takes any FoxPro value, so a number, a logical, a date or an array can be written out. |
| `Count(vAny)` | How many members an object has or elements an array has; 0 for anything else. |
| `Keys(vObject)` | A JSON array of the names, in document order. |
| `Has(vObject, cName)` | A logical. |
| `Get(vObject, cName)` | The member, or `.NULL.` when there is none. `oData.name` for a name that is not there raises unknown-member, as a property would; `Get` asks rather than insists. |

## The data bridge

`FoxScript.Data.CursorToJson(cAlias)` answers **text**, because the name says so and because what
a program does with it next is put it on a socket. `FoxScript.Json.Parse()` is there for a
program that wants to reach into it. `JsonToCursor(vJsonOrText, cAlias)` builds a cursor and
answers how many records went into it.

The mapping is written down because nothing can be measured for it.

**A cursor to JSON.** An array with one object per record, in record order. A deleted record is
left out when `SET DELETED` is ON, as `SCAN` already does. Key names are the field names
**lower-cased**, the same choice the product makes when it writes DIF and SYLK, measured.
Character and memo fields become strings with their trailing blanks trimmed. Numeric, float,
integer, double and currency become numbers. Logical becomes true or false. A date becomes
`"YYYY-MM-DD"` and a datetime `"YYYY-MM-DDTHH:MM:SS"`. A general or blob field becomes null; a
null field is null.

**JSON to a cursor.** The value must be an array of objects, or one object, which becomes one
record. The fields are the union of the keys, in the order first seen. Each field's type comes
from the first non-null value seen for it: a string becomes character as wide as the widest
value, or a memo past 254; a number becomes numeric twenty wide with as many decimals as the
widest value needs; a boolean becomes logical; an object or array becomes a memo holding its
text. A field whose every value is null becomes logical, which is what `vfp9.exe` says a bare
`.NULL.` is, measured. A null value becomes the empty value of its field's type, because the
fields this makes are not nullable.

## What did not change

Nothing about calls to procedures, built-ins or DLL functions. Nothing about `PRIVATE`,
`PUBLIC` or `LOCAL`. Nothing about any word that was a name before. The lexer gained no tokens:
`LAMBDA` and `ENDLAMBDA` are ordinary identifiers to it, as every keyword in this language is,
and the parser recognises them by position alone.
