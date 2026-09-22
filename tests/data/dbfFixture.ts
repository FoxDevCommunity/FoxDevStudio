/**
 * Visual FoxPro tables built byte for byte, for the tests that need a real one.
 *
 * Nothing here uses the reader it is testing against: the bytes are laid out from the format
 * description, so a test that passes says the reader agrees with the format rather than with
 * itself.
 */

export interface Field {
  name: string;
  kind: 'C' | 'N' | 'L' | 'D' | 'M';
  width: number;
  decimals?: number;
}

/** A `.dbf`. `deleted` holds the 1-based numbers of the records to mark deleted. */
export function buildDbf(fields: Field[], records: string[][], deleted: number[] = []): Uint8Array {
  const recordLen = 1 + fields.reduce((n, f) => n + f.width, 0);
  const headerLen = 32 + fields.length * 32 + 1;
  const out: number[] = new Array<number>(headerLen).fill(0);
  out[0] = 0x30; // Visual FoxPro
  writeU32(out, 4, records.length);
  writeU16(out, 8, headerLen);
  writeU16(out, 10, recordLen);

  let offset = 1;
  fields.forEach((field, i) => {
    const at = 32 + i * 32;
    for (let c = 0; c < field.name.length; c++) out[at + c] = field.name.charCodeAt(c);
    out[at + 11] = field.kind.charCodeAt(0);
    writeU32(out, at + 12, offset);
    out[at + 16] = field.width;
    out[at + 17] = field.decimals ?? 0;
    offset += field.width;
  });
  out[headerLen - 1] = 0x0d; // field terminator

  records.forEach((record, i) => {
    out.push(deleted.includes(i + 1) ? 0x2a : 0x20);
    fields.forEach((field, f) => {
      const text = record[f] ?? '';
      if (field.kind === 'M') {
        // a Visual FoxPro memo field holds a 4-byte block number, not the digits of one
        const at = out.length;
        out.push(0, 0, 0, 0);
        writeU32(out, at, Number(text) || 0);
        return;
      }
      // character fields pad on the right, numeric ones on the left
      const padded = field.kind === 'N' ? text.padStart(field.width) : text.padEnd(field.width);
      for (let c = 0; c < field.width; c++) out.push(padded.charCodeAt(c) & 0xff);
    });
  });
  out.push(0x1a); // end of file
  return Uint8Array.from(out);
}

/** A `.fpt` holding one text block per entry, starting at block 1. */
export function buildFpt(entries: string[], blockSize = 64): Uint8Array {
  const out: number[] = new Array<number>(blockSize).fill(0);
  writeU32BE(out, 0, entries.length + 1); // next free block
  writeU16BE(out, 6, blockSize);
  for (const text of entries) {
    const block: number[] = new Array<number>(blockSize).fill(0);
    writeU32BE(block, 0, 1); // a text memo
    writeU32BE(block, 4, text.length);
    for (let c = 0; c < text.length; c++) block[8 + c] = text.charCodeAt(c) & 0xff;
    out.push(...block);
  }
  return Uint8Array.from(out);
}

function writeU16(out: number[], at: number, value: number) {
  out[at] = value & 0xff;
  out[at + 1] = (value >> 8) & 0xff;
}

function writeU32(out: number[], at: number, value: number) {
  for (let i = 0; i < 4; i++) out[at + i] = (value >> (i * 8)) & 0xff;
}

// Memo files are big-endian, unlike everything else in a DBF.
function writeU16BE(out: number[], at: number, value: number) {
  out[at] = (value >> 8) & 0xff;
  out[at + 1] = value & 0xff;
}

function writeU32BE(out: number[], at: number, value: number) {
  for (let i = 0; i < 4; i++) out[at + i] = (value >> ((3 - i) * 8)) & 0xff;
}
