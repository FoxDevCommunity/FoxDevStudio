import type { IpcChannel, IpcReq, IpcRes } from '@shared/ipc/channels';
import type { BuildExeOptions, BuildExeResult, FileDialogOptions, HttpRequestIn, HttpResponseOut, MessageOptions } from '@shared/ipc/api';
import { dirname } from '@shared/paths';
import type { FileService } from '../services/fileService';
import type { RecentFiles } from '../services/recentFiles';
import { createTableService, type TableService } from '../services/tableService';
import { createDllService, type DllService } from '../services/dllService';
import { createLowLevelService, type LowLevelService } from '../services/lowlevelService';
import { createHttpService, type HttpService } from '@shared/runtime/httpService';
import { spawn } from 'node:child_process';
import { homeDir } from '../services/foxproHome';
import { createProjectService } from '../services/projectService';
import { PathGuard } from './pathGuard';

/** Dependencies the handlers need; injected so tests can run them without Electron. */
export interface HandlerDeps {
  getVersion(): string;
  /** Project file to open at startup (CLI argument or FOXDEV_OPEN). */
  startupProject?: string | null;
  fs: FileService;
  recent: RecentFiles;
  /** Folder holding the bundled Visual FoxPro Foundation Classes. */
  classLibraryDir: string;
  /** Open tables; created here when not supplied. */
  tables?: TableService;
  /** Native library calls; created here when not supplied. */
  dlls?: DllService;
  /** Low-level file handles; created here when not supplied. */
  lowlevel?: LowLevelService;
  /** `FoxScript.Http` sockets; created here when not supplied. */
  http?: HttpService;
  /** How a request that arrived reaches the window that runs the program. */
  sendHttpRequest?(id: number, request: HttpRequestIn): void;
  dialogs: {
    openFile(opts: FileDialogOptions): Promise<string | null>;
    saveFile(opts: FileDialogOptions): Promise<string | null>;
    pickFolder(opts?: { title?: string }): Promise<string | null>;
    message(opts: MessageOptions): Promise<number>;
  };
  window: {
    setTitle(title: string): void;
    setDocumentEdited(edited: boolean): void;
    confirmClose(ok: boolean): void;
  };
  guard?: PathGuard;
  /** Player mode: the bundle this window runs, already allowed by the guard. */
  playBundle?: string | null;
  /** Produces a standalone application folder; absent in tests that do not build. */
  buildExe?(opts: BuildExeOptions): Promise<BuildExeResult>;
}

export type Handlers = {
  [K in IpcChannel]: (...args: IpcReq<K>) => Promise<IpcRes<K>> | IpcRes<K>;
};

