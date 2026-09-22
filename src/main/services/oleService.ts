/**
 * COM automation, on the one thread that may do it.
 *
 * `CREATEOBJECT("Word.Application")` in a running FoxPro program reaches the Rust addon through
 * here. COM objects belong to the apartment of the thread that made them, and the renderer is
 * not that thread, so everything is done in this process and the renderer holds only handles.
 *
 * The calls are synchronous by necessity, not by preference: the VM asks the host for a property
 * while it is still on the stack, so an answer that arrived later would not be an answer at all.
 * `ipcMain.on` with `event.returnValue` is Electron's way of saying that.
 *
 * The addon is built only on Windows and is optional even there: without it every call reports
 * that COM is unavailable, which is what the runtime turns into a VFP error the program can see.
 */

import { existsSync } from 'node:fs';
import { join } from 'node:path';

/** One value crossing to the addon; the shape `crates/foxole` declares. */
export interface OleValue {
  kind: 'null' | 'number' | 'string' | 'bool' | 'date' | 'object';
  num?: number;
  text?: string;
  flag?: boolean;
  handle?: number;
}

interface OleAddon {
  available(): boolean;
  create(name: string): number;
  active(name: string, className: string): number;
  get(handle: number, name: string, args: OleValue[]): OleValue;
  set(handle: number, name: string, value: OleValue): void;
  call(handle: number, name: string, args: OleValue[]): OleValue;
  release(handle: number): void;
  releaseAll(): void;
}

/** What a call answers with: a value, or why it could not be made. */
export type OleReply = { ok: true; value: OleValue } | { ok: false; error: string };

export interface OleService {
  available(): boolean;
  perform(op: string, args: unknown[]): OleReply;
}

/**
 * Loads the addon, once, from beside the application.
 *
 * `process.dlopen` rather than `require`: the main process is bundled, and a bundler rewrites
 * `require` of a path it cannot see at build time. This is also how Node loads any addon.
 */
function loadAddon(dirs: string[]): OleAddon | null {
  if (process.platform !== 'win32') return null;
  for (const dir of dirs) {
    const path = join(dir, 'foxole.node');
    if (!existsSync(path)) continue;
    try {
      const holder = { exports: {} as OleAddon };
      process.dlopen(holder, path);
      return holder.exports;
    } catch {
      // a wrong-architecture or unloadable build is the same as not having one
      return null;
    }
  }
  return null;
}

/**
 * @param dirs where to look for `foxole.node`: the app folder in development, the unpacked
 * resources folder in a packaged build.
 */
export function createOleService(dirs: string[]): OleService {
  let addon: OleAddon | null | undefined;
  const load = (): OleAddon | null => (addon === undefined ? (addon = loadAddon(dirs)) : addon);

  const perform = (op: string, args: unknown[]): OleReply => {
    const ole = load();
    if (!ole) {
      // asking whether COM is there is always answered; anything else is a failure
      if (op === 'available') return { ok: true, value: { kind: 'bool', flag: false } };
      return { ok: false, error: 'COM automation is not available in this build' };
    }
    try {
      switch (op) {
        case 'available':
          return { ok: true, value: { kind: 'bool', flag: true } };
        case 'create':
          return { ok: true, value: { kind: 'object', handle: ole.create(String(args[0])) } };
        case 'active':
          return {
            ok: true,
            value: { kind: 'object', handle: ole.active(String(args[0]), String(args[1] ?? '')) },
          };
        case 'get':
          return { ok: true, value: ole.get(Number(args[0]), String(args[1]), (args[2] ?? []) as OleValue[]) };
        case 'set':
          ole.set(Number(args[0]), String(args[1]), args[2] as OleValue);
          return { ok: true, value: { kind: 'null' } };
        case 'call':
          return { ok: true, value: ole.call(Number(args[0]), String(args[1]), (args[2] ?? []) as OleValue[]) };
        case 'release':
          ole.release(Number(args[0]));
          return { ok: true, value: { kind: 'null' } };
        case 'releaseAll':
          ole.releaseAll();
          return { ok: true, value: { kind: 'null' } };
        default:
          return { ok: false, error: `unknown COM operation ${op}` };
      }
    } catch (error) {
      return { ok: false, error: error instanceof Error ? error.message : String(error) };
    }
  };

  return { available: () => load() !== null, perform };
}
