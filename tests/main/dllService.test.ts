/**
 * `DECLARE ... DLL`, through the library it actually calls.
 *
 * This is the one part of the runtime that leaves the process, so it is tested against real
 * Windows libraries rather than a stand-in: a call that works here works for a program. It
 * checks itself off Windows, where there is nothing to call into.
 */

import { describe, expect, it } from 'vitest';
import { createDllService } from '@main/services/dllService';

const service = createDllService();
const windows = process.platform === 'win32' && service.available();

describe('calling a native library', () => {
  it('says whether it can call one at all', () => {
    expect(typeof service.available()).toBe('boolean');
  });
});

describe.skipIf(!windows)('on Windows', () => {
  it('calls a function that takes nothing and returns a number', () => {
    const result = service.call({
      library: 'kernel32',
      function: 'GetTickCount',
      returns: 'INTEGER',
      params: [],
      byRef: [],
      args: [],
    });
    expect(typeof result.value).toBe('number');
    expect(Number(result.value)).toBeGreaterThan(0);
  });

  it('fills a string the program passed by reference', () => {
    // the shape every VFP program uses to read an .ini file, and the reason STRING @ exists
    const result = service.call({
      library: 'Win32API',
      function: 'GetPrivateProfileString',
      returns: 'INTEGER',
      params: ['STRING', 'STRING', 'STRING', 'STRING', 'INTEGER', 'STRING'],
      byRef: [false, false, false, true, false, false],
      args: ['Section', 'Key', 'the default', ' '.repeat(64), 64, 'C:/no/such/file.ini'],
    });
    // The file is not there, so the library answers with the default it was given - and the
    // buffer comes back whole, at the length it was passed, with a NUL after the text. Measured
    // in the product: a 260-character buffer filled by GetSystemDirectoryA still answers LEN 260
    // and holds its first CHR(0) at 20, which is why every VFP program trims it itself.
    expect(result.value).toBe('the default'.length);
    expect(String(result.written[3])).toHaveLength(64);
    expect(String(result.written[3]).split(String.fromCharCode(0))[0]).toBe('the default');
  });

  it('finds a function whose real name ends in A, as VFP does', () => {
    // GetWindowsDirectory is exported as GetWindowsDirectoryA; a VFP program declares neither
    const result = service.call({
      library: 'kernel32',
      function: 'GetWindowsDirectory',
      returns: 'INTEGER',
      params: ['STRING', 'INTEGER'],
      byRef: [true, false],
      args: [' '.repeat(260), 260],
    });
    expect(Number(result.value)).toBeGreaterThan(0);
    expect(String(result.written[0])).toMatch(/^[A-Za-z]:\\/);
  });

  it('writes back a number passed by reference', () => {
    // GetComputerNameA fills a buffer and updates the size it was given
    const size = 128;
    const result = service.call({
      library: 'kernel32',
      function: 'GetComputerName',
      returns: 'INTEGER',
      params: ['STRING', 'INTEGER'],
      byRef: [true, true],
      args: [' '.repeat(size), size],
    });
    expect(Number(result.value)).not.toBe(0);
    expect(String(result.written[0]).length).toBeGreaterThan(0);
    // the size written back is the name's length, and the buffer is still the whole 128
    expect(String(result.written[0])).toHaveLength(size);
    expect(Number(result.written[1])).toBe(String(result.written[0]).split(String.fromCharCode(0))[0]!.length);
  });

  it('reports a library that is not there rather than crashing', () => {
    expect(() =>
      service.call({ library: 'no_such_library_here', function: 'Nope', returns: 'INTEGER', params: [], byRef: [], args: [] }),
    ).toThrow();
  });
});
