/**
 * The sockets a `FoxScript.Http` server listens on.
 *
 * This is Node's own `http` and nothing else: a server is a port, a request is a payload, and
 * the answer comes back from whoever is driving the runtime. It knows nothing about routes,
 * lambdas or the VM - routing lives where the routes were registered, which is the VM - so the
 * whole of this file is "open a port, hand over what arrived, write back what came".
 *
 * It imports no Electron, so the tests drive it directly the way the real main process does.
 */

import type { IncomingMessage, Server, ServerResponse } from 'node:http';

import type { HttpRequestIn, HttpResponseOut } from '../ipc/api';

/** Answers a request, or `null` when nothing in the runtime would. */
export type HttpHandler = (request: HttpRequestIn) => Promise<HttpResponseOut | null>;

export interface HttpService {
  /** Opens a port. `port` 0 asks for any free one; answers the port actually bound. */
  listen(server: number, port: number): Promise<number>;
  /** Stops listening. True when there was something to stop. */
  close(server: number): Promise<boolean>;
  closeAll(): Promise<void>;
  /** Installs what answers requests. The last one installed is the one that answers. */
  onRequest(handler: HttpHandler): void;
}

/** The sentence a request nothing answered gets, so a client is never left hanging. */
const NOT_FOUND: HttpResponseOut = {
  status: 404,
  headers: { 'content-type': 'text/plain' },
  body: 'No route',
};

/** The sentence a request the runtime failed on gets. */
const FAILED: HttpResponseOut = {
  status: 500,
  headers: { 'content-type': 'text/plain' },
  body: 'Handler failed',
};

function nodeHttp(): typeof import('node:http') {
  // the same trick libraryHost.ts uses: this file is reachable from a browser-targeted bundle,
  // where a static import of a Node module would be resolved at build time
  const get = (process as NodeJS.Process & { getBuiltinModule?: (id: string) => unknown }).getBuiltinModule;
  if (!get) throw new Error('an HTTP server needs Node');
  return get('node:http') as typeof import('node:http');
}

function bodyOf(req: IncomingMessage): Promise<string> {
  return new Promise((done, fail) => {
    let text = '';
    req.setEncoding('utf8');
    req.on('data', (chunk: string) => {
      text += chunk;
    });
    req.on('end', () => done(text));
    req.on('error', fail);
  });
}

function headersOf(req: IncomingMessage): Record<string, string> {
  const out: Record<string, string> = {};
  for (const [name, value] of Object.entries(req.headers)) {
    if (value === undefined) continue;
    out[name.toLowerCase()] = Array.isArray(value) ? value.join(', ') : value;
  }
  return out;
}

export function createHttpService(): HttpService {
  const servers = new Map<number, Server>();
  let answer: HttpHandler = async () => null;

  const arrived = async (server: number, req: IncomingMessage, res: ServerResponse): Promise<void> => {
    const url = new URL(req.url ?? '/', 'http://localhost');
    const query: Record<string, string> = {};
    for (const [k, v] of url.searchParams) query[k] = v;
    let out: HttpResponseOut | null;
    try {
      out = await answer({
        server,
        method: (req.method ?? 'GET').toUpperCase(),
        path: decodeURIComponent(url.pathname),
        query,
        headers: headersOf(req),
        body: await bodyOf(req),
      });
    } catch {
      out = FAILED;
    }
    const written = out ?? NOT_FOUND;
    res.writeHead(written.status, written.headers);
    res.end(written.body);
  };

  return {
    listen(server, port) {
      return new Promise<number>((done, fail) => {
        const existing = servers.get(server);
        if (existing) {
          fail(new Error(`server ${server} is already listening`));
          return;
        }
        const made = nodeHttp().createServer((req, res) => {
          void arrived(server, req, res);
        });
        made.once('error', fail);
        made.listen(port, '127.0.0.1', () => {
          made.removeListener('error', fail);
          servers.set(server, made);
          const bound = made.address();
          done(typeof bound === 'object' && bound !== null ? bound.port : port);
        });
      });
    },

    close(server) {
      const open = servers.get(server);
      if (!open) return Promise.resolve(false);
      servers.delete(server);
      return new Promise<boolean>((done) => {
        open.closeAllConnections?.();
        open.close(() => done(true));
      });
    },

    async closeAll() {
      const open = [...servers.keys()];
      for (const server of open) await this.close(server);
    },

    onRequest(handler) {
      answer = handler;
    },
  };
}
