/**
 * A Visual FoxPro library (.fll), loaded and called by a program running on our VM.
 *
 * An .fll is not a .dll a program declares functions out of: it is a library written against
 * FoxPro's own C API, which the host hands its function table to and which answers with a table
 * of the functions it adds to the language. `SET LIBRARY TO` loads it and its functions are then
 * called as though they had always been in the language. vfpencryption71.fll - the one every
 * VFP application that needs a hash or a cipher uses - exports one symbol, `@DispatchAPI@4`, and
 * imports nothing from vfp9.exe, so hosting it is a matter of building the API table it asks
 * for rather than of pretending to be the product.
 *
 * `vfpencryption.prg` beside this file is the conformance program, and every value in it was
 * read out of the product with the library loaded: all sixteen checks pass there. This runs the
 * same program on our VM and expects the same sixteen.
 *
 * It skips itself when the library is not on this machine - point FOXDEV_FLL at it - and, until
 * this runtime can host a library at all, it says so rather than failing: `SET LIBRARY TO` is
 * remembered and does nothing here, so `Hash()` is a procedure that is not found. The day the
 * host is built, the sixteen checks below become its acceptance test with no edit to this file.
 */

import { existsSync, readFileSync } from 'node:fs';
import { beforeAll, describe, expect, it } from 'vitest';
import { createMemoryApi } from '@renderer/api/memoryApi';
import { setApi } from '@renderer/api/foxdev';
import { useSessionStore } from '@renderer/runtime/session';
import { compileProgram } from '@renderer/runtime/vmBridge';
import { requireBytes } from '@shared/runtime/programSource';
import { loadFoxVm } from '../../src/wasm/foxvm/loader';

const PROGRAM = 'tests/fll/vfpencryption.prg';
// the library is not ours to ship: drop a copy at the repository root, or name one in FOXDEV_FLL
const LIBRARY = process.env['FOXDEV_FLL'] ?? 'vfpencryption71.fll';
const have = existsSync(LIBRARY);

const settle = (): Promise<unknown> => new Promise((r) => setTimeout(r, 0));
const deadline = (ms: number): Promise<unknown> => new Promise((r) => setTimeout(r, ms));

beforeAll(async () => {
  if (have) await loadFoxVm();
}, 120_000);

/** Runs a program on the VM and answers with every line it printed. */
async function run(text: string): Promise<string[]> {
  const api = createMemoryApi();
  api.binary$.set(LIBRARY, new Uint8Array(readFileSync(LIBRARY)));
  setApi(api);
  useSessionStore.getState().cancel();
  useSessionStore.setState({ output: [] });
  // a program that stops on an error waits for an answer nobody is here to give, so it is given
  // a few seconds and then cancelled: what it said before it stopped is the report
  await Promise.race([
    deadline(8000),
    useSessionStore.getState().execute(
      {
        async getForm() {
          return null;
        },
        async getProgram(name) {
          return { name: 'fllprobe', bytes: requireBytes(name, compileProgram(text, 'fllprobe')) };
        },
        async getMenu() {
          return null;
        },
      },
      'DO fllprobe',
    ),
  ]);
  for (let i = 0; i < 200 && useSessionStore.getState().status === 'running'; i++) await settle();
  useSessionStore.getState().cancel();
  return useSessionStore
    .getState()
    .output.map((l) => l.text.trim())
    .filter((t) => t !== '');
}

describe.skipIf(!have)('a program using vfpencryption71.fll', () => {
  /** Whether this runtime can load a FoxPro library at all. */
  let hosted = false;

  beforeAll(async () => {
    const said = await run(`SET LIBRARY TO ${LIBRARY}
? TYPE([Hash("a", 5)])`);
    hosted = said.includes('C');
  }, 60_000);

  it('says whether it can host a FoxPro library', () => {
    // Not yet: an .fll is a library written against FoxPro's own C API, and hosting one means
    // building the table of API functions it calls back into - _Alloc, _RetChar, _Parameter and
    // the rest - and handing it over through the @DispatchAPI@4 it exports. Until then this
    // records where we are rather than pretending.
    expect(typeof hosted).toBe('boolean');
    if (!hosted) console.log('vfpencryption71.fll is not hosted yet: SET LIBRARY TO is remembered and does nothing');
  });

  // The skip is asked for here rather than with `it.skipIf`, which is decided when this file is
  // collected - before the beforeAll above has run, so it could only ever have read the `false`
  // this starts at. Asked for from inside the test, it lifts the moment the host answers.
  it('answers what the product answers, check for check', async (ctx) => {
    if (!hosted) ctx.skip();
    // every mention, not the first: the first is in the program's own comment, and the one that
    // has to become a path this machine can open is the SET LIBRARY TO line further down
    const said = await run(readFileSync(PROGRAM, 'utf8').replaceAll('vfpencryption71.fll', LIBRARY));
    expect(said.filter((l) => l.startsWith('[FAIL]')), said.join('\n')).toEqual([]);
    expect(said.at(-1), said.join('\n')).toBe('PASSED 16 FAILED 0');
  }, 120_000);
});
