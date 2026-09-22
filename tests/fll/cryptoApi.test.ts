/**
 * A 32-bit Visual FoxPro library, answering an HTTP request from inside a lambda.
 *
 * This is the one test that exercises the whole architecture at once, and each piece of it is
 * a different wall that had to be got round:
 *
 *   - `vfpencryption71.fll` is a 32-bit image and every process this runtime runs in is 64-bit,
 *     so it is hosted by `fllhost.exe` out of process and reached over a pipe;
 *   - the handler is a lambda, a value the program made at run time and handed to the server;
 *   - the request arrives from a real socket in the main process as a HostEvent, and is
 *     dispatched as its own fiber while the program is parked in READ EVENTS;
 *   - the VM is never re-entered on any of those paths.
 *
 * The expected digest is not from the library's documentation. `2AAE6C35...846ED` is the SHA-1
 * of "hello world" as Visual FoxPro answered it with this library loaded, and it is the same
 * value `vfpencryption.prg` beside this file checks. If the plumbing above is wrong anywhere,
 * the bytes come back wrong here.
 *
 * Skips itself when the library is not on the machine; point FOXDEV_FLL at it.
 */

import { existsSync, readFileSync } from 'node:fs';
import { afterEach, beforeAll, beforeEach, describe, expect, it } from 'vitest';
import { setApi } from '@renderer/api/foxdev';
import { createMemoryApi } from '@renderer/api/memoryApi';
import { useSessionStore } from '@renderer/runtime/session';
import { createProjectSource } from '@renderer/runtime/projectSource';
import { createHttpService } from '@shared/runtime/httpService';
import type { HttpResponseOut } from '@shared/ipc/api';
import { loadFoxVm } from '../../src/wasm/foxvm/loader';

// the library is not ours to ship: drop a copy at the repository root, or name one in FOXDEV_FLL
const LIBRARY = process.env['FOXDEV_FLL'] ?? 'vfpencryption71.fll';
const have = existsSync(LIBRARY);
const source = createProjectSource();

/** The main process's half: the service, and the two channels carrying a request and its answer. */
function wire() {
  const service = createHttpService();
  const waiting = new Map<number, (response: HttpResponseOut | null) => void>();
  let next = 1;
  let listener:
    ((id: number, request: Parameters<Parameters<typeof service.onRequest>[0]>[0]) => void) | null = null;
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
  api.binary$.set(LIBRARY, new Uint8Array(readFileSync(LIBRARY)));
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

function get(port: number, path: string): Promise<{ status: number; body: string }> {
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
      res.on('end', () => done({ status: res.statusCode ?? 0, body }));
    });
    req.on('error', fail);
    req.end();
  });
}

async function portOf(): Promise<number> {
  for (let tries = 0; tries < 400; tries++) {
    const port = useSessionStore.getState().vm?.getGlobal('gnPort');
    if (typeof port === 'number' && port > 0) return port;
    await new Promise((r) => setTimeout(r, 10));
  }
  throw new Error('the server never started listening');
}

/**
 * The handler hashes with the library and hands back hex.
 *
 * `Hash` is not a function this runtime has: it arrives in the language when SET LIBRARY TO
 * loads the .fll, and is called here the way any built-in is.
 */
const PROGRAM = [
  `SET LIBRARY TO "${LIBRARY}" ADDITIVE`,
  '',
  'PUBLIC gnPort, goServer',
  'goServer = FoxScript.Http.CreateServer()',
  '',
  'goServer.Get("/api/v1/hash/:algorithm", LAMBDA(req, res)',
  '    LOCAL lnAlgorithm, lcText, lcBytes, lcHex, i',
  '    lnAlgorithm = VAL(req.Params("algorithm"))',
  '    lcText = NVL(req.Query("text"), "")',
  '',
  '    IF EMPTY(lcText)',
  '        res.Status(400).Json(\'{"error": "text is required"}\')',
  '        RETURN',
  '    ENDIF',
  '',
  '    lcBytes = Hash(lcText, lnAlgorithm)',
  '    lcHex = ""',
  '    FOR i = 1 TO LEN(lcBytes)',
  '        lcHex = lcHex + RIGHT("0" + TRANSFORM(ASC(SUBSTR(lcBytes, i, 1)), "@0"), 2)',
  '    ENDFOR',
  '',
  '    res.Status(200).Json(\'{"bytes": \' + TRANSFORM(LEN(lcBytes)) + \', "hex": "\' + lcHex + \'"}\')',
  'ENDLAMBDA)',
  '',
  'gnPort = goServer.Listen(0)',
  'READ EVENTS',
].join('\n');

let service: ReturnType<typeof createHttpService> | null = null;

beforeAll(async () => {
  if (have) await loadFoxVm();
}, 120_000);

beforeEach(() => {
  if (!have) return;
  service = wire();
  useSessionStore.getState().cancel();
  useSessionStore.setState({ output: [] });
});

afterEach(async () => {
  useSessionStore.getState().cancel();
  await service?.closeAll();
  service = null;
});

describe.skipIf(!have)('a 32-bit library answering over HTTP', () => {
  it('hashes what the product hashes, through a lambda, over a socket', async () => {
    void useSessionStore.getState().execute(source, PROGRAM);
    const port = await portOf();

    // SHA-1, the value the product answered with this library loaded
    const sha1 = await get(port, '/api/v1/hash/1?text=hello%20world');
    expect(sha1.status).toBe(200);
    expect(JSON.parse(sha1.body)).toEqual({
      bytes: 20,
      hex: '2AAE6C35C94FCFB415DBE95F408B9CE91EE846ED',
    });

    // SHA-256, likewise
    const sha256 = await get(port, '/api/v1/hash/2?text=hello%20world');
    expect(JSON.parse(sha256.body)).toEqual({
      bytes: 32,
      hex: 'B94D27B9934D3E08A52E52D7DA7DABFAC484EFE37A5380EE9088F7ACE2EFCDE9',
    });

    // MD5, to show the algorithm argument reaches the library rather than being ignored
    const md5 = await get(port, '/api/v1/hash/5?text=hello%20world');
    expect(JSON.parse(md5.body)).toEqual({ bytes: 16, hex: '5EB63BBBE01EEED093CB22BB8F5ACDC3' });

    // the handler's own guard, before the library is reached at all
    const empty = await get(port, '/api/v1/hash/2?text=');
    expect(empty.status).toBe(400);

    // and the program is still parked in READ EVENTS, which is what keeps the server alive
    expect(useSessionStore.getState().scheduler?.readEventsDepth).toBe(1);
  }, 60_000);
});
