/**
 * The data engine across every seam it has: a real DBF on the (in-memory) file system, opened
 * through the IPC data API, paged into the wasm VM, and decoded there into field values.
 *
 * Nothing outside the VM reads a field here. The test builds bytes and asserts on what a FoxPro
 * program printed, which is the only way to know the whole chain agrees.
 */

import { beforeAll, beforeEach, describe, expect, it, vi } from 'vitest';
import { setApi } from '@renderer/api/foxdev';
import { createMemoryApi, type MemoryApi } from '@renderer/api/memoryApi';
import { useSessionStore } from '@renderer/runtime/session';
import { compileProgram } from '@renderer/runtime/vmBridge';
import { requireBytes, type ProgramSource } from '@shared/runtime/programSource';
import { loadFoxVm } from '../../src/wasm/foxvm/loader';
import { buildDbf, buildFpt } from '../data/dbfFixture';

const P = '/proj';

/** The program under test, compiled straight from its text: no project, no documents. */
let program = '';
const source: ProgramSource = {
  getForm: async () => null,
  getMenu: async () => null,
  getProgram: async () => ({ name: 'test', bytes: requireBytes('test', compileProgram(program, 'test')) }),
};

let api: MemoryApi;

beforeAll(async () => {
  await loadFoxVm();
});

beforeEach(() => {
  api = createMemoryApi();
  setApi(api);
  api.binary$.set(
    `${P}/customer.dbf`,
    buildDbf(
      [
        { name: 'CUSTNO', kind: 'C', width: 6 },
        { name: 'NAME', kind: 'C', width: 10 },
        { name: 'AMOUNT', kind: 'N', width: 9, decimals: 2 },
        { name: 'NOTES', kind: 'M', width: 4 },
      ],
      [
        ['A100', 'Acme', '   125.50', '1'],
        ['B200', 'Beta', '80.00', '         2'],
        ['C300', 'Cirrus', '1000.25', '0'],
      ],
    ),
  );
  api.binary$.set(`${P}/customer.fpt`, buildFpt(['first note', 'second note']));
  useSessionStore.getState().cancel();
  useSessionStore.setState({ output: [] });
});

/** Runs a program and returns the lines it printed. */
async function run(src: string): Promise<string[]> {
  program = src;
  await useSessionStore.getState().runProgram(source, `${P}/test.prg`);
  return useSessionStore
    .getState()
    .output.filter((line) => line.kind === 'output' || line.kind === 'error')
    .map((line) => line.text);
}

describe('the data engine', () => {
  /**
   * `SET PATH TO` across the whole chain: the VM works out where else to look and the file
   * service is asked for each in turn.
   *
   * Measured in Visual FoxPro 9 (crates/foxvm/tests/programs/ref_setpath.prg): the default
   * directory is tried first and each entry of the path after it, and a name that carries a
   * folder of its own is not looked for on the path at all.
   */
  it('finds a table through SET PATH, and not one whose name carries a folder', async () => {
    api.binary$.set(`${P}/data/orders.dbf`, buildDbf([{ name: 'ORDERNO', kind: 'C', width: 4 }], [['1001'], ['1002']]));
    expect(
      await run(`SET DEFAULT TO ${P}
SET PATH TO data
USE orders
? ALIAS()
? RECCOUNT()
USE
TRY
	USE nowhere\\orders
CATCH TO oErr
	? TRANSFORM(oErr.ErrorNo)
ENDTRY`),
      // the error names the place the program meant rather than the last place that was tried
    ).toEqual(['ORDERS', '         2', "Error 1 in TEST line 8: File '/proj\\nowhere\\orders.dbf' does not exist. (handled by the program)", '1']);
  });

  it('opens a table and reports on it', async () => {
    expect(
      await run(`USE ${P}/customer.dbf
? ALIAS()
? RECCOUNT()
? FCOUNT()
? DBF()`),
    ).toEqual(['CUSTOMER', '         3', '         4', `${P}/customer.dbf`]);
  });

  it('reads fields of the record it is sitting on', async () => {
    expect(
      await run(`USE ${P}/customer.dbf
? "[" + custno + "]"
? ALLTRIM(name)
? amount
? notes`),
    ).toEqual(['[A100  ]', 'Acme', '   125.50', 'first note']);
  });

  it('moves the pointer with GO and SKIP', async () => {
    expect(
      await run(`USE ${P}/customer.dbf
GO 3
? ALLTRIM(name)
SKIP -1
? RECNO()
GO BOTTOM
? EOF()
SKIP
? EOF()
? RECNO()
GO TOP
? ALLTRIM(customer.name)`),
    ).toEqual(['Cirrus', '         2', '.F.', '.T.', '         4', 'Acme']);
  });

  it('reads a table larger than one page', async () => {
    // 200 records is past PAGE_RECORDS, so walking to the end costs several reads
    const rows = Array.from({ length: 200 }, (_, i) => [`R${i + 1}`]);
    api.binary$.set(`${P}/many.dbf`, buildDbf([{ name: 'CODE', kind: 'C', width: 6 }], rows));
    expect(
      await run(`USE ${P}/many.dbf
LOCAL n, last
n = 0
DO WHILE NOT EOF()
  n = n + 1
  last = ALLTRIM(code)
  SKIP
ENDDO
? n
? last`),
    ).toEqual(['       200', 'R200']);
  });

  it('closes the table when the program says so', async () => {
    // a work area with no table in it is neither past the end nor before the start - measured
    expect(
      await run(`USE ${P}/customer.dbf
USE
? USED()
? EOF()`),
    ).toEqual(['.F.', '.F.']);
  });

  it('reports a table that is not there as a file error the program can see', async () => {
    program = `USE ${P}/nope.dbf`;
    const finished = useSessionStore.getState().runProgram(source, 'test');
    await vi.waitFor(() => expect(useSessionStore.getState().errorReport).not.toBeNull());

    const report = useSessionStore.getState().errorReport;
    expect(report?.error.code).toBe(1);
    expect(report?.error.message).toContain('nope.dbf');
    report?.resolve('cancel');
    await finished;
  });
});
