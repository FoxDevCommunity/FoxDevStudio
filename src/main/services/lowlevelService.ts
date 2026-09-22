/**
 * The low-level file functions and the file commands, on real files.
 *
 * `FOPEN()` hands a program a handle it reads and writes through until `FCLOSE()`; `ADIR()`
 * lists a folder; `COPY FILE` and `RENAME` move files about. Each arrives as one operation with
 * the fields it needs, and answers with a value and the error number Visual FoxPro's `FERROR()`
 * would report: 2 for a file that is not there, 5 for one that cannot be touched, 6 for a handle
 * that is not open, 31 for anything else.
 *
 * Text crosses as one character per byte, which is how every byte-shaped thing travels between
 * the VM and the host, so a program can read a binary file with FREAD() and see its bytes.
 */

import { closeSync, fstatSync, ftruncateSync, mkdirSync, openSync, readSync, readdirSync, renameSync, rmdirSync, statSync, writeSync, copyFileSync, fsyncSync } from 'node:fs';
import { basename, dirname, extname, isAbsolute, join, resolve } from 'node:path';

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

export type LowLevelValue = string | number | boolean | null | LowLevelValue[];

export interface LowLevelResult {
  value: LowLevelValue;
  /** The FERROR() number: 0 when the operation went through. */
  error: number;
}

export interface LowLevelService {
  perform(request: LowLevelRequest): LowLevelResult;
  /** Lets go of every handle a run left open. */
  closeAll(): void;
}

const NOT_FOUND = 2;
const ACCESS_DENIED = 5;
const BAD_HANDLE = 6;
const FAILURE = 31;

function errorNumber(error: unknown): number {
  const code = (error as { code?: string }).code;
  if (code === 'ENOENT') return NOT_FOUND;
  if (code === 'EACCES' || code === 'EPERM' || code === 'EBUSY') return ACCESS_DENIED;
  if (code === 'EBADF') return BAD_HANDLE;
  return FAILURE;
}

const latin1 = (buffer: Buffer): string => buffer.toString('latin1');
const bytesOf = (text: string): Buffer => Buffer.from(text, 'latin1');

/** DIR-style attributes of a file: the letters ADIR() puts in its fifth column. */
function attributes(path: string): string {
  try {
    const stat = statSync(path);
    return stat.isDirectory() ? 'D' : 'A';
  } catch {
    return '';
  }
}

function pad2(n: number): string {
  return String(n).padStart(2, '0');
}

/** A file's date as ISO text and its time as hh:mm:ss, the way ADIR() answers. */
function stamp(path: string): [string, string] {
  try {
    const m = statSync(path).mtime;
    return [`${m.getFullYear()}-${pad2(m.getMonth() + 1)}-${pad2(m.getDate())}`, `${pad2(m.getHours())}:${pad2(m.getMinutes())}:${pad2(m.getSeconds())}`];
  } catch {
    return ['', ''];
  }
}

/** A file-name glob: a star is any run of characters, a question mark one; case does not count. */
function glob(mask: string, name: string): boolean {
  const pattern = new RegExp(`^${mask.replace(/[.+^${}()|[\]\\]/g, '\\$&').replace(/\*/g, '.*').replace(/\?/g, '.')}$`, 'i');
  return pattern.test(name);
}

