import type { ProjectDocument } from '../project/schema';
import type { LibraryHost } from '../runtime/libraryHost';

/**
 * What arrived on a `FoxScript.Http` server. `server` is the handle the VM created it under,
 * which is what the route table is keyed by.
 */
export interface HttpRequestIn {
  server: number;
  method: string;
  /** The path with no query string, which is what a route is matched against. */
  path: string;
  query: Record<string, string>;
  /** Header names lower-cased, which is how HTTP treats them. */
  headers: Record<string, string>;
  body: string;
}

/** What to write back to a client. */
export interface HttpResponseOut {
  status: number;
  headers: Record<string, string>;
  body: string;
}

export interface FileFilter {
  name: string;
  extensions: string[];
}

export interface FileDialogOptions {
  title?: string;
  filters?: FileFilter[];
  defaultPath?: string;
}

export interface BuildExeOptions {
  /** Application name; becomes <name>.exe and the folder name. */
  name: string;
  /** Directory to create the application folder in. */
  outDir: string;
  /** Bundle text to embed as resources/app.fxa. */
  bundle: string;
}

export interface BuildExeResult {
  ok: boolean;
  /** Full path of the produced executable when ok. */
  exePath?: string;
  error?: string;
}

export interface MessageOptions {
  type: 'question' | 'warning' | 'info' | 'error';
  message: string;
  detail?: string;
  buttons: string[];
  defaultId?: number;
  cancelId?: number;
}

export interface ProjectHandle {
  path: string;
  doc: ProjectDocument;
}

/**
 * The API exposed to the renderer by the preload script as `window.foxdev`.
 * Renderer code must only reach the OS through this surface (no fs, no electron imports).
 */
/** One value crossing to a COM object. An object is a handle the main process holds. */
export interface OleValue {
  kind: 'null' | 'number' | 'string' | 'bool' | 'date' | 'object';
  num?: number;
  text?: string;
  flag?: boolean;
  handle?: number;
}

/** One `DECLARE ... DLL` call, as the VM describes it. */
export interface DllCallRequest {
  library: string;
  function: string;
  /** VFP type word of the return value; empty when the declaration omitted it. */
  returns: string;
  params: string[];
  byRef: boolean[];
  args: (string | number | boolean | null)[];
}

export interface DllCallResult {
  value: string | number | boolean | null;
  /** What the library wrote into the arguments passed by reference. */
  written: (string | number | null)[];
}

/** One low-level file operation, as the VM describes it: FOPEN() and its kin, ADIR(), COPY FILE. */
export interface LowLevelRequest {
  op: string;
  handle: number;
  path: string;
  target: string;
  text: string;
  count: number;
  offset: number;
  whence: number;
}

export type LowLevelValue = string | number | boolean | null | LowLevelValue[] | { $date: string };

export interface LowLevelResult {
  value: LowLevelValue;
  /** The FERROR() number: 0 when the operation went through. */
  error: number;
}

