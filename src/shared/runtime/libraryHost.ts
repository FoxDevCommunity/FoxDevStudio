/**
 * `SET LIBRARY TO`: hosting a Visual FoxPro library.
 *
 * A .fll is not a .dll a program declares functions out of. It is a library written against
 * FoxPro's own C API - the host hands it a table of API services and it answers with a table of
 * the functions it adds to the language - and every one ever shipped is a 32-bit Win32 image.
 * Node is 64-bit, so nothing in this application's own processes can load one; `fllhost.exe`,
 * built for x86 from `native/fllhost/fllhost.c`, is the process that does, and this is the side
 * that talks to it.
 *
 * The conversation is synchronous, and has to be: a program may call a library function from
 * inside an expression the runtime is in the middle of evaluating - `TYPE([Hash("a", 5)])` is
 * the smallest case - and there is nowhere to put a promise there. Node cannot wait on a child
 * process's own pipes without giving up the stack, so the host makes a named pipe and this end
 * opens it as a file: `fs.readSync` and `fs.writeSync` on it block, which is the whole point.
 * Measured: two thousand round trips take about fifty milliseconds.
 *
 * Nothing here is imported the way a Node module usually is. This file is reached from the
 * renderer's in-memory API as well as from the main process, and the renderer is built for a
 * browser where `node:fs` does not resolve at all - so the modules are asked for at run time
 * through `process.getBuiltinModule`, and where there is no `process` there is simply no host.
 */

/** A function a library adds to the language. */
export interface LibraryFunction {
  /** Its name, in capitals, as the library declared it. */
  name: string;
  /** How many parameters it declares, or -1 internal, -2 call on load, -3 call on unload. */
  parmCount: number;
  /** One letter per parameter, comma separated, a dot in front of an optional one: "C,.I". */
  parmTypes: string;
}

/** One value crossing to or from a library function. */
export type LibraryValue =
  | { kind: 'none' }
  | { kind: 'string'; text: string }
  | { kind: 'number'; num: number }
  | { kind: 'logical'; flag: boolean }
  | { kind: 'date'; text: string }
  | { kind: 'datetime'; text: string }
  /** A variable passed with @: the library reads and writes it through _Load and _Store. */
  | { kind: 'ref'; value: LibraryValue };

export interface LibraryLoad {
  /** How this library is named in later calls. */
  id: number;
  /** The file the host actually opened, which is what `SET("LIBRARY")` reports. */
  path: string;
  functions: LibraryFunction[];
  /** Anything a call-on-load function printed. */
  output: string;
}

export interface LibraryCall {
  value: LibraryValue;
  /** What the function printed with _PutStr. */
  output: string;
  /** The number the function gave _Error, -1 for _UserError, 0 when it raised nothing. */
  error: number;
  /** What it gave _UserError. */
  errorText: string;
  /** API functions it asked for that this host does not have, as their numbers. */
  missing: string;
  /** What it stored in the variables it was handed by reference, by argument position. */
  refs: { index: number; value: LibraryValue }[];
}

export interface LibraryHost {
  /** Whether a library can be hosted at all here: Windows, with the host built. */
  available(): boolean;
  /** Loads one. Throws with what went wrong when it will not load. */
  load(path: string): LibraryLoad;
  call(library: number, fn: number, args: LibraryValue[]): LibraryCall;
  /** Lets one go, running its call-on-unload functions first. */
  unload(library: number): void;
  /** Lets go of every library and of the host process itself. */
  releaseAll(): void;
}

/** A Node built-in, when this is running on Node at all. */
function builtin<T>(name: string): T | null {
  const proc = (globalThis as { process?: { getBuiltinModule?: (n: string) => unknown } }).process;
  const get = proc?.getBuiltinModule;
  if (!get) return null;
  try {
    return (get.call(proc, name) as T) ?? null;
  } catch {
    return null;
  }
}

interface NodeFs {
  existsSync(path: string): boolean;
  openSync(path: string, flags: string): number;
  closeSync(fd: number): void;
  readSync(fd: number, buffer: Uint8Array, offset: number, length: number, position: null): number;
  writeSync(fd: number, buffer: Uint8Array): number;
}
interface NodeChild {
  spawn(
    command: string,
    args: string[],
    options: { stdio: string[]; windowsHide: boolean },
  ): { pid?: number; kill(): void; on(event: string, cb: () => void): void };
}

