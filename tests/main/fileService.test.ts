/**
 * Reading and writing text in the code page Visual FoxPro works in.
 *
 * VFP stores text in the ANSI code page, so a file this IDE writes has to come back the same
 * way when VFP opens it, and a file VFP wrote has to read back as the text it holds rather
 * than as replacement characters.
 */

import { describe, expect, it } from 'vitest';
import { decodeText, encodeText } from '../../src/main/services/fileService';

/** The bytes of `Taquería` as Windows-1252 has them: í is one byte, and not valid UTF-8. */
const ANSI = new Uint8Array([...new TextEncoder().encode('Taquer'), 0xed, 0x61]);

describe('reading a text file', () => {
  it('reads UTF-8 as UTF-8, with or without a byte order mark', () => {
    const bytes = new TextEncoder().encode('Taquería ok');
    expect(decodeText(bytes)).toBe('Taquería ok');
    expect(decodeText(new Uint8Array([0xef, 0xbb, 0xbf, ...bytes]))).toBe('Taquería ok');
  });

  it('reads a file Visual FoxPro wrote in the ANSI code page', () => {
    expect(decodeText(ANSI)).toBe('Taquería');
    expect(decodeText(ANSI)).not.toContain('\u{fffd}');
  });

  it('leaves plain ASCII exactly as it was', () => {
    const bytes = new TextEncoder().encode('USE customer\r\n? RECCOUNT()\r\n');
    expect(decodeText(bytes)).toBe('USE customer\r\n? RECCOUNT()\r\n');
  });
});

describe('writing a text file', () => {
  it('writes the ANSI code page, so Visual FoxPro reads back what it was given', () => {
    expect(Array.from(encodeText('Taquería'))).toEqual(Array.from(ANSI));
    // an em dash and a half are in Windows-1252 too, at 0x97 and 0xbd
    expect(Array.from(encodeText('— ½'))).toEqual([0x97, 0x20, 0xbd]);
  });

  it('reads back byte for byte what it wrote', () => {
    for (const text of ['Taquería', 'plain ascii', '? "€1,50 — ½"\r\n', '']) {
      expect(decodeText(encodeText(text))).toBe(text);
    }
  });

  it('keeps text the code page cannot hold rather than losing it', () => {
    // Windows-1252 has no Japanese, and writing it would put question marks in its place
    const text = '* 日本語';
    expect(decodeText(encodeText(text))).toBe(text);
    expect(Array.from(encodeText(text))).toEqual(Array.from(new TextEncoder().encode(text)));
  });
});
