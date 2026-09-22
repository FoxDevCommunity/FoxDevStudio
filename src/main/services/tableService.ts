/**
 * Open tables, on the main process side of the data engine.
 *
 * The host owns bytes and the VM owns meaning: nothing here parses a record or knows what a field
 * is. It opens a file, reads a run of bytes at an offset, writes one back, and closes. That is
 * what lets a table be larger than the VM's own address space: offsets are JavaScript numbers,
 * exact past 2^53, where Visual FoxPro stops at 2 GB because it computes them in signed 32 bits.
 *
 * Bytes cross to the VM as one character per byte. A page of records is tens of kilobytes and
 * marshalling that as an array of numbers costs more than the read did.
 */

import { open, type FileHandle } from 'node:fs/promises';
import { basename, dirname, extname, join } from 'node:path';
import { readdir, readFile, rm, writeFile } from 'node:fs/promises';

/** A table the VM has open. The handle is an opaque number to everyone but this file. */
interface OpenTable {
  path: string;
  dbf: FileHandle;
  /** The memo file, when the table has one and it could be found. A memo written to a table
   *  that has none makes it. */
  memo: FileHandle | null;
  /** Bytes per memo block, read from the memo header. */
  blockSize: number;
}

export interface TableService {
  open(path: string, exclusive: boolean): Promise<{ handle: number; header: string; writable: boolean }>;
  /** Writes an empty table from its header, and an empty memo file beside it when asked. */
  create(path: string, header: string, memo: boolean): Promise<void>;
  read(handle: number, offset: number, length: number): Promise<string>;
  readMemo(handle: number, block: number): Promise<string>;
  /** Appends a memo to the file beside the table; answers with the block it went to. */
  writeMemo(handle: number, bytes: string): Promise<number>;
  write(handle: number, offset: number, bytes: string): Promise<void>;
  close(handle: number): Promise<void>;
  /** The whole compound index beside the table, or an empty string when there is none. */
  readIndex(handle: number): Promise<string>;
  /** Writes that index and marks the table as having one; an empty string removes it. */
  writeIndex(handle: number, bytes: string): Promise<void>;
  /** Closes everything; called when a session ends so no file is left open. */
  closeAll(): Promise<void>;
}

/** Enough of the header to know how long the rest of it is. */
const PREAMBLE = 32;

