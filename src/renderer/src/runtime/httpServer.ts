/**
 * What happens between a request arriving and a lambda answering it.
 *
 * The sockets are in the main process; the routes are in the VM, where the program registered
 * them. This is the seam: ask the VM which lambda a request matches, wrap the request and the
 * reply as host objects, and dispatch the lambda the way a Click is dispatched - a fiber of its
 * own, never called from inside a host request.
 *
 * One request is one fiber, and the scheduler serialises fibers. A handler therefore runs to
 * completion or parks; two requests never interleave inside the VM, and **a handler that blocks
 * holds the queue** - a long SELECT, a MESSAGEBOX, a WAIT WINDOW with no timeout stops every
 * other request until it is done. See docs/foxscript.md.
 */

import type { HttpRequestIn, HttpResponseOut } from '@shared/ipc/api';
import type { MemberEntry } from '@shared/runtime/objectModel';
import type { HostObject } from '@shared/runtime/oleObjects';
import type { Scheduler } from '@shared/runtime/scheduler';
import type { VmValue } from '@shared/runtime/values';

/** What the bridge needs of the VM: the route table, which is where the routes were put. */
export interface RouteReader {
  route(server: number, method: string, path: string): { function: number; params: Record<string, string> } | undefined;
}

/** What the bridge needs of the object model: a handle for a host object. */
export interface HandleSource {
  hostHandle(object: HostObject): number;
}

/** The sentence a response that is already written gets when a program writes to it again. */
const FINISHED = 'the response has already been sent';

const asText = (v: VmValue | undefined): string => (v === null || v === undefined ? '' : String(v));

/**
 * `req`: what arrived, read-only.
 *
 * The named parts of the path, the query string and the headers are all lookups by name rather
 * than properties, because a program does not know their names when it writes the handler and
 * an object cannot have a property per request. `req.Params("id")` is the shape the whole of
 * this milestone's target program uses.
 */
export class HttpRequestObject implements HostObject {
  readonly className = 'FoxScriptHttpRequest';

  constructor(
    private readonly request: HttpRequestIn,
    private readonly params: Record<string, string>,
  ) {}

  private static readonly PROPERTIES = ['METHOD', 'PATH', 'BODY'];
  private static readonly METHODS = ['PARAMS', 'QUERY', 'HEADER'];

  member(name: string): 'prop' | 'method' | 'none' {
    const upper = name.toUpperCase();
    if (HttpRequestObject.PROPERTIES.includes(upper)) return 'prop';
    if (HttpRequestObject.METHODS.includes(upper)) return 'method';
    return 'none';
  }

  list(): MemberEntry[] {
    return [
      ...HttpRequestObject.PROPERTIES.map((name) => member(name, 'Property', this.get(name))),
      ...HttpRequestObject.METHODS.map((name) => member(name, 'Method')),
    ];
  }

  get(name: string): VmValue | undefined {
    switch (name.toUpperCase()) {
      case 'METHOD':
        return this.request.method;
      case 'PATH':
        return this.request.path;
      case 'BODY':
        return this.request.body;
      default:
        return undefined;
    }
  }

  set(): void {
    throw new Error('a request cannot be changed');
  }

  call(name: string, args: VmValue[]): VmValue | undefined {
    const key = asText(args[0]);
    switch (name.toUpperCase()) {
      case 'PARAMS':
        return this.params[key] ?? '';
      case 'QUERY':
        return this.request.query[key] ?? '';
      // a header name means the same in any case, which is what HTTP says
      case 'HEADER':
        return this.request.headers[key.toLowerCase()] ?? '';
      default:
        return undefined;
    }
  }
}

/**
 * `res`: what to write back.
 *
 * A host object with a handle, like a form. Every method answers with the response itself so
 * that `res.Status(200).Json(...)` reads the way it does in every other language that has this
 * shape; the ones that write a body also finish the response, and writing to a finished
 * response is an error the host raises rather than a crash.
 */
export class HttpResponseObject implements HostObject {
  readonly className = 'FoxScriptHttpResponse';
  private status = 200;
  private readonly headers: Record<string, string> = {};
  private body = '';
  private sent = false;

  private static readonly PROPERTIES = ['SENT'];
  private static readonly METHODS = ['STATUS', 'HEADER', 'JSON', 'SEND'];

  member(name: string): 'prop' | 'method' | 'none' {
    const upper = name.toUpperCase();
    if (HttpResponseObject.PROPERTIES.includes(upper)) return 'prop';
    if (HttpResponseObject.METHODS.includes(upper)) return 'method';
    return 'none';
  }

  list(): MemberEntry[] {
    return [
      ...HttpResponseObject.PROPERTIES.map((name) => member(name, 'Property', this.get(name))),
      ...HttpResponseObject.METHODS.map((name) => member(name, 'Method')),
    ];
  }

  get(name: string): VmValue | undefined {
    return name.toUpperCase() === 'SENT' ? this.sent : undefined;
  }

  set(): void {
    throw new Error('a response is written through its methods');
  }

  call(name: string, args: VmValue[]): HostObject | undefined {
    const upper = name.toUpperCase();
    if (!HttpResponseObject.METHODS.includes(upper)) return undefined;
    if (this.sent) throw new Error(FINISHED);
    switch (upper) {
      case 'STATUS':
        this.status = Number(args[0] ?? 200);
        return this;
      case 'HEADER':
        this.headers[asText(args[0]).toLowerCase()] = asText(args[1]);
        return this;
      case 'JSON':
        this.headers['content-type'] ??= 'application/json';
        this.body = asText(args[0]);
        this.sent = true;
        return this;
      default:
        this.headers['content-type'] ??= 'text/plain';
        this.body = asText(args[0]);
        this.sent = true;
        return this;
    }
  }

  /** What the main process is to write. */
  written(): HttpResponseOut {
    return { status: this.status, headers: { ...this.headers }, body: this.body };
  }

  /** True once a body has been written; a handler that wrote none answers with what it has. */
  get finished(): boolean {
    return this.sent;
  }
}

function member(name: string, kind: 'Property' | 'Method', value?: VmValue): MemberEntry {
  return { name, kind, native: true, added: false, readOnly: kind === 'Property', changed: false, value: value ?? null };
}

/**
 * Answers one request: match a route, dispatch the lambda, and hand back what `res` collected.
 *
 * `null` when no route answers it, which the main process turns into a 404 rather than leaving
 * a client hanging. An error out of the handler is the runtime's to report; the client is told
 * 500 by the service, because a socket cannot wait for a developer.
 */
export async function answerRequest(
  request: HttpRequestIn,
  vm: RouteReader,
  scheduler: Scheduler,
  handles: HandleSource,
): Promise<HttpResponseOut | null> {
  const matched = vm.route(request.server, request.method, request.path);
  if (!matched) return null;

  const req = new HttpRequestObject(request, matched.params);
  const res = new HttpResponseObject();
  const outcome = await scheduler.raise({
    func: matched.function,
    args: [{ $obj: handles.hostHandle(req) }, { $obj: handles.hostHandle(res) }],
  });
  // the VM no longer has that handler: the run was cancelled between the match and the dispatch
  if (outcome === null) return null;
  return res.written();
}
