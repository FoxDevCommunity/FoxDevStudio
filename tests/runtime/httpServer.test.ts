/**
 * `FoxScript.Http`, end to end: a real socket, a real request, a lambda that answers it.
 *
 * The main process's own service stands in for the main process, because it is the same file
 * and imports no Electron. Everything else is the real thing: the real VM, the real scheduler,
 * the real session, the real route table.
 *
 * The specification is docs/foxscript.md, "FoxScript.Http".
 */

import { afterEach, beforeAll, beforeEach, describe, expect, it } from 'vitest';
import { setApi } from '@renderer/api/foxdev';
import { createMemoryApi } from '@renderer/api/memoryApi';
import { useSessionStore } from '@renderer/runtime/session';
import { createProjectSource } from '@renderer/runtime/projectSource';
import { createHttpService } from '@shared/runtime/httpService';
import type { HttpResponseOut } from '@shared/ipc/api';
import { loadFoxVm } from '../../src/wasm/foxvm/loader';

const source = createProjectSource();

/** The main process's half: the service, and the two channels that carry a request and its answer. */
function wireHttp() {
  const service = createHttpService();
  const waiting = new Map<number, (response: HttpResponseOut | null) => void>();
  let next = 1;
  let listener: ((id: number, request: Parameters<Parameters<typeof service.onRequest>[0]>[0]) => void) | null = null;
  service.onRequest(
    (request) =>
      new Promise<HttpResponseOut | null>((answered) => {
        const id = next++;
        waiting.set(id, answered);
        if (listener) listener(id, request);
        else answered(null);
      }),
  );
  const api = createMemoryApi();
  api.http = {
    listen: (server, port) => service.listen(server, port),
    close: (server) => service.close(server),
    onRequest: (cb) => {
      listener = cb;
      return () => {
        listener = null;
      };
    },
    respond: async (id, response) => {
      waiting.get(id)?.(response);
      waiting.delete(id);
    },
  };
  setApi(api);
  return service;
}

/** A real request over a real socket, answered with the status, the headers and the body. */
function fetchText(port: number, path: string): Promise<{ status: number; type: string; body: string }> {
  const http = (process as NodeJS.Process & { getBuiltinModule(id: string): unknown }).getBuiltinModule(
    'node:http',
  ) as typeof import('node:http');
  return new Promise((done, fail) => {
    const req = http.request({ host: '127.0.0.1', port, path, method: 'GET' }, (res) => {
      let body = '';
      res.setEncoding('utf8');
      res.on('data', (chunk: string) => {
        body += chunk;
      });
      res.on('end', () =>
        done({ status: res.statusCode ?? 0, type: String(res.headers['content-type'] ?? ''), body }),
      );
    });
    req.on('error', fail);
    req.end();
  });
}

/** The port the program printed into a public variable, once it has one. */
async function portOf(): Promise<number> {
  for (let tries = 0; tries < 200; tries++) {
    const port = useSessionStore.getState().vm?.getGlobal('gnPort');
    if (typeof port === 'number' && port > 0) return port;
    await new Promise((r) => setTimeout(r, 10));
  }
  throw new Error('the server never started listening');
}

let service: ReturnType<typeof createHttpService> | null = null;

beforeAll(async () => {
  await loadFoxVm();
});

beforeEach(() => {
  service = wireHttp();
  useSessionStore.getState().cancel();
  useSessionStore.setState({ output: [] });
});

afterEach(async () => {
  useSessionStore.getState().cancel();
  await service?.closeAll();
  service = null;
});

const PROGRAM = [
  'PUBLIC gnPort, goServer, gcSeen',
  'gcSeen = ""',
  'goServer = FoxScript.Http.CreateServer()',
  'goServer.Get("/api/v1/customers/:id", LAMBDA(req, res)',
  '  gcSeen = gcSeen + "[" + req.Method + " " + req.Path + "]"',
  '  IF VAL(req.Params("id")) = 7',
  '    res.Status(200).Send(\'{"id": 7, "name": "Ada"}\')',
  '  ELSE',
  '    res.Status(404).Send(\'{"error": "Not found"}\')',
  '  ENDIF',
  'ENDLAMBDA)',
  'gnPort = goServer.Listen(0)',
  'READ EVENTS',
].join('\n');

describe('FoxScript.Http', () => {
  it('answers a real request with what the lambda wrote, and 404s the one it has no answer for', async () => {
    void useSessionStore.getState().execute(source, PROGRAM);
    const port = await portOf();

    const found = await fetchText(port, '/api/v1/customers/7');
    expect(found.status).toBe(200);
    expect(found.body).toBe('{"id": 7, "name": "Ada"}');

    const missing = await fetchText(port, '/api/v1/customers/9');
    expect(missing.status).toBe(404);
    expect(missing.body).toBe('{"error": "Not found"}');

    // the handler saw the request, and the route matched the path with the id taken out of it
    expect(useSessionStore.getState().vm?.getGlobal('gcSeen')).toBe(
      '[GET /api/v1/customers/7][GET /api/v1/customers/9]',
    );
  });

  it('answers a path no route matches without asking the runtime anything', async () => {
    void useSessionStore.getState().execute(source, PROGRAM);
    const port = await portOf();

    const nothing = await fetchText(port, '/nothing/here');
    expect(nothing.status).toBe(404);
    // the runtime was never woken, so the handler's record of what it saw is still empty
    expect(useSessionStore.getState().vm?.getGlobal('gcSeen')).toBe('');
  });

  it('closes the port when the run is cancelled', async () => {
    void useSessionStore.getState().execute(source, PROGRAM);
    const port = await portOf();
    expect((await fetchText(port, '/api/v1/customers/7')).status).toBe(200);

    useSessionStore.getState().cancel();
    // a port that outlived its program would be a leak nobody could close
    await new Promise((r) => setTimeout(r, 50));
    await expect(fetchText(port, '/api/v1/customers/7')).rejects.toThrow();
  });

  it('refuses to write to a response that has already been sent', async () => {
    const twice = [
      'PUBLIC gnPort, goServer, gcError',
      'gcError = ""',
      'goServer = FoxScript.Http.CreateServer()',
      'goServer.Get("/twice", LAMBDA(req, res)',
      '  res.Send("first")',
      '  TRY',
      '    res.Send("second")',
      '  CATCH TO loErr',
      '    gcError = loErr.Message',
      '  ENDTRY',
      'ENDLAMBDA)',
      'gnPort = goServer.Listen(0)',
      'READ EVENTS',
    ].join('\n');
    void useSessionStore.getState().execute(source, twice);
    const port = await portOf();

    const answered = await fetchText(port, '/twice');
    expect(answered.body).toBe('first');
    // an error the program can catch, not a crash and not a second answer on the wire
    expect(useSessionStore.getState().vm?.getGlobal('gcError')).toContain('already been sent');
  });
});