export function createTableService(): TableService {
  const tables = new Map<number, OpenTable>();
  let next = 1;

  const get = (handle: number): OpenTable => {
    const table = tables.get(handle);
    if (!table) throw new Error(`Table handle ${handle} is not open`);
    return table;
  };

  return {
    async open(path, exclusive) {
      // 'r+' so a later REPLACE can write; a table that is read-only on disk still opens to read,
      // and says so, because a browser that quietly refuses every edit is worse than one that
      // says the file cannot be written
      let writable = exclusive;
      const dbf = await open(path, exclusive ? 'r+' : 'r').catch(() => {
        writable = false;
        return open(path, 'r');
      });
      const preamble = Buffer.alloc(PREAMBLE);
      await dbf.read(preamble, 0, PREAMBLE, 0);
      const headerLength = preamble.readUInt16LE(8);
      const header = Buffer.alloc(headerLength);
      await dbf.read(header, 0, headerLength, 0);

      const memo = await openMemo(path);
      const handle = next++;
      tables.set(handle, { path, dbf, memo: memo?.file ?? null, blockSize: memo?.blockSize ?? 0 });
      return { handle, header: toLatin1(header), writable };
    },

    async create(path, header, memo) {
      await writeFile(path, Buffer.concat([fromLatin1(header), Buffer.from([0x1a])]));
      if (memo) {
        // an FPT with no blocks yet: the next free block follows the header, blocks are 64 bytes
        const head = Buffer.alloc(512);
        head.writeUInt32BE(8, 0);
        head.writeUInt16BE(64, 6);
        await writeFile(join(dirname(path), `${basename(path, extname(path))}.fpt`), head);
      }
    },

    async read(handle, offset, length) {
      const table = get(handle);
      const buffer = Buffer.alloc(length);
      const { bytesRead } = await table.dbf.read(buffer, 0, length, offset);
      // a short read is the end of the file, which the caller notices by getting fewer records
      return toLatin1(buffer.subarray(0, bytesRead));
    },

    async readMemo(handle, block) {
      const table = get(handle);
      if (!table.memo || block <= 0) return '';
      // a memo block opens with an 8-byte big-endian header: a type and the payload length
      const head = Buffer.alloc(8);
      const start = block * table.blockSize;
      const got = await table.memo.read(head, 0, 8, start);
      if (got.bytesRead < 8) return '';
      const payload = Buffer.alloc(head.readUInt32BE(4));
      await table.memo.read(payload, 0, payload.length, start + 8);
      return toLatin1(Buffer.concat([head, payload]));
    },

    async writeMemo(handle, bytes) {
      const table = get(handle);
      if (!table.memo) {
        // a table can be given a memo field after it was made, and then the file has to exist
        const path = join(dirname(table.path), `${basename(table.path, extname(table.path))}.fpt`);
        const head = Buffer.alloc(512);
        head.writeUInt32BE(8, 0);
        head.writeUInt16BE(64, 6);
        await writeFile(path, head);
        table.memo = await open(path, 'r+');
        table.blockSize = 64;
      }
      const size = table.blockSize || 512;
      const head = Buffer.alloc(8);
      await table.memo.read(head, 0, 8, 0);
      const next = head.readUInt32BE(0) || 1;
      // a block opens with its type and the length of what follows, both big-endian, and is
      // padded out to a whole number of blocks
      const payload = fromLatin1(bytes);
      const written = Buffer.alloc(Math.ceil((8 + payload.length) / size) * size);
      written.writeUInt32BE(1, 0);
      written.writeUInt32BE(payload.length, 4);
      payload.copy(written, 8);
      await table.memo.write(written, 0, written.length, next * size);
      const after = Buffer.alloc(4);
      after.writeUInt32BE(next + written.length / size, 0);
      await table.memo.write(after, 0, 4, 0);
      return next;
    },

    async write(handle, offset, bytes) {
      const table = get(handle);
      await table.dbf.write(fromLatin1(bytes), 0, bytes.length, offset);
    },

    async readIndex(handle) {
      const table = get(handle);
      const path = await beside(table.path, 'cdx');
      if (!path) return '';
      return readFile(path)
        .then(toLatin1)
        .catch(() => '');
    },

    async writeIndex(handle, bytes) {
      const table = get(handle);
      const path = (await beside(table.path, 'cdx')) ?? structuralIndexOf(table.path);
      if (bytes.length === 0) await rm(path, { force: true });
      else await writeFile(path, fromLatin1(bytes));
      // byte 28 of a table's header says a structural index sits beside it, and Visual FoxPro
      // reads it to decide whether to open one
      await table.dbf.write(Buffer.from([bytes.length === 0 ? 0 : 1]), 0, 1, 28).catch(() => undefined);
    },

    async close(handle) {
      const table = tables.get(handle);
      if (!table) return;
      tables.delete(handle);
      await table.dbf.close().catch(() => undefined);
      await table.memo?.close().catch(() => undefined);
    },

    async closeAll() {
      const open = [...tables.values()];
      tables.clear();
      for (const table of open) {
        await table.dbf.close().catch(() => undefined);
        await table.memo?.close().catch(() => undefined);
      }
    },
  };
}

/** The name a table's structural index takes: the table's own, with a `.cdx` on the end. */
function structuralIndexOf(path: string): string {
  return join(dirname(path), `${basename(path, extname(path))}.cdx`);
}

/**
 * A file beside a table sharing its name, found whatever case it is written in: the samples are
 * not consistent about that, and Windows does not care but a name comparison does.
 */
async function beside(path: string, ext: string): Promise<string | null> {
  const stem = basename(path, extname(path)).toLowerCase();
  const dir = dirname(path);
  try {
    const names = await readdir(dir);
    const match = names.find((n) => {
      const dot = n.lastIndexOf('.');
      return dot > 0 && n.slice(0, dot).toLowerCase() === stem && n.slice(dot + 1).toLowerCase() === ext;
    });
    return match ? join(dir, match) : null;
  } catch {
    return null;
  }
}

/**
 * The memo file beside a table. Visual FoxPro pairs `.dbf` with `.fpt`, and the samples are not
 * consistent about case, so the directory is searched rather than a name being guessed.
 */
async function openMemo(path: string): Promise<{ file: FileHandle; blockSize: number } | null> {
  const stem = basename(path, extname(path)).toLowerCase();
  const dir = dirname(path);
  let match: string | undefined;
  try {
    const names = await readdir(dir);
    match = names.find((n) => {
      const dot = n.lastIndexOf('.');
      return dot > 0 && n.slice(0, dot).toLowerCase() === stem && n.slice(dot + 1).toLowerCase() === 'fpt';
    });
  } catch {
    return null;
  }
  if (!match) return null;

  try {
    const path = join(dir, match);
    const file = await open(path, 'r+').catch(() => open(path, 'r'));
    const head = Buffer.alloc(8);
    await file.read(head, 0, 8, 0);
    const raw = head.readUInt16BE(6);
    return { file, blockSize: raw === 0 ? 512 : raw };
  } catch {
    return null;
  }
}

/** One character per byte, which is exact for 0..255 and cheap on both sides of the bridge. */
function toLatin1(bytes: Buffer): string {
  return bytes.toString('latin1');
}

function fromLatin1(text: string): Buffer {
  return Buffer.from(text, 'latin1');
}
