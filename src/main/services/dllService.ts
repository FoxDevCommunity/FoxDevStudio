/**
 * `DECLARE ... DLL`: calling a function of a native library.
 *
 * Visual FoxPro programs reach the Windows API this way - `GetPrivateProfileString`,
 * `ShellExecute`, `GetWindowsDirectory` - and a program that cannot make the call cannot run.
 * The call is made with koffi, which loads a library and builds a callable from a C prototype
 * at run time, so nothing has to be compiled to add one.
 *
 * It happens in the main process because that is where Node lives, and because a library loaded
 * into the renderer would be loaded into a sandbox that cannot have it. The VM yields the call
 * as a request and waits for the answer, so a slow library blocks the program that called it and
 * nothing else.
 */

import { createRequire } from 'node:module';

/** The types Visual FoxPro's DECLARE understands, and the C type each one is. */
const TYPES: Record<string, string> = {
  SHORT: 'int16',
  INTEGER: 'int32',
  LONG: 'int32',
  SINGLE: 'float',
  DOUBLE: 'double',
  STRING: 'str',
  OBJECT: 'void *',
};

/** What a call needs to know, as the VM sends it. */
export interface DllCall {
  library: string;
  function: string;
  /** VFP type word of the return value; empty when the declaration omitted it. */
  returns: string;
  /** VFP type word per parameter. */
  params: string[];
  /** Which parameters the library is given the address of. */
  byRef: boolean[];
  args: (string | number | boolean | null)[];
}

export interface DllResult {
  value: string | number | boolean | null;
  /** Arguments the library wrote back, at the positions declared by reference. */
  written: (string | number | null)[];
}

export interface DllService {
  available(): boolean;
  call(request: DllCall): DllResult;
}

interface Koffi {
  load(path: string): { func(prototype: string): (...args: unknown[]) => unknown };
  pointer(type: string): unknown;
  out(type: unknown): unknown;
  alloc(type: string, count: number): unknown;
  decode(value: unknown, type: string): unknown;
}

/**
 * `IN WIN32API` is Visual FoxPro's way of saying "wherever in Windows this lives"; the libraries
 * below are the ones it looks in.
 *
 * Measured against vfp9.exe rather than guessed at. In the set: `GetUserNameA` and
 * `CryptAcquireContextA` (advapi32) both answer, and so does `WNetGetConnectionA` (mpr) - which
 * is what the Foundation Classes' crypto class needs, since it declares
 * `CryptAcquireContextA IN WIN32API`. Not in it: `ShellExecuteA` (shell32), `GetOpenFileNameA`
 * (comdlg32), `CoInitialize` (ole32), `VarBstrCmp` (oleaut32), `PathFileExistsA` (shlwapi),
 * `GetFileVersionInfoSizeA` (version), `InternetOpenA` (wininet) and `waveOutGetNumDevs`
 * (winmm) all raise 1754. `timeGetTime` does answer, but through kernel32, which forwards it:
 * declared `IN kernel32` it works and `IN winmm` it works, which is how winmm was ruled out.
 */
const WIN32API = ['kernel32.dll', 'user32.dll', 'gdi32.dll', 'advapi32.dll', 'mpr.dll'];

/**
 * The two ways a declared function fails to be one, as Visual FoxPro reports them.
 *
 * Measured: a library Windows will not load is 1753, "Cannot load 32-bit DLL nosuchlibrary.dll."
 * (the product says 32-bit whatever the process is), and a function none of the libraries
 * exported is 1754, "Cannot find entry point NoSuchEntry in the DLL." The function is named as
 * the program declared it rather than as the ANSI retry spelled it.
 *
 * The number travels at the front of the message - `1754|Cannot find ...` - because an error
 * crossing from the main process arrives as text and nothing else, which is how the COM bridge
 * carries its HRESULT across the same gap.
 */
function libraryMissing(library: string): Error {
  return new Error(`1753|Cannot load 32-bit DLL ${library}.`);
}

function entryPointMissing(entryPoint: string): Error {
  return new Error(`1754|Cannot find entry point ${entryPoint} in the DLL.`);
}

