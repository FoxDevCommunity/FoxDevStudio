import type { DllCallRequest, DllCallResult, LowLevelRequest, LowLevelResult } from './api';
import type { ProjectDocument } from '../project/schema';
import type { BuildExeOptions, BuildExeResult, FileDialogOptions, HttpRequestIn, HttpResponseOut, MessageOptions, ProjectHandle } from './api';

/**
 * Typed IPC contract shared by main (handle), preload (invoke) and tests.
 * Every channel maps to its request tuple and response type so a typo is a compile error.
 */
export interface IpcMap {
  'app:getVersion': { req: []; res: string };
  'app:getStartupProject': { req: []; res: string | null };
  'app:getRecentProjects': { req: []; res: string[] };
  'app:addRecentProject': { req: [path: string]; res: void };
  'app:setTitle': { req: [title: string]; res: void };
  'app:setDocumentEdited': { req: [edited: boolean]; res: void };
  'app:confirmClose': { req: [ok: boolean]; res: void };
  'project:create': { req: [dir: string, name: string]; res: ProjectHandle };
  'project:open': { req: [path: string]; res: ProjectHandle };
  'project:save': { req: [path: string, doc: ProjectDocument]; res: void };
  /** Asks to read a path near one already reachable; see `PathGuard.allowNeighbour`. */
  'project:allowNear': { req: [path: string]; res: boolean };
  /** RUN: a command line for the command processor, answered with what it exited with. */
  'app:run': { req: [command: string, cwd: string, nowait: boolean]; res: number };
  'app:capture': {
    req: [command: string, cwd: string];
    res: { code: number; out: string; err: string };
  };
  /** GETENV(): what the operating system has the name set to, or an empty string. */
  'app:getEnv': { req: [name: string]; res: string };
  /** The directory HOME(n) names; what is handed back is readable. */
  'project:homeDir': { req: [which: number, appDir: string]; res: string };
  /** Folder holding the bundled Visual FoxPro Foundation Classes, already readable. */
  'app:getClassLibraryDir': { req: []; res: string };

  // The data engine. Bytes travel as one character per byte, and offsets as plain numbers,
  // which stay exact past 2^53 - far beyond the 2 GB Visual FoxPro stops at.
  'data:open': { req: [path: string, exclusive: boolean]; res: { handle: number; header: string; writable: boolean } };
  'dll:available': { req: []; res: boolean };
  'dll:call': { req: [request: DllCallRequest]; res: DllCallResult };
  'data:create': { req: [path: string, header: string, memo: boolean]; res: void };
  'data:read': { req: [handle: number, offset: number, length: number]; res: string };
  'data:readMemo': { req: [handle: number, block: number]; res: string };
  'data:write': { req: [handle: number, offset: number, bytes: string]; res: void };
  'data:close': { req: [handle: number]; res: void };
  // The compound index beside a table, whole: small enough to hand over in one piece, and read
  // in one piece because a tag is a tree that has to be walked from its root anyway.
  'data:writeMemo': { req: [handle: number, bytes: string]; res: number };
  // A whole file of bytes, one character each: the database container is written this way.
  'files:writeBytes': { req: [path: string, bytes: string]; res: void };
  'data:readIndex': { req: [handle: number]; res: string };
  'data:writeIndex': { req: [handle: number, bytes: string]; res: void };
  'files:readText': { req: [path: string]; res: string };
  'files:writeText': { req: [path: string, text: string]; res: void };
  'files:exists': { req: [path: string]; res: boolean };
  /** ERASE / DELETE FILE; false when there was nothing to remove. */
  'files:remove': { req: [path: string]; res: boolean };
  'files:lowlevel': { req: [request: LowLevelRequest]; res: LowLevelResult };
  /** Binary read for the DBF-based Visual FoxPro formats. */
  'files:readBytes': { req: [path: string]; res: Uint8Array };
  'files:listDir': { req: [dir: string]; res: string[] };
  'dialog:openFile': { req: [opts: FileDialogOptions]; res: string | null };
  'dialog:saveFile': { req: [opts: FileDialogOptions]; res: string | null };
  'dialog:pickFolder': { req: [opts: { title?: string } | undefined]; res: string | null };
  'dialog:message': { req: [opts: MessageOptions]; res: number };
  /** Player mode: the bundle this window was launched to run, if any. */
  'player:getBundle': { req: []; res: { path: string; text: string } | null };
  /** Copies the installed app next to a bundle to make a standalone executable. */
  'build:exe': { req: [opts: BuildExeOptions]; res: BuildExeResult };
  /** `oServer.Listen(nPort)`: opens a port and answers the one actually bound. */
  'http:listen': { req: [server: number, port: number]; res: number };
  /** `oServer.Close()`. */
  'http:close': { req: [server: number]; res: boolean };
  /** What the runtime wrote for a request the main process handed it. */
  'http:respond': { req: [id: number, response: HttpResponseOut | null]; res: void };
}

export type IpcChannel = keyof IpcMap;
export type IpcReq<K extends IpcChannel> = IpcMap[K]['req'];
export type IpcRes<K extends IpcChannel> = IpcMap[K]['res'];

/** Events pushed from main to renderer (webContents.send). */
export interface IpcEvents {
  'menu:command': [commandId: string];
  'window:closeRequested': [];
  /**
   * A request arrived on a `FoxScript.Http` server. The runtime answers it on `http:respond`
   * with the same id; that is the host speaking first, which is what a HostEvent is for.
   */
  'http:request': [id: number, request: HttpRequestIn];
}
export type IpcEvent = keyof IpcEvents;