export interface FoxDevApi {
  project: {
    /** Creates `<dir>/<name>.fxproject` (and the directory) and returns it. */
    create(dir: string, name: string): Promise<ProjectHandle>;
    /** Reads and validates a project file. Rejects with a message on failure. */
    open(path: string): Promise<ProjectHandle>;
    save(path: string, doc: ProjectDocument): Promise<void>;
    /**
     * Asks to read `path`, granting its folder when it sits near something already reachable.
     * This is how an imported Visual FoxPro project reaches the class libraries beside it and at
     * the product root; anything further away is refused.
     */
    allowNear(path: string): Promise<boolean>;
    /** GETENV(): what the operating system has the name set to, empty when it is not set. */
    getEnv(name: string): Promise<string>;
    /** The directory HOME(n) names, empty when there is none. */
    homeDir(which: number, appDir: string): Promise<string>;
    /** RUN: hands a command line to the command processor; answers what it exited with. */
    run(command: string, cwd: string, nowait: boolean): Promise<number>;
    /**
     * The same, but waits and hands back what the command wrote. Source control is the caller:
     * a provider is a program that answers on its output, not one that just exits.
     */
    capture(command: string, cwd: string): Promise<{ code: number; out: string; err: string }>;
    /**
     * Where the bundled Visual FoxPro Foundation Classes live. A form records only a file name
     * for these (`CLASSLOC` is `_base.vcx`), because VFP finds them on its own search path, so
     * the importer needs somewhere to look.
     */
    classLibraryDir(): Promise<string>;
  };
  /**
   * `DECLARE ... DLL`: a function of a native library, called through koffi in the main
   * process. The VM waits for the answer, so this is a promise like every other request.
   */
  dll: {
    /** Whether a native library can be called at all in this build. */
    available(): Promise<boolean>;
    call(request: DllCallRequest): Promise<DllCallResult>;
  };
  /**
   * `SET LIBRARY TO`: a Visual FoxPro library, hosted in the 32-bit process that can load one.
   *
   * Synchronous for the same reason COM is: a program may call a library function from inside an
   * expression the runtime is already evaluating - `TYPE([Hash("a", 5)])` - and there is nowhere
   * to put a promise there.
   */
  library: LibraryHost;
  /**
   * COM automation, on Windows, when the addon that does it was built.
   *
   * These are synchronous on purpose: the VM asks the host for a property while it is on the
   * stack, so a promise would arrive too late to be an answer. Each call blocks the renderer
   * for as long as the COM server takes, which is what Visual FoxPro does too.
   */
  ole: {
    /** Whether COM can be reached at all: false off Windows, and in a build without the addon. */
    available(): boolean;
    /** `CREATEOBJECT(cProgId)`. Throws with the COM message when the object cannot be made. */
    create(progId: string): number;
    /** `GETOBJECT()`: an object already running under that name, or the one a file stands for. */
    active(name: string, className: string): number;
    get(handle: number, name: string, args: OleValue[]): OleValue;
    set(handle: number, name: string, value: OleValue): void;
    call(handle: number, name: string, args: OleValue[]): OleValue;
    release(handle: number): void;
    /** Lets go of every object: the end of a run. */
    releaseAll(): void;
  };
  /**
   * Open tables. The host owns bytes and the VM owns meaning: nothing here parses a record.
   * Offsets are byte offsets from the start of the file.
   */
  data: {
    /**
     * Opens a table, answering with a handle, its raw header bytes, and whether it can be
     * written. A file that is read-only on disk still opens to read.
     */
    open(path: string, exclusive: boolean): Promise<{ handle: number; header: string; writable: boolean }>;
    /**
     * Writes an empty table: the header given (latin-1, one character per byte) and the
     * end-of-file byte, plus an empty memo file beside it when `memo` is set.
     */
    create(path: string, header: string, memo: boolean): Promise<void>;
    read(handle: number, offset: number, length: number): Promise<string>;
    readMemo(handle: number, block: number): Promise<string>;
    /** Writes a memo beside the table and answers with the block it went to. */
    writeMemo(handle: number, bytes: string): Promise<number>;
    write(handle: number, offset: number, bytes: string): Promise<void>;
    close(handle: number): Promise<void>;
    /** The whole `.cdx` beside the table, or an empty string when there is none. */
    readIndex(handle: number): Promise<string>;
    /** Writes that `.cdx`, and marks the table as having one. An empty string removes it. */
    writeIndex(handle: number, bytes: string): Promise<void>;
  };
  files: {
    readText(path: string): Promise<string>;
    writeText(path: string, text: string): Promise<void>;
    exists(path: string): Promise<boolean>;
    /** ERASE / DELETE FILE; false when there was nothing to remove. */
    remove(path: string): Promise<boolean>;
    /** The low-level file functions and file commands, one operation at a time. */
    lowlevel(request: LowLevelRequest): Promise<LowLevelResult>;
    /** Binary write, one character per byte, for Visual FoxPro's DBF-based files. */
    writeBytes(path: string, bytes: string): Promise<void>;
    /** Binary read, for Visual FoxPro's DBF-based files. */
    readBytes(path: string): Promise<Uint8Array>;
    /** File names directly inside a directory. */
    listDir(dir: string): Promise<string[]>;
  };
  dialog: {
    openFile(opts: FileDialogOptions): Promise<string | null>;
    saveFile(opts: FileDialogOptions): Promise<string | null>;
    pickFolder(opts?: { title?: string }): Promise<string | null>;
    /** Resolves with the index of the pressed button. */
    message(opts: MessageOptions): Promise<number>;
  };
  player: {
    /** The bundle this window was launched to run, or null in the IDE. */
    getBundle(): Promise<{ path: string; text: string } | null>;
  };
  build: {
    /** Produces a standalone application folder around a bundle. */
    exe(opts: BuildExeOptions): Promise<BuildExeResult>;
  };
  /**
   * `FoxScript.Http`: the sockets, which live in the main process.
   *
   * The runtime never asks for a request; the main process hands one over the moment it
   * arrives, and the runtime answers with the same id. That is the one direction nothing else
   * in this API goes, and it is why `HostEvent` exists.
   */
  http: {
    /** Opens a port; answers the one actually bound, which `0` makes the system choose. */
    listen(server: number, port: number): Promise<number>;
    close(server: number): Promise<boolean>;
    /** Subscribes to arriving requests. Returns an unsubscribe function. */
    onRequest(cb: (id: number, request: HttpRequestIn) => void): () => void;
    /** What to write back; `null` means nothing in the runtime answered it. */
    respond(id: number, response: HttpResponseOut | null): Promise<void>;
  };
  app: {
    getVersion(): Promise<string>;
    /** Project file passed on the command line or via FOXDEV_OPEN, if any. */
    getStartupProject(): Promise<string | null>;
    getRecentProjects(): Promise<string[]>;
    addRecentProject(path: string): Promise<void>;
    setTitle(title: string): Promise<void>;
    setDocumentEdited(edited: boolean): Promise<void>;
    /** Native-menu commands. Returns an unsubscribe function. */
    onMenuCommand(cb: (commandId: string) => void): () => void;
    /** Main asks before closing the window; answer with confirmClose. */
    onCloseRequested(cb: () => void): () => void;
    confirmClose(ok: boolean): Promise<void>;
  };
}