/** A wait that does not give up the stack, for the moment between spawning and connecting. */
function pause(ms: number): void {
  Atomics.wait(new Int32Array(new SharedArrayBuffer(4)), 0, 0, ms);
}

/** Builds one frame: its length, then its bytes. */
class Frame {
  private parts: Uint8Array[] = [];
  private total = 0;
  private add(bytes: Uint8Array): this {
    this.parts.push(bytes);
    this.total += bytes.length;
    return this;
  }
  u8(v: number): this {
    return this.add(new Uint8Array([v & 0xff]));
  }
  u16(v: number): this {
    const b = new Uint8Array(2);
    new DataView(b.buffer).setUint16(0, v & 0xffff, true);
    return this.add(b);
  }
  u32(v: number): this {
    const b = new Uint8Array(4);
    new DataView(b.buffer).setUint32(0, v >>> 0, true);
    return this.add(b);
  }
  i32(v: number): this {
    const b = new Uint8Array(4);
    new DataView(b.buffer).setInt32(0, v | 0, true);
    return this.add(b);
  }
  f64(v: number): this {
    const b = new Uint8Array(8);
    new DataView(b.buffer).setFloat64(0, v, true);
    return this.add(b);
  }
  /** A string as bytes: one byte per character, which is what a library reads. */
  private static latin1(text: string): Uint8Array {
    const b = new Uint8Array(text.length);
    for (let i = 0; i < text.length; i++) b[i] = text.charCodeAt(i) & 0xff;
    return b;
  }
  text(s: string): this {
    const b = Frame.latin1(s);
    return this.u16(b.length).add(b);
  }
  bytes32(s: string): this {
    const b = Frame.latin1(s);
    return this.u32(b.length).add(b);
  }
  done(): Uint8Array {
    const out = new Uint8Array(4 + this.total);
    new DataView(out.buffer).setUint32(0, this.total, true);
    let at = 4;
    for (const p of this.parts) {
      out.set(p, at);
      at += p.length;
    }
    return out;
  }
}

/** Reads one frame back. */
class Reply {
  private at = 0;
  private view: DataView;
  constructor(private bytes: Uint8Array) {
    this.view = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength);
  }
  u8(): number {
    return this.view.getUint8(this.at++);
  }
  remaining(): number {
    return this.bytes.byteLength - this.at;
  }
  u16(): number {
    const v = this.view.getUint16(this.at, true);
    this.at += 2;
    return v;
  }
  i16(): number {
    const v = this.view.getInt16(this.at, true);
    this.at += 2;
    return v;
  }
  u32(): number {
    const v = this.view.getUint32(this.at, true);
    this.at += 4;
    return v;
  }
  i32(): number {
    const v = this.view.getInt32(this.at, true);
    this.at += 4;
    return v;
  }
  f64(): number {
    const v = this.view.getFloat64(this.at, true);
    this.at += 8;
    return v;
  }
  private chars(n: number): string {
    let out = '';
    for (let i = 0; i < n; i++) out += String.fromCharCode(this.bytes[this.at + i]!);
    this.at += n;
    return out;
  }
  text(): string {
    return this.chars(this.u16());
  }
  bytes32(): string {
    return this.chars(this.u32());
  }
  value(): LibraryValue {
    const tag = String.fromCharCode(this.u8());
    switch (tag) {
      case 'C':
        return { kind: 'string', text: this.bytes32() };
      case 'N': {
        const num = this.f64();
        this.u16();
        this.u16();
        return { kind: 'number', num };
      }
      case 'I': {
        const num = this.i32();
        this.u16();
        return { kind: 'number', num };
      }
      case 'L':
        return { kind: 'logical', flag: this.u8() !== 0 };
      case 'Y': {
        // currency crosses as the two halves of the scaled integer FoxPro keeps it in
        const lo = this.u32();
        const hi = this.i32();
        return { kind: 'number', num: (hi * 4294967296 + lo) / 10000 };
      }
      case 'D':
      case 'T':
        return { kind: 'number', num: this.f64() };
      case 'S':
        return { kind: 'date', text: this.bytes32() };
      case 'W':
        return { kind: 'datetime', text: this.bytes32() };
      default:
        return { kind: 'none' };
    }
  }
}

/**
 * @param dirs where to look for `fllhost.exe`: the app folder in development, the unpacked
 * resources folder in a packaged build.
 */
