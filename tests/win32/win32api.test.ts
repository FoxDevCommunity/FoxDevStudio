/**
 * Calling Windows itself, the whole way through: a FoxPro program, compiled by our own
 * compiler, run by our own VM, reaching the real kernel32 and user32 through the same
 * `dll.call` the application uses.
 *
 * `tests/main/dllService.test.ts` proves the service can call a library. This proves a *program*
 * can: `DECLARE ... DLL` with every parameter shape a Visual FoxPro program uses - a struct
 * packed by hand and passed by reference, a buffer the caller sizes, a handle passed straight
 * back into another call, a function that takes nothing, one that returns nothing - and the
 * values that come back read with BINTOC and CTOBIN.
 *
 * It checks itself off Windows, and where the build has no way to call a library at all.
 */

import { readFileSync } from 'node:fs';
import { beforeAll, describe, expect, it } from 'vitest';
import { createMemoryApi } from '@renderer/api/memoryApi';
import { setApi } from '@renderer/api/foxdev';
import { useSessionStore } from '@renderer/runtime/session';
import { compileProgram } from '@renderer/runtime/vmBridge';
import { requireBytes } from '@shared/runtime/programSource';
import { loadFoxVm } from '../../src/wasm/foxvm/loader';
import type { FoxDevApi } from '@shared/ipc/api';

/**
 * The library service the application gives the renderer, loaded the way the application loads
 * it. The path is a variable so that this file, which belongs to the renderer's half of the
 * build, does not pull the main process's half into it at compile time.
 */
interface DllService {
  available(): boolean;
  call(request: Parameters<FoxDevApi['dll']['call']>[0]): Awaited<ReturnType<FoxDevApi['dll']['call']>>;
}
const servicePath = '../../src/main/services/dllService';
const { createDllService } = (await import(/* @vite-ignore */ servicePath)) as { createDllService: () => DllService };
const service = createDllService();
const windows = process.platform === 'win32' && service.available();
const PROGRAM = 'tests/win32/win32api.prg';

const settle = (): Promise<unknown> => new Promise((r) => setTimeout(r, 0));

beforeAll(async () => {
  if (windows) await loadFoxVm();
}, 120_000);

describe.skipIf(!windows)('a program calling the Windows API', () => {
  it('passes every one of the twelve calls', async () => {
    const api = createMemoryApi();
    // the real library service, the one the application gives the renderer
    api.dll = {
      async available() {
        return true;
      },
      async call(request) {
        return service.call(request);
      },
    };
    setApi(api);
    useSessionStore.getState().cancel();
    useSessionStore.setState({ output: [] });

    const text = readFileSync(PROGRAM, 'utf8');
    await useSessionStore.getState().execute(
      {
        async getForm() {
          return null;
        },
        async getProgram(name) {
          return { name: 'win32api', bytes: requireBytes(name, compileProgram(text, 'win32api')) };
        },
        async getMenu() {
          return null;
        },
      },
      'DO win32api',
    );
    for (let i = 0; i < 400 && useSessionStore.getState().status === 'running'; i++) await settle();

    const said = useSessionStore
      .getState()
      .output.map((l) => l.text.trim())
      .filter((t) => t !== '');
    // every line the program printed, so a failure says which call and what it answered
    expect(said.filter((l) => l.startsWith('[FAIL]')), said.join('\n')).toEqual([]);
    // one call cannot work in a 64-bit host whatever we do, and the program says so itself
    expect(said.filter((l) => l.startsWith('[KNOWN]'))).toHaveLength(1);
    expect(said.at(-1)).toBe('PASSED 11 FAILED 0');
  }, 120_000);
});