export function createHandlers(deps: HandlerDeps): Handlers {
  const guard = deps.guard ?? new PathGuard();
  const dlls = deps.dlls ?? createDllService();
  const lowlevel = deps.lowlevel ?? createLowLevelService();
  const projects = createProjectService(deps.fs);
  // the path is checked when a table is opened; after that a handle is the only way in
  const tables = deps.tables ?? createTableService();

  // A request the runtime has not answered yet, by the id it was handed over under. The main
  // process waits for `http:respond`; a window that goes away without answering would leave a
  // client hanging, so a request nobody answers is written as a 404 by the service itself.
  const pending = new Map<number, (response: HttpResponseOut | null) => void>();
  let nextRequest = 1;
  const http = deps.http ?? createHttpService();
  http.onRequest(
    (request) =>
      new Promise<HttpResponseOut | null>((answered) => {
        const id = nextRequest++;
        pending.set(id, answered);
        if (deps.sendHttpRequest) deps.sendHttpRequest(id, request);
        else answered(null);
      }),
  );

  const allowResult = <T extends string | null>(p: T, asDir = false): T => {
    if (p && asDir) guard.allowDir(p);
    else if (p) guard.allowFile(p);
    return p;
  };

  return {
    'app:getVersion': () => deps.getVersion(),
    'app:getStartupProject': () => {
      const p = deps.startupProject ?? null;
      if (p) guard.allowDir(dirname(p));
      return p;
    },
    'app:getRecentProjects': () => deps.recent.list(),
    'app:addRecentProject': (path) => deps.recent.add(path),
    'app:setTitle': (title) => deps.window.setTitle(title),
    'app:setDocumentEdited': (edited) => deps.window.setDocumentEdited(edited),
    'app:confirmClose': (ok) => deps.window.confirmClose(ok),

    'project:create': async (dir, name) => {
      guard.check(dir);
      const handle = await projects.create(dir, name);
      guard.allowDir(dir);
      return handle;
    },
    'project:open': async (path) => {
      // recent projects were recorded by main itself, so they are trusted like dialog results
      if (!guard.isAllowed(path) && !(await deps.recent.list()).includes(path)) guard.check(path);
      const handle = await projects.open(path);
      guard.allowDir(dirname(path));
      return handle;
    },
    /**
     * A Visual FoxPro project keeps shared classes and data in folders beside its own -
     * solution.pjx refers to `..\classes\samples.vcx` - so importing one needs more than the
     * project's own directory.
     *
     * The widening is one level, from a path the renderer can already reach. That bounds it to
     * the neighbourhood of something the user chose in a dialog, rather than letting the
     * renderer name any folder it likes.
     */
    'project:allowNear': (path) => guard.isAllowed(path) || guard.allowNeighbour(path),

    // RUN hands a command line to the command processor, which is what it is for. It runs
    // where the project is, so a relative command means what the program means by it.
    'app:run': async (command, cwd, nowait) => {
      if (command.trim() === '') return 0;
      const options = { cwd: cwd === '' ? undefined : cwd, windowsHide: false };
      if (nowait) {
        const child = spawn(command, { ...options, shell: true, detached: true, stdio: 'ignore' });
        child.unref();
        return 0;
      }
      return new Promise<number>((resolve) => {
        const child = spawn(command, { ...options, shell: true, stdio: 'ignore' });
        child.on('error', () => resolve(-1));
        child.on('close', (code) => resolve(code ?? 0));
      });
    },

    // The same as app:run, waiting and keeping what the command wrote. The source-control
    // provider reads its answers off git's output, so exit codes on their own are not enough.
    'app:capture': async (command, cwd) => {
      if (command.trim() === '') return { code: 0, out: '', err: '' };
      return new Promise((resolve) => {
        const child = spawn(command, {
          cwd: cwd === '' ? undefined : cwd,
          shell: true,
          windowsHide: true,
          stdio: ['ignore', 'pipe', 'pipe'],
        });
        let out = '';
        let err = '';
        child.stdout?.on('data', (chunk: Buffer) => {
          out += chunk.toString('utf8');
        });
        child.stderr?.on('data', (chunk: Buffer) => {
          err += chunk.toString('utf8');
        });
        child.on('error', (e: Error) => resolve({ code: -1, out, err: e.message }));
        child.on('close', (code: number | null) => resolve({ code: code ?? 0, out, err }));
      });
    },

    // GETENV(): the environment the application was started in, which a program reads to
    // find out where it is running
    'app:getEnv': (name) => process.env[name] ?? process.env[name.toUpperCase()] ?? '',

    // HOME() and _SAMPLES: where Visual FoxPro is installed, which is the only thing those
    // names can mean. What is handed over is granted, the way a dialog result is.
    'project:homeDir': async (which, appDir) => {
      const dir = await homeDir(which, appDir);
      if (dir !== '') guard.allowDir(dir);
      return dir;
    },

    'app:getClassLibraryDir': () => {
      guard.allowDir(deps.classLibraryDir);
      return deps.classLibraryDir;
    },

    'project:save': async (path, doc) => {
      guard.check(path);
      await projects.save(path, doc);
    },

    'data:open': async (path, exclusive) => {
      guard.check(path);
      return tables.open(path, exclusive);
    },
    'dll:available': () => dlls.available(),
    'dll:call': (request) => dlls.call({ ...request, byRef: request.byRef }),
    'data:create': async (path, header, memo) => {
      guard.check(path);
      await tables.create(path, header, memo);
    },
    'data:read': (handle, offset, length) => tables.read(handle, offset, length),
    'data:readMemo': (handle, block) => tables.readMemo(handle, block),
    'data:writeMemo': (handle, bytes) => tables.writeMemo(handle, bytes),
    'data:write': (handle, offset, bytes) => tables.write(handle, offset, bytes),
    'data:close': (handle) => tables.close(handle),
    'data:readIndex': (handle) => tables.readIndex(handle),
    'data:writeIndex': (handle, bytes) => tables.writeIndex(handle, bytes),

    'files:readText': async (path) => {
      guard.check(path);
      return deps.fs.readText(path);
    },
    'files:writeText': async (path, text) => {
      guard.check(path);
      return deps.fs.writeText(path, text);
    },
    'files:exists': async (path) => {
      guard.check(path);
      return deps.fs.exists(path);
    },
    'files:lowlevel': (request) => {
      // every path an operation names must be one the renderer may reach
      if (request.path) guard.check(request.path);
      if (request.target && request.op !== 'fullpath' && request.op !== 'dir') guard.check(request.target);
      return lowlevel.perform(request);
    },
    'files:remove': async (path) => {
      guard.check(path);
      return deps.fs.remove(path);
    },
    'files:readBytes': async (path) => {
      guard.check(path);
      return deps.fs.readBytes(path);
    },
    'files:writeBytes': async (path, bytes) => {
      guard.check(path);
      const out = new Uint8Array(bytes.length);
      for (let i = 0; i < bytes.length; i++) out[i] = bytes.charCodeAt(i) & 0xff;
      await deps.fs.writeBytes(path, out);
    },
    'files:listDir': async (dir) => {
      guard.check(dir);
      return deps.fs.listDir(dir);
    },

    'dialog:openFile': async (opts) => allowResult(await deps.dialogs.openFile(opts)),
    'dialog:saveFile': async (opts) => allowResult(await deps.dialogs.saveFile(opts)),
    'dialog:pickFolder': async (opts) => allowResult(await deps.dialogs.pickFolder(opts), true),
    'dialog:message': (opts) => deps.dialogs.message(opts),

    'player:getBundle': async () => {
      if (!deps.playBundle) return null;
      return { path: deps.playBundle, text: await deps.fs.readText(deps.playBundle) };
    },
    'build:exe': async (opts) => {
      if (!deps.buildExe) return { ok: false, error: 'Building executables is not available here.' };
      guard.check(opts.outDir);
      return deps.buildExe(opts);
    },
    'http:listen': (server, port) => http.listen(server, port),
    'http:close': (server) => http.close(server),
    'http:respond': (id, response) => {
      pending.get(id)?.(response);
      pending.delete(id);
    },
  };
}
