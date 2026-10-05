import { chmod, mkdtemp, readFile, rm, writeFile } from 'node:fs/promises';
import { join } from 'node:path';
import { tmpdir } from 'node:os';
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { createTableService } from '@main/services/tableService';

let root: string;
let tables: ReturnType<typeof createTableService>;
beforeEach(async () => {
  root = await mkdtemp(join(tmpdir(), 'foxdev-table-write-'));
  tables = createTableService();
});
afterEach(async () => {
  await tables.closeAll();
  await rm(root, { recursive: true, force: true });
});

describe('writable table handles', () => {
  it.each([false, true])('persists bytes when exclusive=%s', async (exclusive) => {
    const path = join(root, 'sample.dbf');
    // The service treats DBF contents as bytes; only header length is decoded.
    const original = Buffer.alloc(40);
    original.writeUInt16LE(32, 8);
    original.writeUInt16LE(8, 10);
    await writeFile(path, original);
    const opened = await tables.open(path, exclusive);
    await tables.write(opened.handle, 32, ' changed');
    await tables.close(opened.handle);
    const expected = Buffer.from(original);
    expected.write(' changed', 32, 'latin1');
    expect(await readFile(path)).toEqual(expected);
    expect(opened.writable).toBe(true);
  });
});

// POSIX mode bits provide a real read-only file; Windows and root need another fixture.
it.skipIf(process.platform === 'win32' || process.getuid?.() === 0)('falls back to read-only when writing is denied', async () => {
  const path = join(root, 'readonly.dbf');
  const bytes = Buffer.alloc(32);
  bytes.writeUInt16LE(32, 8);
  await writeFile(path, bytes);
  await chmod(path, 0o444);
  try {
    const opened = await tables.open(path, false);
    expect(opened.writable).toBe(false);
    expect(await tables.read(opened.handle, 0, 32)).toBe(bytes.toString('latin1'));
  } finally {
    await chmod(path, 0o644);
  }
});
