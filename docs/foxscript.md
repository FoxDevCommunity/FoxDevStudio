# FoxScript: the milestone after parity

## What it is, and the rule that comes first

FoxScript is Visual FoxPro 9 with things added: lambdas, a standard library that reaches the
machine the way a program written this decade expects to, and tables that are not stopped by two
gigabytes. It is a superset. Every program that runs in the product must keep running here and
keep meaning the same thing, which is why this milestone comes **after** parity and not beside
it: a language you are still discovering cannot be extended honestly, because you cannot tell an
extension from a gap.

The whole project's method is to ask vfp9.exe. FoxScript has no vfp9.exe to ask. That changes how
this milestone is built, and it is the most important sentence in this document: **where there is
a product to measure, measure it; where there is not, write the specification down first and make
the tests the specification.** An invented behaviour that nobody wrote down is how a superset
turns into a dialect nobody can predict.

## The program this is aiming at

```foxpro
SET LIBRARY TO "vfpencryption71.fll" ADDITIVE      && the 32-bit shim hosts it

LOCAL oServer
oServer = FoxScript.Http.CreateServer()

oServer.Get("/api/v1/customers/:id", LAMBDA(req, res)
    LOCAL lnId
    lnId = VAL(req.Params("id"))

    SELECT * FROM customer WHERE cust_id = lnId INTO CURSOR c_cust

    IF RECCOUNT("c_cust") > 0
        res.Status(200).Json(FoxScript.Data.CursorToJson("c_cust"))
    ELSE
        res.Status(404).Json('{"error": "Not Found"}')
    ENDIF

    USE IN c_cust
ENDLAMBDA)

oServer.Listen(8080)
READ EVENTS
```

Four things in it do not exist yet: the lambda, the `FoxScript` namespace, an HTTP server, and
`CursorToJson`. One of them already does - `SET LIBRARY TO` hosts a real `.fll` through the
32-bit shim, so a request handler can call into legacy code today.

## What a lambda costs in this language

FoxPro is line-oriented, case-insensitive, comments with `&&`, and continues a line with `;`.
Nothing in it is an expression that contains statements. A lambda is.

**Why `LAMBDA(...) ... ENDLAMBDA` and not `(...) => { ... }`.** Every block in this language is a word and
its matching end: `IF`/`ENDIF`, `DO CASE`/`ENDCASE`, `FOR`/`ENDFOR`, `SCAN`/`ENDSCAN`,
`TRY`/`ENDTRY`, `WITH`/`ENDWITH`, `TEXT`/`ENDTEXT`, `DEFINE CLASS`/`ENDDEFINE`,
`PROCEDURE`/`ENDPROC`. A brace block would be the only C-shaped thing in it, and braces are not
free anyway: `{^2024-01-31}` is a date literal and `{}` an empty one, so the lexer would have to
tell a block from a date. The word form costs nothing and reads like the language.

`ENDLAMBDA` rather than a bare `END`, and that is measured rather than chosen: `END` at the start
of a statement is **already** the beginning of `END TRANSACTION` - the product answers a bare one
with error 1221, "Command is missing required clause." `ENDLAMBDA` collides with nothing.

**Lexing.** Nothing new. `LAMBDA` and `ENDLAMBDA` are ordinary identifiers to the lexer, as every
keyword in this language is.

**Parsing, and the one real ambiguity.** `END` and `LAMBDA` are both legal variable and field
names in the product - `END = 5` and `tst.lambda` both work, measured - so neither may become
reserved. A keyword here is positional, as the rest of them are, and `LAMBDA` has to be
recognised in *expression* position, which is exactly where a variable of that name would be
read. `LAMBDA(1)` is therefore ambiguous with reading element 1 of an array called `LAMBDA`.

The rule that settles it, and it must be written down before it is coded: `LAMBDA` opens a lambda
only when the closing bracket of its parameter list is followed by the end of the line. An array
reference or a call is followed by an operator, a comma or a bracket. A program that has an array
called `LAMBDA` keeps working.

**The line-orientation is the hard part, and no spelling fixes it.** A lambda body is a list of
newline-terminated statements sitting inside an argument list, so the expression parser has to
stop treating a newline as the end of the statement for as long as the body lasts. That is true
of every spelling; `ENDLAMBDA` simply does not make it worse. The parser is already
error-recovering, and a lambda body must recover at its own `ENDLAMBDA` rather than swallowing
the rest of the file.