export function createLibraryHost(dirs: string[]): LibraryHost {
  const fs = builtin<NodeFs>('fs');
  const cp = builtin<NodeChild>('child_process');

  const exe = (): string | null => {
    const proc = (globalThis as { process?: { platform?: string } }).process;
    if (!fs || !cp || proc?.platform !== 'win32') return null;
    for (const dir of dirs) {
      const path = `${dir}/fllhost.exe`;
      if (fs.existsSync(path)) return path;
    }
    return null;
  };

  let fd = -1;
  let child: { kill(): void } | null = null;
  let started = false;

  /** Starts the host if it is not already up. Answers whether there is one to talk to. */
  const start = (): boolean => {
    if (fd >= 0) return true;
    if (started) return false;
    started = true;
    const path = exe();
    if (!path || !fs || !cp) return false;
    const now = Date.now();
    const name = `\\\\.\\pipe\\foxdev-fll-${(globalThis as { process?: { pid?: number } }).process?.pid ?? 0}-${now}`;
    child = cp.spawn(path, [name], { stdio: ['ignore', 'ignore', 'ignore'], windowsHide: true });
    // the host has to have made the pipe before this end can open it, and there is no way to
    // be told: it is tried until it opens or five seconds have gone
    const until = now + 5000;
    for (;;) {
      try {
        fd = fs.openSync(name, 'r+');
        return true;
      } catch (error) {
        if (Date.now() > until) {
          child?.kill();
          child = null;
          throw new Error(`the library host would not start: ${String(error)}`, { cause: error });
        }
        pause(5);
      }
    }
  };

  const stop = (): void => {
    if (fd >= 0 && fs) {
      try {
        fs.closeSync(fd);
      } catch {
        // the host may already be gone, which is the state we were after
      }
    }
    fd = -1;
    child?.kill();
    child = null;
    started = false;
  };

  const exchange = (frame: Uint8Array): Reply => {
    if (!start() || !fs) throw new Error('a Visual FoxPro library cannot be hosted here');
    try {
      fs.writeSync(fd, frame);
      const exact = (n: number): Uint8Array => {
        const out = new Uint8Array(n);
        let got = 0;
        while (got < n) {
          const k = fs.readSync(fd, out, got, n - got, null);
          if (k <= 0) throw new Error('the library host went away');
          got += k;
        }
        return out;
      };
      const length = new DataView(exact(4).buffer).getUint32(0, true);
      return new Reply(exact(length));
    } catch (error) {
      // a host that has died takes its libraries with it; the next call starts a new one
      stop();
      throw error instanceof Error ? error : new Error(String(error));
    }
  };

  /** The first byte of every reply says whether it worked; a failure carries its own words. */
  const ok = (reply: Reply): Reply => {
    if (reply.u8() !== 0) throw new Error(reply.text());
    return reply;
  };

  return {
    available: () => exe() !== null,

    load(path) {
      const reply = ok(exchange(new Frame().u8(1).text(path).done()));
      const id = reply.u16();
      const full = reply.text();
      const count = reply.u16();
      const functions: LibraryFunction[] = [];
      for (let i = 0; i < count; i++) {
        functions.push({ name: reply.text(), parmCount: reply.i16(), parmTypes: reply.text() });
      }
      return { id, path: full, functions, output: reply.bytes32() };
    },

    call(library, fn, args) {
      const frame = new Frame().u8(2).u16(library).u16(fn).u16(args.length);
      for (const arg of args) {
        // a variable passed by reference is its value with an 'R' in front
        if (arg.kind === 'ref') frame.u8(0x52);
        const a = arg.kind === 'ref' ? arg.value : arg;
        switch (a.kind) {
          case 'string':
            frame.u8(0x43).bytes32(a.text);
            break;
          case 'number':
            frame.u8(0x4e).f64(a.num).u16(10).u16(0);
            break;
          case 'logical':
            frame.u8(0x4c).u16(a.flag ? 1 : 0);
            break;
          default:
            frame.u8(0x30);
            break;
        }
      }
      const reply = ok(exchange(frame.done()));
      const answer: LibraryCall = {
        value: reply.value(),
        output: reply.bytes32(),
        error: reply.i32(),
        errorText: reply.text(),
        missing: reply.text(),
        refs: [],
      };
      // a host built before references were carried ends its reply here
      if (reply.remaining() >= 2) {
        const count = reply.u16();
        for (let i = 0; i < count; i++) answer.refs.push({ index: reply.u16(), value: reply.value() });
      }
      return answer;
    },

    unload(library) {
      if (fd >= 0) ok(exchange(new Frame().u8(3).u16(library).done()));
    },

    releaseAll() {
      stop();
    },
  };
}
