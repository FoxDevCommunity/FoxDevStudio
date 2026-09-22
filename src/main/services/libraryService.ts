/**
 * Visual FoxPro libraries, hosted in the one process that can load one.
 *
 * `SET LIBRARY TO something.fll` in a running program reaches `fllhost.exe` through here. A .fll
 * is a 32-bit image and every process this application runs is 64-bit, so the library is loaded
 * by a child process and this end only talks to it; `src/shared/runtime/libraryHost.ts` is the
 * conversation and `native/fllhost/fllhost.c` is the other side of it.
 *
 * The calls are synchronous by necessity, the way the COM ones are: a program may call a library
 * function from inside an expression the runtime is already evaluating, and an answer that
 * arrived later would not be an answer. `ipcMain.on` with `event.returnValue` is Electron's way
 * of saying that.
 *
 * The host is built only on Windows and is optional even there: without it every call reports
 * that the library is not found, which is what the runtime turns into error 1726.
 */

import { createLibraryHost, type LibraryHost, type LibraryValue } from '@shared/runtime/libraryHost';

/** What a call answers with: what it worked out, or why it could not be made. */
export type LibraryReply = { ok: true; value: unknown } | { ok: false; error: string };

export interface LibraryService {
  available(): boolean;
  perform(op: string, args: unknown[]): LibraryReply;
}

/**
 * @param dirs where to look for `fllhost.exe`: the app folder in development, the unpacked
 * resources folder in a packaged build.
 */
export function createLibraryService(dirs: string[]): LibraryService {
  let host: LibraryHost | null = null;
  const open = (): LibraryHost => (host ??= createLibraryHost(dirs));

  const perform = (op: string, args: unknown[]): LibraryReply => {
    try {
      switch (op) {
        case 'available':
          return { ok: true, value: open().available() };
        case 'load':
          return { ok: true, value: open().load(String(args[0])) };
        case 'call':
          return { ok: true, value: open().call(Number(args[0]), Number(args[1]), (args[2] ?? []) as LibraryValue[]) };
        case 'unload':
          open().unload(Number(args[0]));
          return { ok: true, value: null };
        case 'releaseAll':
          open().releaseAll();
          return { ok: true, value: null };
        default:
          return { ok: false, error: `the library host does not do ${op}` };
      }
    } catch (error) {
      return { ok: false, error: error instanceof Error ? error.message : String(error) };
    }
  };

  return { available: () => open().available(), perform };
}