**A short form** - one expression, no body - is worth having for `ASORT`-shaped work and can wait.
`LAMBDA(x) RETURN x * 2 ENDLAMBDA` says it already, and a second spelling is a second thing to
document.

**Scope, which is the hard part.** `LOCAL` is a slot in a frame; `PRIVATE` and an undeclared name
are dynamic, found by walking the caller chain. A lambda outlives the frame it was written in, so:

- a `LOCAL` the lambda closes over must be **captured by value when the lambda is made**, because
  the frame it lived in is gone by the time the lambda runs, and FoxPro has no reference to a
  local to capture instead;
- a `PRIVATE` or undeclared name keeps its dynamic meaning and is resolved **when the lambda
  runs**, against the chain that is live then - which is what dynamic scope already means and is
  the least surprising answer for a FoxPro programmer;
- `THIS`, `THISFORM` and `THISFORMSET` are captured with the lambda when it is written inside a
  method, because a handler that loses its object is useless.

That split needs to be in the documentation before it is in the compiler, and each half needs a
test that fails if it is swapped.

**Values.** `Value` gains a function: a module id, a function index, and the captured frame. It
must answer `VARTYPE()` with something - the product has no letter for it, so FoxScript defines
one and says so - and `TYPE()`, `EVALUATE()`, `STORE`, an array element and a property must all
be able to hold one.

## The lambda, settled

Everything above is the design. What follows is the specification: written down before it was
coded, and each numbered point has a test that fails if the point is changed. Where Visual
FoxPro already answers the question the answer is the product's and says "measured"; the rest is
FoxScript's own and says so.

### 1. When `LAMBDA` opens a lambda

`LAMBDA` opens a lambda only when it is followed by `(`, a possibly empty comma-separated list
of plain names, `)`, and then the end of the line. Anything else is an ordinary read of a
variable called `LAMBDA`.

The rule the design gives - the closing bracket at the end of the line - is not quite enough on
its own. `x = LAMBDA(1)` at the end of a line is a perfectly good read of element 1 of an array
called `LAMBDA`, so the parameter list has to look like a parameter list too. An array
subscript is an expression; a parameter list is a list of names. That distinguishes them
without reserving the word, and `LAMBDA = 5`, `DIMENSION lambda(3)`, `lambda(1)` and
`tst.lambda` all keep working. Measured in vfp9.exe: `LAMBDA` and `ENDLAMBDA` are legal variable
names, legal field names and legal property names there, so neither may become reserved here.

`LAMBDA` is matched as a whole word. FoxPro abbreviates its command verbs to four letters, but
this one is read in expression position, where `LAMB` has to stay available as a variable name.
`ENDLAMBDA` is a block terminator like `ENDIF` and abbreviates to four letters as they all do.

### 2. The body

The body is the newline-terminated statements between the `LAMBDA(...)` line and `ENDLAMBDA`.
The expression parser stops treating a newline as the end of the statement for as long as the
body lasts, and recovers at the lambda's own `ENDLAMBDA` rather than swallowing the rest of the
file: a body that does not close is reported as "LAMBDA without matching ENDLAMBDA" at the
`LAMBDA`, a bad line inside a body is one error and not a cascade, and the procedure below the
lambda is still read. `ENDLAMBDA` outside a lambda body is not a keyword at all - it is read as
the statement it looks like, because `ENDLAMBDA = 7` is legal FoxPro, measured, and a word that
closes nothing must not stop a program that uses it as a name.

`ENDLAMBDA` does not have to end its line, because it stands inside an argument list and the
call that holds it has still to be closed: `ENDLAMBDA)` is the ordinary shape.

`LPARAMETERS` and `PARAMETERS` inside a lambda body are a syntax error. The parameters are
written in `LAMBDA(...)` and there is nowhere for a second declaration to mean anything. This is
FoxScript's own rule; the product's own answer to a misplaced declaration is error 1238, "No
PARAMETER statement is found." (measured), which is about a different mistake.

### 3. Capture

A name in the body is resolved in this order, and the order is the whole of the scope rule:

1. a parameter of the lambda;
2. a `LOCAL` of the lambda;
3. a `LOCAL` of an enclosing routine, which is **captured by value when the lambda is made**;
4. anything else, which stays dynamic and is resolved **when the lambda runs**, against the
   private chain and the globals that are live then.

The third rule is forced: a `LOCAL` is a slot in a frame, the frame is gone by the time a
handler runs, and FoxPro has no reference to a local to keep instead. So the lambda gets a slot
of its own and the value is copied into it at the moment `LAMBDA` is evaluated. Two consequences
follow and both are tested: a later change to the outer local is not seen by the lambda, and an
assignment to the captured name inside the lambda changes the lambda's copy and not the outer
variable.

Every `LOCAL` of the enclosing routine is captured, not only the ones the body mentions. It
costs a copy of each and it is what makes the chain work at any depth: a lambda inside a lambda
captures from the middle one's frame, and the middle one has the outer routine's locals in it to
be captured from. A name the body never reads is invisible either way, so the rule is the
simpler one to state and the simpler one to implement.

The fourth rule is the least surprising answer for a FoxPro programmer, because it is what
dynamic scope already means: a `PRIVATE` set up before the lambda runs is what the lambda sees,
whoever set it up.

`THIS` is captured with the lambda, so a lambda written inside a method keeps its object.
`THISFORM` and `THISFORMSET` are captured with it, in the sense that they are worked out from
the captured `THIS` when the lambda runs, which is what they already are inside a method.

A lambda inside a lambda captures through: the middle lambda captures what the inner one needs
from the outer frame, so the chain works at any depth.

### 4. The value

`Value` gains a function. It is an id into a table the VM owns, exactly as an object value is a
handle into a table the host owns, and the entry holds the module id, the function index, the
captured slot values and the captured `THIS`.

- `VARTYPE()` answers **"F"**. The product has no letter for a function; measured in vfp9.exe,
  the letters it can return are C, N, Y, D, T, L, O, G, Q, X and U, so "F" is free. "F" is not
  the letter of a Float field: a Float field answers "N" there, measured.
- `TYPE()` answers "F" for an expression that evaluates to one, and the type name in an error
  message is "Function".
- `EVALUATE()`, `STORE`, an array element and an object property all hold one. A function value
  crossing to the host travels as its id, the way an object travels as its handle, and comes
  back as the same function.
- `=` and `==` between two function values are true when they are the same function value. That
  mirrors objects, measured: `o1 = o2` in vfp9.exe is .T. for two names for one object and .F.
  for two objects.
- A function value is kept for the life of the runtime, like a compiled procedure. `RELEASE`
  does not free one. This is a deliberate simplification and the reason to make lambdas where a
  procedure would be written, not inside a loop.

### 5. Calling one

`f(a, b)` where `f` holds a function value calls it. The name is read exactly as it is today: a
name that holds an array is subscripted, a name that holds a function is called, and a name that
holds anything else still looks for a program of that name and still fails with error 1, "File
'f.prg' does not exist." (measured). Nothing about calls to procedures, built-ins or DLL
functions changes.

Inside the body:

- missing arguments are padded with `.F.`, measured: a VFP procedure called with one of three
  parameters sees `.F.` in the other two;
- too many arguments raise whatever a procedure call raises, because a lambda is called by the
  same code. Today that is error 1230, "Too many arguments."; vfp9.exe answers 94, "Must specify
  additional parameters.", measured, which is a parity question about procedure calls and not
  about lambdas. The two are asserted together so they cannot drift apart;
- `PCOUNT()` answers how many arguments were actually passed, measured;
- `RETURN` with no value returns `.T.`, measured; so does running off the end of the body;
- `RETURN` inside `TRY`/`CATCH` does whatever it does in a procedure body, for the same reason.
  Measured, vfp9.exe refuses it with error 2060, "RETURN/RETRY statement not allowed in
  TRY/CATCH.", after running the `FINALLY`; this runtime lets it return. Again a parity question
  about routine bodies, and again the two are asserted together;
- a `@` argument is by reference, because a lambda parameter is a parameter like any other;
- **a lambda that is not in a name is called where it stands**: `laRoutes[1, 2](req, res)` and
  `make()(20)` both work. A route table holding handlers is the obvious shape for one, and this
  is how a programmer reaches for it. The rule is narrow on purpose: a `(` calls the value only
  after a subscript or a call - only where the expression already ended in a bracket. Measured
  in vfp9.exe, a `(` there is error 36, "Command contains unrecognized phrase/keyword.", so no
  program that runs in the product is read differently here;
