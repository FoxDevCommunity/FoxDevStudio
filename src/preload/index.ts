import { contextBridge, ipcRenderer } from 'electron';
import type { FoxDevApi, OleValue } from '@shared/ipc/api';
import type { LibraryCall, LibraryLoad } from '@shared/runtime/libraryHost';
import type { IpcChannel, IpcEvent, IpcEvents, IpcReq, IpcRes } from '@shared/ipc/channels';

function invoke<K extends IpcChannel>(channel: K, ...args: IpcReq<K>): Promise<IpcRes<K>> {
  return ipcRenderer.invoke(channel, ...args) as Promise<IpcRes<K>>;
}

function on<K extends IpcEvent>(channel: K, cb: (...args: IpcEvents[K]) => void): () => void {
  const listener = (_e: Electron.IpcRendererEvent, ...args: unknown[]) => cb(...(args as IpcEvents[K]));
  ipcRenderer.on(channel, listener);
  return () => ipcRenderer.removeListener(channel, listener);
}

/**
 * COM, synchronously.
 *
 * `sendSync` blocks this renderer until the main process answers, which is exactly what a
 * property read from inside the VM needs; a failure comes back as a message and is thrown here
 * so the caller sees an ordinary exception.
 */
function ole(op: string, ...args: unknown[]): OleValue {
  const reply = ipcRenderer.sendSync('ole:sync', op, args) as { ok: true; value: OleValue } | { ok: false; error: string };
  if (!reply.ok) throw new Error(reply.error);
  return reply.value;
}

/**
 * A hosted Visual FoxPro library, synchronously, for the same reason COM is: `SET LIBRARY TO`
 * puts a library's functions into the language, and a program calls one from the middle of an
 * expression the VM is already evaluating.
 */
function library(op: string, ...args: unknown[]): unknown {
  const reply = ipcRenderer.sendSync('library:sync', op, args) as { ok: true; value: unknown } | { ok: false; error: string };
  if (!reply.ok) throw new Error(reply.error);
  return reply.value;
}

const api: FoxDevApi = {
  library: {
    available: () => library('available') === true,
    load: (path) => library('load', path) as LibraryLoad,
    call: (lib, fn, args) => library('call', lib, fn, args) as LibraryCall,
    unload: (lib) => void library('unload', lib),
    releaseAll: () => void library('releaseAll'),
  },
  ole: {
    available: () => ole('available').flag === true,
    create: (progId) => ole('create', progId).handle ?? 0,
    active: (name, className) => ole('active', name, className).handle ?? 0,
    get: (handle, name, args) => ole('get', handle, name, args),
    set: (handle, name, value) => void ole('set', handle, name, value),
    call: (handle, name, args) => ole('call', handle, name, args),
    release: (handle) => void ole('release', handle),
    releaseAll: () => void ole('releaseAll'),
  },
  project: {
    create: (dir, name) => invoke('project:create', dir, name),
    open: (path) => invoke('project:open', path),
    save: (path, doc) => invoke('project:save', path, doc),
    allowNear: (path) => invoke('project:allowNear', path),
    getEnv: (name) => invoke('app:getEnv', name),
    homeDir: (which, appDir) => invoke('project:homeDir', which, appDir),
    run: (command, cwd, nowait) => invoke('app:run', command, cwd, nowait),
    capture: (command, cwd) => invoke('app:capture', command, cwd),
    classLibraryDir: () => invoke('app:getClassLibraryDir'),
  },
  dll: {
    available: () => invoke('dll:available'),
    call: (request) => invoke('dll:call', request),
  },
  data: {
    open: (path, exclusive) => invoke('data:open', path, exclusive),
    create: (path, header, memo) => invoke('data:create', path, header, memo),
    read: (handle, offset, length) => invoke('data:read', handle, offset, length),
    readMemo: (handle, block) => invoke('data:readMemo', handle, block),
    writeMemo: (handle, bytes) => invoke('data:writeMemo', handle, bytes),
    write: (handle, offset, bytes) => invoke('data:write', handle, offset, bytes),
    close: (handle) => invoke('data:close', handle),
    readIndex: (handle) => invoke('data:readIndex', handle),
    writeIndex: (handle, bytes) => invoke('data:writeIndex', handle, bytes),
  },
  files: {
    readText: (path) => invoke('files:readText', path),
    writeText: (path, text) => invoke('files:writeText', path, text),
    exists: (path) => invoke('files:exists', path),
    remove: (path) => invoke('files:remove', path),
    lowlevel: (request) => invoke('files:lowlevel', request),
    readBytes: (path) => invoke('files:readBytes', path),
    writeBytes: (path, bytes) => invoke('files:writeBytes', path, bytes),
    listDir: (dir) => invoke('files:listDir', dir),
  },
  dialog: {
    openFile: (opts) => invoke('dialog:openFile', opts),
    saveFile: (opts) => invoke('dialog:saveFile', opts),
    pickFolder: (opts) => invoke('dialog:pickFolder', opts),
    message: (opts) => invoke('dialog:message', opts),
  },
  player: {
    getBundle: () => invoke('player:getBundle'),
  },
  build: {
    exe: (opts) => invoke('build:exe', opts),
  },
  http: {
    listen: (server, port) => invoke('http:listen', server, port),
    close: (server) => invoke('http:close', server),
    onRequest: (cb) => on('http:request', cb),
    respond: (id, response) => invoke('http:respond', id, response),
  },
  app: {
    getVersion: () => invoke('app:getVersion'),
    getStartupProject: () => invoke('app:getStartupProject'),
    getRecentProjects: () => invoke('app:getRecentProjects'),
    addRecentProject: (path) => invoke('app:addRecentProject', path),
    setTitle: (title) => invoke('app:setTitle', title),
    setDocumentEdited: (edited) => invoke('app:setDocumentEdited', edited),
    onMenuCommand: (cb) => on('menu:command', cb),
    onCloseRequested: (cb) => on('window:closeRequested', cb),
    confirmClose: (ok) => invoke('app:confirmClose', ok),
  },
};

contextBridge.exposeInMainWorld('foxdev', api);