function libraryNames(name: string): string[] {
  const trimmed = name.trim().replace(/^["']|["']$/g, '');
  if (trimmed.toUpperCase() === 'WIN32API') return WIN32API;
  // a bare name is a library name; VFP adds the extension when there is none
  return [/\.[A-Za-z]{2,4}$/.test(trimmed) ? trimmed : `${trimmed}.dll`];
}

export function createDllService(): DllService {
  let koffi: Koffi | null | undefined;
  const load = (): Koffi | null => {
    if (koffi === undefined) {
      try {
        koffi = createRequire(__filename)('koffi') as Koffi;
      } catch {
        koffi = null;
      }
    }
    return koffi;
  };

  /** One callable per library, function and signature. Building one is not free. */
  const built = new Map<string, (...args: unknown[]) => unknown>();
  const libraries = new Map<string, { func(prototype: string): (...args: unknown[]) => unknown }>();

  const openLibrary = (name: string) => {
    const held = libraries.get(name);
    if (held) return held;
    const lib = load()!.load(name);
    libraries.set(name, lib);
    return lib;
  };

  return {
    available: () => load() !== null,

    call(request) {
      const api = load();
      if (!api) throw new Error('DECLARE ... DLL needs the koffi library, which is not available in this build');

      const returns = request.returns.trim() === '' ? 'void' : (TYPES[request.returns.toUpperCase()] ?? 'int32');
      const params = request.params.map((p, i) => {
        const type = TYPES[p.toUpperCase()] ?? 'int32';
        if (!request.byRef[i]) return type;
        // A string written into by the library is a buffer it is given the address of; a number
        // is one cell of that type, in and out: GetComputerName is told how much room it has in
        // the same parameter it answers the length in.
        return p.toUpperCase() === 'STRING' ? 'char *' : `_Inout_ ${type} *`;
      });
      const signature = (fn: string) => `${returns} ${fn}(${params.join(', ')})`;
      const key = `${request.library}|${signature(request.function)}`;

      let call = built.get(key);
      if (!call) {
        // Windows exports GetPrivateProfileStringA, and a VFP program declares it without the
        // A: the ANSI name is tried when the name as written is not there, as VFP does
        const attempts = [request.function, `${request.function}A`];
        const names = libraryNames(request.library);
        let loaded = false;
        for (const name of names) {
          let lib;
          try {
            lib = openLibrary(name);
          } catch {
            // a library that will not load is not a library that lacks the function
            continue;
          }
          loaded = true;
          for (const fn of attempts) {
            try {
              call = lib.func(signature(fn));
              break;
            } catch {
              // the next name, or the next library
            }
          }
          if (call) break;
        }
        // Which of the two failures it was decides what the program is told: a library Windows
        // would not load is 1753 and a function none of them exported is 1754. The function is
        // named as the program declared it, not as the ANSI retry spelled it.
        if (!loaded) throw libraryMissing(names[0] ?? request.library.trim());
        if (!call) throw entryPointMissing(request.function);
        built.set(key, call);
      }

      // by-reference arguments are passed as a cell the library writes into: a Buffer for a
      // string, a one-element array for a number, which is koffi's own convention
      const buffers: (Buffer | unknown[] | null)[] = [];
      const args = request.args.map((arg, i) => {
        if (!request.byRef[i]) return arg;
        if (request.params[i]?.toUpperCase() === 'STRING') {
          // the string the program passed is the buffer, and its length is the room the
          // library has: VFP programs size it with SPACE(n) for exactly that reason
          const text = String(arg ?? '');
          const buffer = Buffer.alloc(Math.max(text.length + 1, 1));
          buffer.write(text, 'latin1');
          buffers[i] = buffer;
          return buffer;
        }
        const cell = [Number(arg ?? 0)];
        buffers[i] = cell;
        return cell;
      });

      const value = call(...args) as string | number | boolean | null;
      const written = buffers.map((cell) => {
        if (cell === null || cell === undefined) return null;
        if (Buffer.isBuffer(cell)) {
          // The whole buffer comes back, NULs and all. A library fills a structure as often as
          // it fills a string - a POINT is two numbers, a SYSTEMTIME eight, a counter eight bytes
          // of which seven are usually zero - and cutting it at the first zero left the program
          // with the first field and nothing after it. A program that wanted a C string trims it
          // itself, which is what `AT(CHR(0), buffer)` is for and what every VFP program does.
          return cell.toString('latin1', 0, cell.length - 1);
        }
        return Number((cell as unknown[])[0]);
      });
      return { value: value ?? null, written };
    },
  };
}