- **a lambda kept in a property is read into a name first**: `oForm.Handler(x)` is a call of the
  method `Handler` in the product and stays one here, so `f = oForm.Handler` and then `f(x)` is
  the form that works. Where the product already has an answer, the product wins - which is the
  rule this whole milestone follows, and the reason `oForm.aHandlers[1](x)` does work: that one
  ends in a bracket, where the product has no answer;
- recursion works by way of a name the lambda can still see when it runs, which means a
  `PRIVATE` or a public, not a `LOCAL`: a `LOCAL` holding the lambda is captured by value
  before the lambda exists, so the captured copy would be `.F.`. That falls out of rule 3 rather
  than being a rule of its own, and it is tested both ways.

## Calling back into a VM that is not re-entrant

This is where the architecture already helps, and where it will bite anyone who forgets it.

The wasm exports are **not re-entrant**: calling one while another is on the stack throws. The
whole runtime is built around that - property reads are synchronous imports, every side effect is
a yielded host request, and `Scheduler.drive` performs it while the VM is off the stack. Because
the VM is off the stack, `perform` may dispatch a nested VFP event and it runs to completion
before the outer resume. That is how `SetFocus()` fires `GotFocus`, and how a Timer fires while a
program is parked in `READ EVENTS`.

**How a lambda call stays inside the machinery.** A call in this VM is not a Rust call: `run` is
a loop over an explicit stack of frames, and a call pushes a frame and carries on round the
loop. A lambda call is the same push. There are three doors into it and all three go through one
helper, `push_function_call`, which reads the function value out of the VM's table, pushes a
frame for its module and function index, copies the arguments into the parameter slots and the
captured values into the capture slots, and returns:

- the call instruction, when a program writes `f(1, 2)`;
- `BuiltinResult::CallFunction`, when a built-in has been handed a function and wants its
  answer. The built-in returns rather than calling: the VM pushes the frame and what the lambda
  returns becomes the built-in's result, exactly as `EXECSCRIPT()` already works through
  `BuiltinResult::RunScript`. A built-in cannot run FoxPro itself, because the code it runs may
  stop for the host half way through;
- `Vm::start_function`, when the host wakes the runtime with an event. That one starts a fiber
  whose first frame is the lambda.

Nothing on any of those paths re-enters a wasm export, and nothing recurses into the
interpreter.

An HTTP request is the same shape, arriving from a different direction. The server lives in the
main process, where the sockets are; a request crosses to the runtime as an event, and the handler
lambda is **dispatched** exactly as a Click is dispatched - never called directly from inside a
host request. One request is one fiber. The scheduler already serialises fibers, which means:

- a handler runs to completion or parks; two requests do not interleave inside the VM;
- a handler that blocks - a long `SELECT`, a `MESSAGEBOX` - holds the queue, and that must be
  documented rather than discovered;
- `res` is a host object with a handle, like a form; writing to it after the request has finished
  is an error the host raises, not a crash.

`READ EVENTS` is what keeps the server alive, which is already how a FoxPro application waits.

### `HostEvent`, settled

A host event is a function value and the arguments to call it with: `Scheduler.raise({ func,
args })`. The function value is the id a lambda crossed the bridge as, so the host keeps a plain
number and hands it back, exactly as it keeps an object's handle.

- **An event that arrives while a fiber is on the JS stack is queued, not dispatched.** Starting
  a fiber there would be a re-entrant call into the wasm exports, and `vmBridge` refuses that by
  design. The queue drains the moment the stack unwinds, in arrival order. `Scheduler` already
  had `isExecuting` for exactly this and nothing had used it.
- **One handler runs to completion or parks before the next begins.** That is the scheduler's
  existing bargain and this does not change it, which means two requests never interleave inside
  the VM, and **a handler that blocks holds the queue**: a long `SELECT`, a `MESSAGEBOX`, a
  `WAIT WINDOW` with no timeout will stop every other request until it is done. That is written
  down here rather than left to be discovered.
- **`raise` answers with what the handler returned, or nothing at all** when the VM has no
  function of that id - a handler left over from a run that has been cancelled. Cancelling a
  session settles every queued event the same way rather than leaving the host waiting.