export function createLowLevelService(): LowLevelService {
  /** Open handles: the descriptor and where the next read or write goes. */
  const handles = new Map<number, { fd: number; position: number; path: string }>();
  let next = 1;

  const opened = (handle: number) => handles.get(handle);

  const perform = (r: LowLevelRequest): LowLevelResult => {
    const ok = (value: LowLevelValue): LowLevelResult => ({ value, error: 0 });
    const fail = (value: LowLevelValue, error: number): LowLevelResult => ({ value, error });
    try {
      switch (r.op) {
        case 'open': {
          // 0 read, 1 write, 2 read and write; opening for writing does not create the file
          const flag = r.count === 1 ? 'r+' : r.count === 2 ? 'r+' : 'r';
          const fd = openSync(r.path, flag);
          handles.set(next, { fd, position: 0, path: r.path });
          return ok(next++);
        }
        case 'create': {
          const fd = openSync(r.path, 'w+');
          handles.set(next, { fd, position: 0, path: r.path });
          return ok(next++);
        }
        case 'close': {
          const h = opened(r.handle);
          if (!h) return fail(false, BAD_HANDLE);
          closeSync(h.fd);
          handles.delete(r.handle);
          return ok(true);
        }
        case 'read': {
          const h = opened(r.handle);
          if (!h) return fail('', BAD_HANDLE);
          const buffer = Buffer.alloc(Math.max(0, r.count));
          const got = readSync(h.fd, buffer, 0, buffer.length, h.position);
          h.position += got;
          return ok(latin1(buffer.subarray(0, got)));
        }
        case 'gets': {
          const h = opened(r.handle);
          if (!h) return fail('', BAD_HANDLE);
          const buffer = Buffer.alloc(Math.max(0, r.count));
          const got = readSync(h.fd, buffer, 0, buffer.length, h.position);
          const chunk = buffer.subarray(0, got);
          const lf = chunk.indexOf(0x0a);
          // a line ends at the line feed, which is consumed and not returned
          if (lf < 0) {
            h.position += got;
            return ok(latin1(chunk));
          }
          h.position += lf + 1;
          const line = chunk.subarray(0, lf);
          return ok(latin1(line[line.length - 1] === 0x0d ? line.subarray(0, -1) : line));
        }
        case 'write': {
          const h = opened(r.handle);
          if (!h) return fail(0, BAD_HANDLE);
          const data = bytesOf(r.text);
          const wrote = writeSync(h.fd, data, 0, data.length, h.position);
          h.position += wrote;
          return ok(wrote);
        }
        case 'seek': {
          const h = opened(r.handle);
          if (!h) return fail(-1, BAD_HANDLE);
          const size = fstatSync(h.fd).size;
          const base = r.whence === 1 ? h.position : r.whence === 2 ? size : 0;
          h.position = Math.max(0, base + r.offset);
          return ok(h.position);
        }
        case 'eof': {
          const h = opened(r.handle);
          if (!h) return fail(true, BAD_HANDLE);
          return ok(h.position >= fstatSync(h.fd).size);
        }
        case 'flush': {
          const h = opened(r.handle);
          if (!h) return fail(false, BAD_HANDLE);
          fsyncSync(h.fd);
          return ok(true);
        }
        case 'chsize': {
          const h = opened(r.handle);
          if (!h) return fail(-1, BAD_HANDLE);
          ftruncateSync(h.fd, r.count);
          return ok(r.count);
        }
        case 'exists':
          try {
            statSync(r.path);
            return ok(true);
          } catch {
            return ok(false);
          }
        case 'copy':
          copyFileSync(r.path, r.target);
          return ok(null);
        case 'rename':
          renameSync(r.path, r.target);
          return ok(null);
        case 'mkdir':
          mkdirSync(r.path, { recursive: true });
          return ok(null);
        case 'rmdir':
          rmdirSync(r.path);
          return ok(null);
        case 'dir': {
          // the mask names a folder and a pattern; a folder alone lists everything in it
          const folder = extname(r.path) === '' && attributes(r.path) === 'D' ? r.path : dirname(r.path);
          const mask = folder === r.path ? '*' : basename(r.path);
          const rows: LowLevelValue[] = [];
          for (const name of readdirSync(folder).sort()) {
            if (!glob(mask, name)) continue;
            const full = join(folder, name);
            const attr = attributes(full);
            // ADIR() lists files unless asked for directories with the D attribute
            if (attr === 'D' && !r.target.toUpperCase().includes('D')) continue;
            const [date, time] = stamp(full);
            const size = attr === 'D' ? 0 : statSync(full).size;
            rows.push([name, size, { $date: date } as unknown as LowLevelValue, time, attr]);
          }
          return ok(rows);
        }
        case 'fullpath': {
          // FULLPATH answers in upper case, as Visual FoxPro does
          const base = r.target ? dirname(resolve(r.target)) : process.cwd();
          return ok((isAbsolute(r.path) ? resolve(r.path) : resolve(base, r.path)).toUpperCase());
        }
        case 'diskspace':
          // Node has no portable free-space call without a native module; a large number keeps
          // the programs that guard on it running
          return ok(1024 * 1024 * 1024);
        case 'drivetype':
          return ok(3);
        case 'locfile':
          try {
            statSync(r.path);
            return ok(resolve(r.path));
          } catch {
            return fail('', NOT_FOUND);
          }
        default:
          return fail(null, FAILURE);
      }
    } catch (error) {
      return fail(null, errorNumber(error));
    }
  };

  return {
    perform,
    closeAll() {
      for (const h of handles.values()) {
        try {
          closeSync(h.fd);
        } catch {
          // already gone
        }
      }
      handles.clear();
    },
  };
}
