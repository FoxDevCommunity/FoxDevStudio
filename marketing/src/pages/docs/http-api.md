---
layout: ../../layouts/DocsLayout.astro
title: The HTTP API
kicker: FoxScript
description: FoxScript.Http from the program's side, the Node service that holds the sockets, the seam between them, and the reason for each decision.
---

A FoxPro application has always been able to answer a question about its own data; what it
never had was a way for something outside the machine to ask. `FoxScript.Http` is that: a
server a program opens, routes it registers with lambdas, and requests that arrive as events and
are answered from the code that already knows the business.

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

Apart from the lambda and the server, every line of that is ordinary FoxPro: the query, the
cursor, the library call. `READ EVENTS` is what keeps the server alive, which is already how a
FoxPro application waits.

[![A FoxScript program registering HTTP routes with lambdas, calling a 32-bit encryption library from inside one, and the Output window reporting that the server is listening on port 8080.](/screenshots/http-server.jpg)](/screenshots/http-server.jpg)

A longer one, running in the IDE: a hashing route that takes its algorithm from the path and
its text from the query string, an encryption route that parses a JSON body, and, from inside
the lambdas, a 32-bit `.fll` answering through the library host. The Output window has the
server's own report: listening on 8080, a 64-bit runtime serving a 32-bit library.

## The program's side

### The server

`FoxScript.Http.CreateServer()` answers with a server, an object of the namespace, so
`AMEMBERS()` lists it honestly and `VARTYPE()` says "O".

| Member | What it does |
| --- | --- |
| `Get`, `Post`, `Put`, `Patch`, `Delete`, `Head`, `Options` | Each registers one route under that method and answers with the server, so registrations chain. A route registered twice under one method is refused with error 3001, because a program that does it has two answers for one question. |
| `Listen(nPort)` | Opens the port and answers the port actually bound. `0` asks for any free one, which is what a test wants. |
| `Port` | The port, or `.F.` before the server is listening: a server with no port has none, and the product says that with `.F.` rather than a number nobody may use. |
| `Close()` | Stops listening. Cancelling a run closes every server it opened, because a port that outlived its program would be one nobody could close. |

Routes use `matchit`'s pattern language: `:id` names a segment. The routes live in the VM,
because that is where the program registered them and because a pattern language wants one
implementation, not two.

### The request

`req` is a host object with a handle, like a form.

| Member | What it answers |
| --- | --- |
| `req.Method` | `"GET"`, `"POST"`, ... |
| `req.Path` | The path with no query string, which is what the route was matched against. |
| `req.Body` | The body as text. |
| `req.Params(cName)` | A named segment of the route, `:id` for example. |
| `req.Query(cName)` | A query-string value. |
| `req.Header(cName)` | A header, matched without regard to case, which is what HTTP says. |

`Params`, `Query` and `Header` are lookups by name rather than properties because a program does
not know those names when it writes the handler, and an object cannot have a property per
request.

### The response

| Member | What it does |
| --- | --- |
| `res.Status(n)` | Sets the status code; answers the response. |
| `res.Header(cName, cValue)` | Sets a header; answers the response. |
| `res.Json(cTextOrJsonValue)` | Writes the body, sets `Content-Type: application/json` unless the handler set one, and finishes the response. |
| `res.Send(cText)` | The same with `text/plain`. |

Each answers with the response so `res.Status(200).Json(...)` reads the way that shape reads
everywhere else. **Writing to a response that has already been sent is an error the host
raises**, catchable like any other, not a crash and not a second answer on the wire.

### What one request costs

