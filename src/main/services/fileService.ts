import { access, mkdir, readFile, readdir, rm, writeFile } from 'node:fs/promises';
import iconv from 'iconv-lite';
import { dirname } from 'node:path';

export interface FileService {
  readText(path: string): Promise<string>;
  /** Binary read, for the DBF-based Visual FoxPro formats. */
  readBytes(path: string): Promise<Uint8Array>;
  /** Writes a whole file of bytes, for the Visual FoxPro formats that are not text. */
  writeBytes(path: string, bytes: Uint8Array): Promise<void>;
  /** File names (not paths) directly inside a directory. */
  listDir(dir: string): Promise<string[]>;
  writeText(path: string, text: string): Promise<void>;
  exists(path: string): Promise<boolean>;
  /** ERASE / DELETE FILE. True when a file was there to remove. */
  remove(path: string): Promise<boolean>;
  mkdirp(dir: string): Promise<void>;
}

/** The code page Visual FoxPro works in on a Western install, which is what it reads and writes. */
const WINDOWS_ANSI = 'win1252';

/**
 * A text file as text, in whatever it was written in.
 *
 * Reading everything as UTF-8 loses a byte that is not valid UTF-8: a Visual FoxPro file is
 * written in the ANSI code page of the machine that wrote it, so 'Taquería' comes back as
 * 'Taquer�a'. Anything valid as UTF-8 is UTF-8 - that is what the encoding was designed
 * for - and anything else is read as Windows-1252, which is what Visual FoxPro writes.
 */
export function decodeText(bytes: Uint8Array): string {
  const withoutBom = bytes[0] === 0xef && bytes[1] === 0xbb && bytes[2] === 0xbf ? bytes.subarray(3) : bytes;
  try {
    return new TextDecoder('utf-8', { fatal: true }).decode(withoutBom);
  } catch {
    return iconv.decode(Buffer.from(withoutBom), WINDOWS_ANSI);
  }
}

/**
 * Text as the bytes to write, in the code page Visual FoxPro works in.
 *
 * VFP stores text in the ANSI code page, Windows-1252 on a Western install, so a file written
 * here is written the same way and VFP reads back what it was given. A character Windows-1252
 * cannot hold would be written as a question mark and lost, so text that needs more than the
 * code page has is written as UTF-8 instead: an encoding VFP would not expect is better than
 * text that is gone.
 */
export function encodeText(text: string): Buffer {
  const bytes = iconv.encode(text, WINDOWS_ANSI);
  return iconv.decode(bytes, WINDOWS_ANSI) === text ? bytes : Buffer.from(text, 'utf8');
}

export const nodeFileService: FileService = {
  async readText(path) {
    return decodeText(await readFile(path));
  },
  async readBytes(path) {
    const buffer = await readFile(path);
    return new Uint8Array(buffer);
  },
  async writeBytes(path, bytes) {
    await writeFile(path, bytes);
  },
  async listDir(dir) {
    const entries = await readdir(dir, { withFileTypes: true });
    return entries.filter((e) => e.isFile()).map((e) => e.name);
  },
  async writeText(path, text) {
    await mkdir(dirname(path), { recursive: true });
    await writeFile(path, encodeText(text));
  },
  async exists(path) {
    try {
      await access(path);
      return true;
    } catch {
      return false;
    }
  },
  async remove(path) {
    try {
      await access(path);
    } catch {
      return false;
    }
    await rm(path, { force: true });
    return true;
  },
  async mkdirp(dir) {
    await mkdir(dir, { recursive: true });
  },
};
