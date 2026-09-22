import type { BuildExeOptions, BuildExeResult, FileDialogOptions, FoxDevApi, LowLevelRequest, LowLevelResult, LowLevelValue, MessageOptions } from '@shared/ipc/api';
import type { ProjectDocument } from '@shared/project/schema';
import { createEmptyProjectDocument, parseProjectDocument, stringifyProjectDocument } from '@shared/project/serialize';
import { join, normalize } from '@shared/paths';
import { createLibraryHost } from '@shared/runtime/libraryHost';

type DialogKind = 'openFile' | 'saveFile' | 'pickFolder' | 'message';

/**
 * In-memory FoxDevApi used in tests and when running outside Electron (plain browser).
 * Files live in a Map; dialogs are answered from a scripted queue (`queueDialog`).
 */
export interface MemoryApi extends FoxDevApi {
  files$: Map<string, string>;
  recent$: string[];
  title$: string;
  edited$: boolean;
  closeConfirmations$: boolean[];
  /** Records every dialog call so tests can assert on titles/filters. */
  dialogCalls$: { kind: DialogKind; opts: unknown }[];
  queueDialog(kind: 'openFile' | 'saveFile' | 'pickFolder', answer: string | null): void;
  queueDialog(kind: 'message', answer: number): void;
  emitMenuCommand(id: string): void;
  emitCloseRequested(): void;
  /** Binary files, for the DBF-based Visual FoxPro formats. */
  binary$: Map<string, Uint8Array>;
  /** Where the bundled Foundation Classes would be; tests point it at a folder of their own. */
  classLibraryDir$: string;
  /** Player mode: the bundle this window should run, if any. */
  bundle$: { path: string; text: string } | null;
  /** Every Build Executable request, and the answer tests want it to give. */
  buildCalls$: BuildExeOptions[];
  buildResult$: BuildExeResult;
}