## The `FoxScript` namespace

One global, not two. The sketch above had both `FoxScript.Http` and `foxdev.Data`; there should be
a single root, `FoxScript`, and everything hangs off it:

- `FoxScript.Http`: `CreateServer()`, and a client: `Get`, `Post`, `Request`.
- `FoxScript.Data`: `CursorToJson`, `JsonToCursor`, and the JSON value itself.
- `FoxScript.Json`: parse and write, because a JSON value is not a FoxPro type and needs one.
- `FoxScript.Fs`, `FoxScript.Process`, `FoxScript.Crypto`: later, and only where the product has
  no answer already. `FoxScript.Crypto` must not duplicate what a `.fll` does; the point of
  hosting libraries was to keep the legacy road open.

Each is an object of the object model, so `THIS`, method calls and properties work with no new
machinery, and each must answer `AMEMBERS()` honestly so a programmer can discover it from the
Command Window.

### Where the namespace lives, and why it is not in the host

The sketch above said "a host object". It cannot be, and the reason is `CursorToJson`: a cursor
is in the VM's own data engine and the host has no way to read one - the data requests go the
other way, the host handing bytes to the VM. `FoxScript.Json` is the same, because a JSON value
is a `Value`. So the namespace is answered by the VM, and the things it makes - a server, a
request, a response - are ordinary host objects, because that is where the sockets are. The VM
makes the values; the host makes the machine.

Nothing new was needed to make `THIS`, a member read and a method call work on one. A native
object is an ordinary object value whose handle comes out of a range the host never allocates
from: the host counts up from 1 and reserves `0x7fff_ffff` for the application object, so
`0xF000_0000` upwards is the VM's. Every path that already carries an object carries this one,
and a handle that did escape to the host is refused by name rather than read as some other
object. There is a test that asserts the two ranges do not meet.

### The rules the namespace follows

- **`FoxScript` is a name only when nothing else in scope answers to it.** A program that
  already has a variable, private or public called `FoxScript` keeps working and keeps meaning
  what it meant; giving the variable up gives the namespace back. That is the same rule that
  keeps `LAMBDA` a variable name, and it is the rule this whole milestone follows: where the
  product already has an answer, the product wins.
- **`VARTYPE()` answers "O".** It is an object to the programmer and there is nothing to be
  gained by telling them otherwise.
- **`AMEMBERS()` answers honestly**, with every member marked native and every property marked
  read-only, which they are. `GETPEM()` reads one the way it reads any other.
- **A program may not write to it.** A member it has raises error 1743, "X is a read-only
  property."; a member it has not raises the product's own 1925, "Unknown member X.". Reading or
  calling a member that is not there raises 1925 as well, rather than reaching the host with a
  handle the host has never heard of.

Step two of the order below gives the root one member, `Version`, so that discovery,
`AMEMBERS()` and `VARTYPE()` are settled before there is anything interesting to discover.

**FoxScript's own errors are numbered from 3001.** Visual FoxPro's numbers stop a long way below
that, so a program can tell one of ours from one of the product's by the number alone, and
nothing here will ever collide with a number the product may yet use. The message says which
failure it was, as the product's own messages do.

## `FoxScript.Http`, settled

```foxpro
oServer = FoxScript.Http.CreateServer()
oServer.Get("/api/v1/customers/:id", LAMBDA(req, res)
    res.Status(200).Json('{"id": 7}')
ENDLAMBDA)
oServer.Listen(8080)
READ EVENTS
```

### Where each piece lives

- **The sockets are in the main process**, on Node's own `http` and nothing else. That file
  knows nothing about routes, lambdas or the VM: open a port, hand over what arrived, write back
  what came.
- **The routes are in the VM**, because that is where the program registered them, and because a
  pattern language wants one implementation and not two. `matchit` is that implementation;
  `:id` in a route is its syntax, not ours.
- **The seam is one read.** The moment a request arrives the host asks the VM which lambda it
  matches - a plain export like a property read, with the VM off the stack - and then dispatches
  that lambda as a host event. Nothing calls into the VM from inside a host request.

### The objects

`FoxScript.Http.CreateServer()` answers with a server: an object of the namespace, so
`AMEMBERS()` lists it honestly.

