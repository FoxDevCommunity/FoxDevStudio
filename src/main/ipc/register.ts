import { ipcMain } from 'electron';
import type { Handlers } from './handlers';
import type { OleService } from '../services/oleService';
import type { LibraryService } from '../services/libraryService';

export function registerIpc(handlers: Handlers): void {
  for (const [channel, fn] of Object.entries(handlers)) {
    ipcMain.handle(channel, (_event, ...args: unknown[]) => (fn as (...a: unknown[]) => unknown)(...args));
  }
}

/**
 * COM, on its own channel, answered synchronously.
 *
 * Everything else the renderer asks for is a promise. This is not: the VM reads a property while
 * it is on the stack, so the answer has to be there before the call returns. `sendSync` blocks
 * the renderer for the length of the COM call, which is what Visual FoxPro does as well.
 */
export function registerOle(ole: OleService): void {
  ipcMain.on('ole:sync', (event, op: string, args: unknown[]) => {
    event.returnValue = ole.perform(op, args ?? []);
  });
}

/**
 * Visual FoxPro libraries, on their own channel, answered synchronously.
 *
 * The same reasoning as COM: `SET LIBRARY TO` makes a library's functions part of the language,
 * and a program calls one from inside an expression the VM is already evaluating. There is
 * nowhere in the middle of an expression to wait for a promise.
 */
export function registerLibrary(libraries: LibraryService): void {
  ipcMain.on('library:sync', (event, op: string, args: unknown[]) => {
    event.returnValue = libraries.perform(op, args ?? []);
  });
}