export function createMemoryApi(seed: Record<string, string> = {}): MemoryApi {
  const files = new Map<string, string>(Object.entries(seed));
  /**
   * The name a file is held under.
   *
   * A real file service is given a path and hands it to the operating system, which knows that
   * `Solution\..\Data\x.dbf` and `Data\x.dbf` are the same file. This one is a map, so unless
   * the name is worked out first the two miss each other - and a test that reads a form's tables
   * the way the form names them, one folder up, would say the file is not there when it is.
   */
  const at = (path: string): string => normalize(path).split('\\').join('/');
  /**
   * The key one of the two maps holds `path` under, or the worked-out name when nothing does.
   *
   * Windows does not mind how a name is capitalised and neither does the service the renderer
   * really talks to. This one is a Map, whose keys are the names the files were seeded under, so
   * without this a form that names `..\..\..\data\testdata.dbc` - which is what every sample's
   * data environment does, in lower case, where the folder on disk is `Data` - is told its table
   * is not there when it is.
   */
  const keyOf = (path: string): string => {
    const wanted = at(path);
    if (files.has(wanted) || api.binary$.has(wanted)) return wanted;
    const lower = wanted.toLowerCase();
    for (const name of files.keys()) if (at(name).toLowerCase() === lower) return name;
    for (const name of api.binary$.keys()) if (at(name).toLowerCase() === lower) return name;
    return wanted;
  };
  /** Tables opened through the data API; the handle is the 1-based position. */
  const openTables: Uint8Array[] = [];
  /** The memo file beside each open table, when there is one. */
  const openMemos: (Uint8Array | null)[] = [];
  /** Where the compound index of each open table lives in the binary map. */
  const openIndexes: string[] = [];
  /** The name a file beside a table takes, matched case-insensitively as the real service does. */
  const besideName = (path: string, ext: string): string => {
    const dot = path.lastIndexOf('.');
    const stem = (dot > 0 ? path.slice(0, dot) : path).toLowerCase();
    for (const name of api.binary$.keys()) if (name.toLowerCase() === `${stem}.${ext}`) return name;
    return `${dot > 0 ? path.slice(0, dot) : path}.${ext}`;
  };
  /** The `.fpt` beside a table, matched case-insensitively the way the real service does. */
  const memoFor = (path: string): Uint8Array | null => {
    const dot = path.lastIndexOf('.');
    const stem = (dot > 0 ? path.slice(0, dot) : path).toLowerCase();
    for (const [name, bytes] of api.binary$) if (name.toLowerCase() === `${stem}.fpt`) return bytes;
    return null;
  };
  const menuListeners = new Set<(id: string) => void>();
  const closeListeners = new Set<() => void>();
  const queues: Record<DialogKind, unknown[]> = { openFile: [], saveFile: [], pickFolder: [], message: [] };

  const answer = <T>(kind: DialogKind, opts: unknown, fallback: T): T => {
    api.dialogCalls$.push({ kind, opts });
    const q = queues[kind];
    return q.length ? (q.shift() as T) : fallback;
  };

  /** Bytes of a file in either map, or null; text files are latin-1 for this purpose. */
  const bytesAt = (path: string): Uint8Array | null => {
    const binary = api.binary$.get(path) ?? api.binary$.get(keyOf(path));
    if (binary) return binary;
    const text = files.get(path) ?? files.get(keyOf(path));
    if (text === undefined) return null;
    const out = new Uint8Array(text.length);
    for (let i = 0; i < text.length; i++) out[i] = text.charCodeAt(i) & 0xff;
    return out;
  };
  const storeBytes = (path: string, bytes: Uint8Array) => {
    files.delete(path);
    api.binary$.set(path, bytes);
  };
  /** A file-name glob: a star is any run of characters, a question mark one; case does not count. */
  const globMatches = (mask: string, name: string): boolean => {
    const escaped = mask.replace(/[.+^$(){}|[\]\\]/g, (c) => '\\' + c).replace(/\*/g, '.*').replace(/\?/g, '.');
    return new RegExp('^' + escaped + '$', 'i').test(name);
  };
  /** Open low-level handles: the path and where the next read or write goes. */
  const lowHandles = new Map<number, { path: string; position: number }>();
  let nextLow = 1;
  const lowlevelInMemory = (r: LowLevelRequest): LowLevelResult => {
    const ok = (value: LowLevelValue): LowLevelResult => ({ value, error: 0 });
    const fail = (value: LowLevelValue, error: number): LowLevelResult => ({ value, error });
    const latin1 = (b: Uint8Array) => Array.from(b, (c) => String.fromCharCode(c)).join('');
    const handle = () => lowHandles.get(r.handle);
    switch (r.op) {
      case 'open':
        if (bytesAt(r.path) === null) return fail(-1, 2);
        lowHandles.set(nextLow, { path: r.path, position: 0 });
        return ok(nextLow++);
      case 'create':
        storeBytes(r.path, new Uint8Array(0));
        lowHandles.set(nextLow, { path: r.path, position: 0 });
        return ok(nextLow++);
      case 'close':
        return lowHandles.delete(r.handle) ? ok(true) : fail(false, 6);
      case 'read':
      case 'gets': {
        const h = handle();
        if (!h) return fail('', 6);
        const bytes = bytesAt(h.path) ?? new Uint8Array(0);
        const rest = bytes.subarray(h.position, h.position + Math.max(0, r.count));
        if (r.op === 'read') {
          h.position += rest.length;
          return ok(latin1(rest));
        }
        const lf = rest.indexOf(0x0a);
        if (lf < 0) {
          h.position += rest.length;
          return ok(latin1(rest));
        }
        h.position += lf + 1;
        const line = rest.subarray(0, lf);
        return ok(latin1(line[line.length - 1] === 0x0d ? line.subarray(0, -1) : line));
      }
      case 'write': {
        const h = handle();
        if (!h) return fail(0, 6);
        const old = bytesAt(h.path) ?? new Uint8Array(0);
        const data = new Uint8Array(r.text.length);
        for (let i = 0; i < r.text.length; i++) data[i] = r.text.charCodeAt(i) & 0xff;
        const out = new Uint8Array(Math.max(old.length, h.position + data.length));
        out.set(old);
        out.set(data, h.position);
        storeBytes(h.path, out);
        h.position += data.length;
        return ok(data.length);
      }
      case 'seek': {
        const h = handle();
        if (!h) return fail(-1, 6);
        const size = (bytesAt(h.path) ?? new Uint8Array(0)).length;
        const base = r.whence === 1 ? h.position : r.whence === 2 ? size : 0;
        h.position = Math.max(0, base + r.offset);
        return ok(h.position);
      }
      case 'eof': {
        const h = handle();
        if (!h) return fail(true, 6);
        return ok(h.position >= (bytesAt(h.path) ?? new Uint8Array(0)).length);
      }
      case 'flush':
        return ok(handle() !== undefined);
      case 'chsize': {
        const h = handle();
        if (!h) return fail(-1, 6);
        const old = bytesAt(h.path) ?? new Uint8Array(0);
        const out = new Uint8Array(r.count);
        out.set(old.subarray(0, Math.min(old.length, r.count)));
        storeBytes(h.path, out);
        return ok(r.count);
      }
      case 'exists':
        return ok(bytesAt(r.path) !== null);
      case 'copy': {
        const bytes = bytesAt(r.path);
        if (!bytes) return fail(null, 2);
        storeBytes(r.target, new Uint8Array(bytes));
        return ok(null);
      }
      case 'rename': {
        const bytes = bytesAt(r.path);
        if (!bytes) return fail(null, 2);
        files.delete(r.path);
        api.binary$.delete(r.path);
        storeBytes(r.target, bytes);
        return ok(null);
      }
      case 'mkdir':
      case 'rmdir':
        return ok(null);
      case 'dir': {
        const folder = r.path.replace(/[\\/][^\\/]*$/, '');
        const mask = r.path.slice(folder.length + 1) || '*';
        const rows: LowLevelValue[] = [];
        for (const name of [...files.keys(), ...api.binary$.keys()].sort()) {
          const dir = name.replace(/[\\/][^\\/]*$/, '');
          const leaf = name.slice(dir.length + 1);
          if (dir.toLowerCase() !== folder.toLowerCase() || !globMatches(mask, leaf)) continue;
          rows.push([leaf, (bytesAt(name) ?? new Uint8Array(0)).length, { $date: '2026-09-07' }, '12:30:00', 'A']);
        }
        return ok(rows);
      }
      case 'fullpath':
        // FULLPATH answers in upper case, as Visual FoxPro does
        return ok((/^([A-Za-z]:|[\\/])/.test(r.path) ? r.path : 'C:/work/' + r.path).toUpperCase());
      case 'diskspace':
        return ok(1024 * 1024 * 1024);
      case 'drivetype':
        return ok(3);
      case 'locfile':
        return bytesAt(r.path) !== null ? ok(r.path) : fail('', 2);
      default:
        return fail(null, 31);
    }
  };

  const readText = async (path: string) => {
    const text = files.get(path) ?? files.get(keyOf(path));
    if (text === undefined) throw new Error(`File not found: ${path}`);
    return text;
  };

  const api: MemoryApi = {
    files$: files,
    recent$: [],
    title$: '',
    edited$: false,
    closeConfirmations$: [],
    dialogCalls$: [],
    queueDialog(kind: DialogKind, value: unknown) {
      queues[kind].push(value);
    },
    emitMenuCommand: (id) => menuListeners.forEach((cb) => cb(id)),
    emitCloseRequested: () => closeListeners.forEach((cb) => cb()),

    /**
     * A Visual FoxPro library is hosted by a child process, not loaded into this one, so this
     * is the real thing rather than a stand-in: wherever there is a Node to spawn it - a test
     * run, the standalone player - `SET LIBRARY TO` works. In a browser there is no `process`
     * to ask and the host reports itself unavailable, which the runtime turns into error 1726.
     */
    library: createLibraryHost(['resources/native/win32']),
    // nothing outside Electron can load a native library either
    dll: {
      async available() {
        return false;
      },
      async call() {
        throw new Error('DECLARE ... DLL is not available here');
      },
    },
    // no COM outside Electron on Windows; a program that asks is told, and carries on
    ole: {
      available: () => false,
      create(progId: string): number {
        throw new Error(`COM automation is not available: ${progId}`);
      },
      active(name: string): number {
        throw new Error(`COM automation is not available: ${name}`);
      },
      get() {
        throw new Error('COM automation is not available');
      },
      set() {
        throw new Error('COM automation is not available');
      },
      call() {
        throw new Error('COM automation is not available');
      },
      release() {},
      releaseAll() {},
    },
    project: {
      async create(dir, name) {
        const path = join(dir, `${name}.fxproject`);
        const doc = createEmptyProjectDocument(name);
        files.set(path, stringifyProjectDocument(doc));
        return { path, doc };
      },
      async open(path) {
        const r = parseProjectDocument(await readText(path));
        if (!r.ok) throw new Error(`Cannot open project: ${r.error}`);
        return { path, doc: r.doc };
      },
      async save(path, doc: ProjectDocument) {
        files.set(path, stringifyProjectDocument(doc));
      },
      // nothing is guarded in memory, so there is never anything to widen
      async allowNear() {
        return true;
      },
      // nothing is run and nothing is installed in an in-memory world
      async run() {
        return 0;
      },
      // and nothing answers: there is no source-control provider in memory either
      async capture() {
        return { code: -1, out: '', err: '' };
      },
      async getEnv(_name: string) {
        return '';
      },
      // nothing is installed in an in-memory world, so HOME() answers about the project only
      // Visual FoxPro's directory names end with a separator and programs join onto them, which
      // is what the real service is careful about; there is no installation to ask about here,
      // so only the two numbers that mean "where this is running" have an answer.
      async homeDir(which: number, appDir: string) {
        if (which !== 0 && which !== 1) return '';
        return appDir === '' || /[\\/]$/.test(appDir) ? appDir : `${appDir}\\`;
      },
      async classLibraryDir() {
        return api.classLibraryDir$;
      },
    },
    // Tables live in `binary$` as whole files; reads are slices of those bytes, which is what
    // the real service does against a file handle.
    data: {
      async open(path: string) {
        const held = keyOf(path);
        const bytes = api.binary$.get(held);
        if (!bytes) throw new Error(`File '${path}' does not exist`);
        const handle = openTables.push(bytes) as number;
        openMemos[handle - 1] = memoFor(held);
        openIndexes[handle - 1] = besideName(held, 'cdx');
        const headerLength = new DataView(bytes.buffer, bytes.byteOffset).getUint16(8, true);
        return { handle, header: latin1(bytes.subarray(0, headerLength)), writable: true };
      },
      async create(path: string, header: string, memo: boolean) {
        const bytes = new Uint8Array(header.length + 1);
        for (let i = 0; i < header.length; i++) bytes[i] = header.charCodeAt(i) & 0xff;
        bytes[header.length] = 0x1a;
        api.binary$.set(path, bytes);
        if (memo) {
          const head = new Uint8Array(512);
          head[3] = 8;
          head[7] = 64;
          api.binary$.set(path.replace(/\.[^.]+$/, '') + '.fpt', head);
        }
      },
      async read(handle: number, offset: number, length: number) {
        const bytes = openTables[handle - 1];
        return bytes ? latin1(bytes.subarray(offset, offset + length)) : '';
      },
      async readMemo(handle: number, block: number) {
        const memo = openMemos[handle - 1];
        if (!memo || block <= 0) return '';
        const size = new DataView(memo.buffer, memo.byteOffset).getUint16(6, false) || 512;
        const start = block * size;
        if (start + 8 > memo.length) return '';
        const length = new DataView(memo.buffer, memo.byteOffset + start).getUint32(4, false);
        return latin1(memo.subarray(start, start + 8 + length));
      },
      async writeMemo(handle: number, text: string) {
        let memo = openMemos[handle - 1];
        if (!memo) {
          memo = new Uint8Array(512);
          memo[3] = 8;
          memo[7] = 64;
          openMemos[handle - 1] = memo;
        }
        const view = new DataView(memo.buffer, memo.byteOffset);
        const size = view.getUint16(6, false) || 512;
        const next = view.getUint32(0, false) || 1;
        const length = Math.ceil((8 + text.length) / size) * size;
        const grown = new Uint8Array(Math.max(memo.length, next * size + length));
        grown.set(memo);
        const at = next * size;
        new DataView(grown.buffer).setUint32(at, 1, false);
        new DataView(grown.buffer).setUint32(at + 4, text.length, false);
        for (let i = 0; i < text.length; i++) grown[at + 8 + i] = text.charCodeAt(i) & 0xff;
        new DataView(grown.buffer).setUint32(0, next + length / size, false);
        openMemos[handle - 1] = grown;
        return next;
      },
      async write(handle: number, offset: number, text: string) {
        const bytes = openTables[handle - 1];
        if (bytes) for (let i = 0; i < text.length; i++) bytes[offset + i] = text.charCodeAt(i) & 0xff;
      },
      async readIndex(handle: number) {
        const bytes = api.binary$.get(openIndexes[handle - 1] ?? '');
        return bytes ? latin1(bytes) : '';
      },
      async writeIndex(handle: number, text: string) {
        const name = openIndexes[handle - 1];
        if (!name) return;
        if (text.length === 0) api.binary$.delete(name);
        else {
          const bytes = new Uint8Array(text.length);
          for (let i = 0; i < text.length; i++) bytes[i] = text.charCodeAt(i) & 0xff;
          api.binary$.set(name, bytes);
        }
        const table = openTables[handle - 1];
        if (table) table[28] = text.length === 0 ? 0 : 1;
      },
      async close() {
        // nothing to release when the bytes were never a file
      },
    },
    files: {
      readText,
      async writeText(path, text) {
        files.set(path, text);
      },
      async exists(path) {
        // a binary file is a file: the Visual FoxPro formats live in the binary map, and both
        // FILE() and the importer ask this before reading one
        const held = keyOf(path);
        return files.has(held) || api.binary$.has(held);
      },
      async remove(path) {
        const held = keyOf(path);
        return files.delete(held) || api.binary$.delete(held);
      },
      async lowlevel(request) {
        return lowlevelInMemory(request);
      },
      async writeBytes(path, bytes) {
        const out = new Uint8Array(bytes.length);
        for (let i = 0; i < bytes.length; i++) out[i] = bytes.charCodeAt(i) & 0xff;
        api.binary$.set(path, out);
      },
      async readBytes(path) {
        const binary = api.binary$.get(path) ?? api.binary$.get(keyOf(path));
        if (binary) return binary;
        const text = files.get(path) ?? files.get(keyOf(path));
        if (text === undefined) throw new Error(`File not found: ${path}`);
        return new TextEncoder().encode(text);
      },
      async listDir(dir) {
        const prefix = `${dir.replace(/[\\/]+$/, '')}/`;
        const names = new Set<string>();
        for (const path of [...files.keys(), ...api.binary$.keys()]) {
          const normalised = path.replace(/\\/g, '/');
          if (!normalised.startsWith(prefix)) continue;
          const rest = normalised.slice(prefix.length);
          if (!rest.includes('/')) names.add(rest);
        }
        return [...names];
      },
    },
    dialog: {
      async openFile(opts: FileDialogOptions) {
        return answer<string | null>('openFile', opts, null);
      },
      async saveFile(opts: FileDialogOptions) {
        return answer<string | null>('saveFile', opts, null);
      },
      async pickFolder(opts) {
        return answer<string | null>('pickFolder', opts, null);
      },
      async message(opts: MessageOptions) {
        return answer<number>('message', opts, opts.cancelId ?? 0);
      },
    },
    binary$: new Map<string, Uint8Array>(),
    classLibraryDir$: '/ffc',
    bundle$: null,
    buildCalls$: [],
    buildResult$: { ok: true, exePath: 'C:/out/App/App.exe' },
    player: {
      async getBundle() {
        return api.bundle$;
      },
    },
    build: {
      async exe(opts) {
        api.buildCalls$.push(opts);
        return api.buildResult$;
      },
    },
    // no sockets in memory: a program may open a server, and nothing ever knocks on it
    http: {
      listen: async (_server, port) => port,
      close: async () => true,
      onRequest: () => () => {},
      respond: async () => {},
    },
    app: {
      getVersion: async () => '0.0.0-memory',
      getStartupProject: async () => null,
      getRecentProjects: async () => [...api.recent$],
      async addRecentProject(path) {
        api.recent$ = [path, ...api.recent$.filter((p) => p !== path)].slice(0, 10);
      },
      async setTitle(title) {
        api.title$ = title;
      },
      async setDocumentEdited(edited) {
        api.edited$ = edited;
      },
      onMenuCommand: (cb) => {
        menuListeners.add(cb);
        return () => menuListeners.delete(cb);
      },
      onCloseRequested: (cb) => {
        closeListeners.add(cb);
        return () => closeListeners.delete(cb);
      },
      async confirmClose(ok) {
        api.closeConfirmations$.push(ok);
      },
    },
  };
  return api;
}

/** One character per byte, the way the real table service sends them. */
function latin1(bytes: Uint8Array): string {
  let out = '';
  for (const b of bytes) out += String.fromCharCode(b);
  return out;
}