- `Get`, `Post`, `Put`, `Patch`, `Delete`, `Head`, `Options` each register one route, and each
  answers with the server, so registrations chain. A route registered twice under one method is
  refused with error 3001 rather than silently replaced, because a program that does it has two
  answers for one question.
- `Listen(nPort)` answers with the port actually bound; `0` asks for any free one, which is what
  a test wants. `Port` reads it back, and is `.F.` before the server is listening - a server
  with no port has none, and the product says that with `.F.` rather than with a number nobody
  may use.
- `Close()` stops listening. Cancelling a run closes every server it opened: a port that
  outlived its program would be one nobody could close.

`req` and `res` are host objects with handles, like a form.

- `req.Method`, `req.Path` and `req.Body` are properties. `req.Params(cName)`, `req.Query(cName)`
  and `req.Header(cName)` are lookups by name, because a program does not know those names when
  it writes the handler and an object cannot have a property per request. A header name is
  matched without regard to case, which is what HTTP says.
- `res.Status(n)`, `res.Header(cName, cValue)`, `res.Json(cText)` and `res.Send(cText)` each
  answer with the response, so `res.Status(200).Json(...)` reads the way it does everywhere else
  this shape appears. `Json` and `Send` write the body and finish the response; `Json` sets the
  content type to `application/json` and `Send` to `text/plain` unless the handler set one.
- **Writing to a response that has already been sent is an error the host raises**, catchable
  like any other, not a crash and not a second answer on the wire.

### What one request costs

One request is one fiber, and the scheduler serialises fibers. So:

- a handler runs to completion or parks, and two requests never interleave inside the VM;
- **a handler that blocks holds the queue**. A long `SELECT`, a `MESSAGEBOX`, a `WAIT WINDOW`
  with no timeout stops every other request until it is done. That is the price of the
  single-threaded model the whole runtime is built on, and it is written down here rather than
  left to be discovered;
- a path no route matches never wakes the runtime at all: the host answers it 404 itself.
- a handler that throws leaves the client a 500 rather than nothing, because a socket cannot
  wait for a developer. The error is still the runtime's to report, in the usual place.

## `FoxScript.Json` and the data bridge, settled

### The JSON value

A JSON value is not a FoxPro type and needs one. `Empty` plus `ADDPROPERTY` is close and is not
it: no arrays, no null, no order. So `Value` gains a JSON value - an ordered map, arrays,
numbers, strings, booleans and null - built on `serde_json`, because that is the crate for this
and hand-rolling a JSON parser is the thing the rules forbid.

- **`VARTYPE()` answers "J".** FoxScript's own letter, chosen the way "F" was: measured, the
  letters vfp9.exe can return are C, N, Y, D, T, L, O, G, Q, X and U, so "J" is free. The type
  name in a message is "Json".
- **A member is read by name**: `oData.name`. Case-insensitively, because every other member
  read in this language is, and a programmer who writes `oData.Name` for a key spelled `name` is
  doing the ordinary FoxPro thing. Where an object has two keys differing only in case, the
  first in document order wins.
- **An element is read by subscript**: `oData[1]` and `oData(1)`, one-based like every other
  subscript in the language. A subscript outside the array is the product's own invalid
  subscript error.
- **What comes out is a FoxPro value where JSON has one**: a string is character, a number is
  numeric, true and false are logical, null is `.NULL.`. An object or an array comes out as
  another JSON value, so reaching goes on: `oData.items[1].name`.
- **Two JSON values are equal when they hold the same thing.** Unlike a lambda or an object,
  there is nothing to be identical to: a JSON value has no methods and no identity worth
  keeping, so `=` compares what is in them.
- **A JSON value crossing to the host goes as its text**, which is what a host that wants to
  write it on a socket needs. `res.Json(oData)` therefore works as readily as
  `res.Json(cText)`.

`FoxScript.Json` has six members and no more:

- `Parse(cText)` answers the value, or raises 3001 with the parser's own complaint.
- `Stringify(vAny [, lPretty])` answers the text. It takes any FoxPro value, not only a JSON
  one, so a number, a logical, a date or an array can be written out.
- `Count(vAny)` is how many members an object has or how many elements an array has; 0 for
  anything else.