One request is one fiber, and [the scheduler](/docs/vm#the-scheduler) serialises fibers. So:

- a handler runs to completion or parks, and two requests never interleave inside the VM;
- **a handler that blocks holds the queue.** A long `SELECT`, a `MESSAGEBOX`, a `WAIT WINDOW`
  with no timeout stops every other request until it is done. That is the price of the
  single-threaded model the whole runtime is built on, and it is written here rather than left
  to be discovered;
- a path no route matches never wakes the runtime at all: the host answers 404 itself;
- a handler that throws leaves the client a 500 rather than nothing, because a socket cannot
  wait for a developer. The error is still the runtime's to report, in the usual place.

## The Node side

The sockets are in the Electron main process, on Node's own `http` module and nothing else.
`src/shared/runtime/httpService.ts` is the whole of that file, and it knows nothing about
routes, lambdas or the VM: open a port, hand over what arrived, write back what came. It imports
no Electron, so the tests drive it directly the way the real main process does.

```ts
export interface HttpService {
  /** Opens a port. `port` 0 asks for any free one; answers the port actually bound. */
  listen(server: number, port: number): Promise<number>;
  /** Stops listening. True when there was something to stop. */
  close(server: number): Promise<boolean>;
  closeAll(): Promise<void>;
  /** Installs what answers requests. The last one installed is the one that answers. */
  onRequest(handler: HttpHandler): void;
}

/** Answers a request, or `null` when nothing in the runtime would. */
export type HttpHandler = (request: HttpRequestIn) => Promise<HttpResponseOut | null>;
```

A request is reduced to a plain record before it crosses to the renderer, and a response is a
plain record on the way back:

```ts
export interface HttpRequestIn {
  server: number;                   // the handle the VM created the server under
  method: string;                   // upper-cased
  path: string;                     // no query string: what a route is matched against
  query: Record<string, string>;
  headers: Record<string, string>;  // names lower-cased, which is how HTTP treats them
  body: string;
}

export interface HttpResponseOut {
  status: number;
  headers: Record<string, string>;
  body: string;
}
```

Two answers are the service's own, so a client is never left hanging: a request nothing answered
gets `404 No route`, and a request the runtime failed on gets `500 Handler failed`. Servers
bind to `127.0.0.1`.

### The IPC channels

Four channels carry it between the main process and the renderer, all typed in
`src/shared/ipc/channels.ts` and exposed through the sandboxed preload as part of `FoxDevApi`:

| Channel | Direction | Shape |
| --- | --- | --- |
| `http:listen` | renderer asks main | `(server, port) -> port bound` |
| `http:close` | renderer asks main | `(server) -> boolean` |
| `http:request` | main tells renderer | `(id, HttpRequestIn)` - a request arrived |
| `http:respond` | renderer tells main | `(id, HttpResponseOut \| null)` - here is the answer |

`http:request` is the unusual one: it is the host speaking first. Everything else the host does
either answers at once or answers with a promise the scheduler awaits; a server is neither, and
the piece that makes it possible is the host event described next.

## The seam

<div class="diagram-frame">
<div class="diagram-bar"><b>GET /api/v1/customers/7</b><span>/</span><span>from the socket to the lambda and back</span></div>
<svg class="diagram" viewBox="0 0 680 400" role="img" aria-label="Two lanes. In the main process: the socket, then httpService, which sends http:request over IPC. In the renderer: the session asks the VM which lambda the route matches, a read with the VM off the stack; a miss is answered 404 by the host without waking the runtime; a hit is raised through the scheduler as a host event, which starts a fiber whose first frame is the lambda; res.Json finishes the response, which goes back over http:respond to httpService and out on the socket.">
  <defs>
    <marker id="http-tip" viewBox="0 0 10 10" refX="9" refY="5" markerWidth="6.5" markerHeight="6.5" orient="auto-start-reverse"><path d="M0 0 L10 5 L0 10 z" class="dg-tip"></path></marker>
    <marker id="http-tip-lead" viewBox="0 0 10 10" refX="9" refY="5" markerWidth="6.5" markerHeight="6.5" orient="auto-start-reverse"><path d="M0 0 L10 5 L0 10 z" class="dg-tip-lead"></path></marker>
  </defs>
  <text class="dg-edge" x="24" y="20">MAIN PROCESS</text>
  <text class="dg-edge" x="24" y="176">RENDERER</text>
  <line class="dg-split" x1="24" y1="156" x2="656" y2="156"></line>
  <rect class="dg-box" x="24" y="34" width="150" height="52" rx="5"></rect>
  <text class="dg-name" x="44" y="56">The socket</text>
  <text class="dg-sub" x="44" y="73">node:http, on 127.0.0.1</text>
  <line class="dg-line" x1="174" y1="60" x2="222" y2="60" marker-end="url(#http-tip)"></line>
  <rect class="dg-box" x="224" y="34" width="190" height="52" rx="5"></rect>
  <text class="dg-name" x="244" y="56">httpService.ts</text>
  <text class="dg-sub" x="244" y="73">method, path, query, headers, body</text>
  <line class="dg-line" x1="414" y1="60" x2="462" y2="60" marker-end="url(#http-tip)"></line>
  <text class="dg-flow" x="438" y="50" text-anchor="middle">IPC</text>
  <rect class="dg-box" x="464" y="34" width="192" height="52" rx="5"></rect>
  <text class="dg-name" x="484" y="56">http:request (id, req)</text>
  <text class="dg-sub" x="484" y="73">the host speaking first</text>
  <line class="dg-line" x1="560" y1="86" x2="560" y2="192" marker-end="url(#http-tip)"></line>
  <rect class="dg-box" x="464" y="194" width="192" height="52" rx="5"></rect>
  <text class="dg-name" x="484" y="216">session.onRequest</text>
  <text class="dg-sub" x="484" y="233">one read: vm.route(GET, path)</text>
  <line class="dg-line" x1="464" y1="220" x2="416" y2="220" marker-end="url(#http-tip)"></line>
  <rect class="dg-box-lead" x="224" y="194" width="190" height="52" rx="5"></rect>
  <text class="dg-name" x="244" y="216">The VM answers</text>
  <text class="dg-sub" x="244" y="233">function id 3, params { id: "7" }</text>
  <line class="dg-split" x1="319" y1="246" x2="319" y2="282"></line>
  <text class="dg-flow" x="330" y="262">no match: the host writes 404 itself;</text>
  <text class="dg-flow" x="330" y="276">the runtime never wakes</text>
  <line class="dg-line-lead" x1="224" y1="220" x2="176" y2="220" marker-end="url(#http-tip-lead)"></line>
  <rect class="dg-box-lead" x="24" y="194" width="150" height="52" rx="5"></rect>
  <text class="dg-name" x="44" y="216">scheduler.raise</text>
  <text class="dg-sub" x="44" y="233">func 3, args [req, res]</text>
  <line class="dg-line-lead" x1="99" y1="246" x2="99" y2="292" marker-end="url(#http-tip-lead)"></line>
  <text class="dg-flow" x="110" y="284">queued if a fiber is on the stack</text>
  <rect class="dg-box-lead" x="24" y="294" width="390" height="52" rx="5"></rect>
  <text class="dg-name" x="44" y="316">A fiber whose first frame is the lambda</text>
  <text class="dg-sub" x="44" y="333">SELECT ... INTO CURSOR, then res.Status(200).Json(...)</text>
  <line class="dg-line" x1="414" y1="320" x2="462" y2="320" marker-end="url(#http-tip)"></line>
  <rect class="dg-box" x="464" y="294" width="192" height="52" rx="5"></rect>
  <text class="dg-name" x="484" y="316">http:respond (id, res)</text>
  <text class="dg-sub" x="484" y="333">status, headers, body</text>
  <path class="dg-line" d="M 656 320 L 668 320 L 668 120 L 99 120 L 99 88" marker-end="url(#http-tip)"></path>
  <text class="dg-flow" x="340" y="112" text-anchor="middle">written back on the same socket</text>
  <text class="dg-flow" x="340" y="384" text-anchor="middle">one request is one fiber; the next waits until this one finishes or parks</text>
</svg>
</div>

The moment a request arrives, three things happen, in this order, and none of them calls into
the VM from inside a host request:

1. **One read.** The host asks the VM which lambda the request matches: `route(server, method,
   path)`, a plain export like a property read, with the VM off the stack. It answers the
   function id and the named parts of the path, or nothing, in which case the host writes the
   404 itself.
2. **One event.** The host raises `{ func, args: [req, res] }` through the scheduler. A host
   event is a function value and the arguments to call it with; the function value is the id
   the lambda crossed the bridge as, so the host keeps a plain number and hands it back, exactly
   as it keeps an object's handle. `start_function` starts a fiber whose first frame is the
   lambda.
3. **One answer.** When the fiber finishes or the handler calls `Json` or `Send`, the response
   record goes back on `http:respond` with the request's id.

An event that arrives while a fiber is on the JS stack is queued, not dispatched, because
starting a fiber there would be a re-entrant call into the wasm exports, and `vmBridge`
refuses that by design. The queue drains the moment the stack unwinds, in arrival order.
`raise` answers with what the handler returned, or nothing at all when the VM has no function of
that id - a handler left over from a run that has been cancelled - and cancelling a session
settles every queued event the same way rather than leaving the host waiting.

## The whys

**Why the server is not a host object.** The first sketch had `FoxScript.Http` in the host.
It cannot be, and the reason is `CursorToJson`: a cursor is in the VM's data engine and the host
has no way to read one - the data requests go the other way, the host handing bytes to the VM.
`FoxScript.Json` is the same, because a JSON value is a `Value`. So the namespace is the VM's,
and the things it makes that touch the world - the sockets, a request, a response - are the
host's. The VM makes the values; the host makes the machine.

**Why the routes are in the VM and the sockets in Node.** A route is what the program said,
and the program lives in the VM. A socket is a file descriptor, and those live in the main
process. Putting the router in Node would mean a second copy of the route table kept in step
with the first, and a pattern language implemented twice. `matchit` is that implementation,
once.

**Why a request is an event and not a call.** The wasm exports are not re-entrant. If the host
called the handler from inside the request callback while a fiber was already stepping, it would
throw. Dispatching it as an event, exactly as a `Click` or a Timer is dispatched, keeps every
request inside the machinery that already handles nested dispatch correctly. There is only one
way into a lambda - `push_function_call` - and the call instruction, a built-in handing back
`CallFunction`, and a host event all go through it.

**Why one request is one fiber, and the queue is serial.** Because the scheduler's bargain is
that one fiber runs to completion or to a park before the next begins, and that bargain is what
makes `SetFocus` fire `GotFocus` in order and `READ EVENTS` wait without spinning. A server
that interleaved requests inside the VM would break every form in the application to gain
concurrency the data engine cannot use anyway. The cost - a blocking handler holds the queue -
is documented rather than hidden.

**Why 404 never wakes the runtime.** Matching is a read, and a read is cheap; starting a fiber
is not. A path nothing answers is answered by the host, so a scanner on the port costs the
program nothing.

**Why a thrown handler is a 500 and not silence.** A socket has a client on the other end who
cannot wait for a developer to read the Output window. The client gets an answer; the error
still goes where errors go.

**Why `Port` is `.F.` and not 0.** Zero is a port number nobody may use but a number all the
same, and a program that compared it would be comparing the wrong kind of thing. `.F.` is how
the product says "there is not one".

**Why a duplicate route is an error.** Two handlers for one question is a bug in the program,
and silently keeping the second would turn it into a bug in production. Error 3001 says which
route, and the number says it is ours.

**Why 127.0.0.1.** The server is a way for the application to be asked, not a way for the
world to be. Exposing it further is a deployment decision, made outside the program, with a
proxy in front.

**Why `SET LIBRARY TO` still works inside a handler.** The 32-bit library host answers
synchronously, because a program may call into a library half way through an expression. A
handler is a fiber like any other, so the call is the same call. A request handler that opens a
cursor, calls an `.fll` and answers is - apart from the lambda and the server - made of things
that already worked.
