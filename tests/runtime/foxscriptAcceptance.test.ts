/**
 * The program this milestone was aiming at, run end to end.
 *
 * It is the program from docs/foxscript.md with one line changed: `SET LIBRARY TO` is left out,
 * because hosting a real 32-bit .fll needs a Windows machine with that library on it and this
 * test has to run anywhere. Everything else is exactly as the document wrote it - the lambda,
 * the route with a named part, the SQL into a cursor, CursorToJson, the 404, and READ EVENTS
 * keeping the server alive - and the requests are real requests over a real socket.
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

/** The table the handler queries, standing in for the sample database. */
const CUSTOMERS = [
  'CREATE CURSOR customer (cust_id N(6), company C(20), city C(20))',
  'INSERT INTO customer VALUES (1, "Ada Ltd", "London")',
  'INSERT INTO customer VALUES (7, "Bee Ltd", "Lisbon")',
  'INSERT INTO customer VALUES (9, "Cee Ltd", "Berlin")',
];

const PROGRAM = [
  ...CUSTOMERS,
  '',
  'PUBLIC gnPort, goServer',
  'goServer = FoxScript.Http.CreateServer()',
  '',
  'goServer.Get("/api/v1/customers/:id", LAMBDA(req, res)',
  '    LOCAL lnId',
  '    lnId = VAL(req.Params("id"))',
  '',
  '    SELECT * FROM customer WHERE cust_id = lnId INTO CURSOR c_cust',
  '',
  '    IF RECCOUNT("c_cust") > 0',
  '        res.Status(200).Json(FoxScript.Data.CursorToJson("c_cust"))',
  '    ELSE',
  '        res.Status(404).Json(\'{"error": "Not found"}\')',
  '    ENDIF',
  '',
  '    USE IN c_cust',
  'ENDLAMBDA)',
  '',
  'gnPort = goServer.Listen(0)',
  'READ EVENTS',
].join('\n');

describe('the FoxScript milestone, end to end', () => {
  it('serves a customer as JSON and answers 404 for one that is not there', async () => {
    void useSessionStore.getState().execute(source, PROGRAM);
    const port = await portOf();

    const found = await fetchText(port, '/api/v1/customers/7');
    expect(found.status).toBe(200);
    expect(found.type).toBe('application/json');
    expect(JSON.parse(found.body)).toEqual([{ cust_id: 7, company: 'Bee Ltd', city: 'Lisbon' }]);

    const missing = await fetchText(port, '/api/v1/customers/42');
    expect(missing.status).toBe(404);
    expect(missing.type).toBe('application/json');
    expect(JSON.parse(missing.body)).toEqual({ error: 'Not found' });

    // a second request to the same route runs its own fiber, and the cursor the first one
    // opened and closed is no obstacle to it
    const again = await fetchText(port, '/api/v1/customers/1');
    expect(JSON.parse(again.body)).toEqual([{ cust_id: 1, company: 'Ada Ltd', city: 'London' }]);

    // the program is still parked in READ EVENTS, which is what keeps the server alive
    expect(useSessionStore.getState().scheduler?.readEventsDepth).toBe(1);
  });
});