- `Keys(vObject)` answers a JSON array of the names, in document order.
- `Has(vObject, cName)` answers a logical.
- `Get(vObject, cName)` answers the member, or `.NULL.` when there is none. Reading a name an
  object has not got through `oData.name` raises the product's unknown-member error, as reading
  a property an object has not got does; `Get` is the form that asks rather than insists.

### `FoxScript.Data`

- `CursorToJson(cAlias)` answers **text**, not a value, because the name says so and because
  what a program does with it next is put it on a socket. `FoxScript.Json.Parse()` is there for
  a program that wants to reach into it.
- `JsonToCursor(vJsonOrText, cAlias)` builds a cursor and answers how many records went into it.

The mapping, both ways, is written down here because nothing can be measured for it:

**A cursor to JSON.** An array with one object per record, in record order. A deleted record is
left out when `SET DELETED` is ON, which is what `SCAN` already does. Key names are the field
names **lower-cased** - the same choice Visual FoxPro itself makes when it writes DIF and SYLK,
measured, and shouting keys are not what anyone wants. Character and memo fields become strings
with their trailing blanks trimmed, because a DBF pads to the field width and JSON should not
carry the padding. Numeric, float, integer, double and currency fields become numbers. Logical
becomes true or false. A date becomes `"YYYY-MM-DD"` and a datetime `"YYYY-MM-DDTHH:MM:SS"`,
which is ISO 8601 and is what every client already reads. A general or blob field has no text of
its own and becomes null. A field that is null is null.

**JSON to a cursor.** The value must be an array of objects, or one object, which becomes one
record. The fields are the union of the keys, in the order they are first seen. Each field's
type comes from the first value seen for it that is not null: a string becomes character as wide
as the widest value for that field, or a memo past 254; a number becomes numeric twenty wide
with as many decimal places as the widest value for that field needs; a boolean becomes logical;
an object or an array becomes a memo holding its JSON text. A field whose every value is null
becomes logical, which is what vfp9.exe says a bare `.NULL.` is - measured, `VARTYPE(.NULL., .T.)`
answers "L" there. A null value becomes the empty value of its field's type, because the fields
this makes are not nullable.

## What has to exist underneath

- **JSON as a value.** A cursor turned into JSON needs an object that is not a FoxPro object: an
  ordered map with arrays, numbers, strings, booleans and null. `Empty` plus `ADDPROPERTY` is close
  and is not it (no arrays, no null, no order).
- **64-bit tables.** The two-gigabyte ceiling is in the DBF header format itself, so this is a new
  container, not a raised limit: a file format of our own that the same `USE`, `SCAN` and
  `SELECT` reach, with a documented conversion both ways. It is a milestone of its own and should
  not be smuggled into this one.
- **A real async seam.** Everything the host does today either answers at once or answers with a
  promise the scheduler awaits. A server is neither: it speaks first. `HostEvent` - the host
  waking the runtime - is the piece that does not exist yet, and Timer dispatch is its only
  precedent.

## What is already in place

Worth saying plainly, because it is more than it looks:

- `SET LIBRARY TO` hosts a real 32-bit `.fll` through `fllhost.exe`, proven against
  vfpencryption71.fll, foxtools.fll and two libraries built from Microsoft's own API samples.
- `DECLARE ... DLL` calls a 64-bit library in process through koffi.
- COM through the `foxole` addon.
- The object model, the scheduler's nested dispatch, fibers, `READ EVENTS`, cursors and SQL.

A request handler that opens a cursor, calls an `.fll` and answers is, apart from the lambda and
the server, made of things that already work.

## The order to build it in

1. **Lambdas in the language**: lexer, parser, a function value, the capture rules above, calling
   one from FoxPro code. No host, no server - a lambda stored in a variable and called is the
   whole of step one, and it is testable on its own.
2. **`FoxScript` as a namespace** with one harmless member, to settle discovery, `AMEMBERS()` and
   what `VARTYPE()` says about these objects.
3. **`HostEvent`**: the host waking the runtime with a payload, dispatched like a Timer. Prove it
   with something duller than a server.
4. **`FoxScript.Http`**, server first, on top of 1-3.
5. **`FoxScript.Json` and `CursorToJson`**, which need no new runtime machinery once there is a
   JSON value.
6. **64-bit tables**, separately, on their own merits.

Nothing here should start while `docs/coverage-plan.md` still has elements outstanding. The
measure of this project is the corpus and the product, and both are still talking.
