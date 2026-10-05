/**
 * `SET LIBRARY TO` against two libraries whose source we can read.
 *
 * `hello.c` and `reverse.c` are Microsoft's own API samples, in `Samples\API` beside Visual
 * FoxPro. `scripts/build-fllhost.mjs` builds them into `tests/fll/build` with the same toolchain
 * that builds the host, so what is asserted here is what those two files say they do: `hello`
 * calls `_PutStr("\nHello, World!\n")` and sets no return value, `reverse` takes one character
 * parameter, allocates a handle, fills it backwards and hands it to `_RetChar`. Between them
 * they cover the whole path - the parameter block, the memory handles, the output stream and
 * the return value - in a library nobody has to take on trust.
 *
 * Every expectation below was read out of vfp9.exe with the same two files loaded.
 *
 * The file skips itself when there is no host to run: no Windows, no Visual C++ to have built
 * the host with, or a checkout where it has not been built.
 */

import { existsSync } from 'node:fs';
import { resolve } from 'node:path';
import { beforeAll, describe, expect, it } from 'vitest';
import { createMemoryApi } from '@renderer/api/memoryApi';
import { setApi } from '@renderer/api/foxdev';
import { useSessionStore } from '@renderer/runtime/session';
import { compileProgram } from '@renderer/runtime/vmBridge';
import { requireBytes } from '@shared/runtime/programSource';
import type { LibraryHost } from '@shared/runtime/libraryHost';
import { loadFoxVm } from '../../src/wasm/foxvm/loader';

const HOST = resolve('resources/native/win32/fllhost.exe');
const HELLO = resolve('tests/fll/build/hello.fll');
const REVERSE = resolve('tests/fll/build/reverse.fll');
const REFPARM = resolve('tests/fll/build/refparm.fll');
const have = existsSync(HOST) && existsSync(HELLO) && existsSync(REVERSE) && existsSync(REFPARM);

/**
 * How `SET("LIBRARY")` names a path. Measured in Visual FoxPro 9: capitals, and quoted when the
 * path holds anything a plain one would not - a hyphen and a space both make it quote.
 */
const named = (path: string): string => {
  const upper = path.toUpperCase();
  return /^[A-Z0-9\\/:._]*$/.test(upper) ? upper : `"${upper}"`;
};

const settle = (): Promise<unknown> => new Promise((r) => setTimeout(r, 0));
const deadline = (ms: number): Promise<unknown> => new Promise((r) => setTimeout(r, ms));

beforeAll(async () => {
  if (have) await loadFoxVm();
}, 120_000);

/** Runs a program and answers with every line it printed. `library` replaces the real host. */
async function run(text: string, library?: LibraryHost): Promise<string[]> {
  const api = createMemoryApi();
  if (library) api.library = library;
  setApi(api);
  useSessionStore.getState().cancel();
  useSessionStore.setState({ output: [] });
  await Promise.race([
    deadline(8000),
    useSessionStore.getState().execute(
      {
        async getForm() {
          return null;
        },
        async getProgram(name) {
          return { name: 'fllsample', bytes: requireBytes(name, compileProgram(text, 'fllsample')) };
        },
        async getMenu() {
          return null;
        },
      },
      'DO fllsample',
    ),
  ]);
  for (let i = 0; i < 200 && useSessionStore.getState().status === 'running'; i++) await settle();
  useSessionStore.getState().cancel();
  return useSessionStore.getState().output.map((l) => l.text);
}

describe.skipIf(!have)('a library built from the API samples', () => {
  it('prints what hello.c prints and answers what a routine with no RETURN answers', async () => {
    const said = await run(`SET LIBRARY TO ${HELLO}
? "before"
luAnswer = Hello()
? "after " + TYPE("luAnswer") + " " + TRANSFORM(luAnswer)`);
    // measured: _PutStr's leading newline ends the line the program was on, the greeting goes
    // on the next, and its trailing newline leaves a blank one
    expect(said).toEqual(['DO fllsample', 'before', 'Hello, World!', '', 'after L .T.']);
  }, 60_000);

  it('hands reverse.c a string parameter and takes back the one it made', async () => {
    const said = await run(`SET LIBRARY TO ${REVERSE}
? "[" + Reverse("Hello, World") + "]"
? "[" + Reverse("") + "]"
? TYPE([Reverse("abc")])`);
    expect(said.slice(1)).toEqual(['[dlroW ,olleH]', '[]', 'C']);
  }, 60_000);

  it('keeps what is loaded when ADDITIVE says to, and lets it go when it does not', async () => {
    // measured: SET("LIBRARY") answers the full paths in capitals, in the order they were
    // loaded, ", " between them
    const said = await run(`SET LIBRARY TO ${HELLO}
SET LIBRARY TO ${REVERSE} ADDITIVE
luHello = Hello()
? "hello: " + TYPE("luHello") + " reverse: " + TYPE([Reverse("ab")])
? SET("LIBRARY")
SET LIBRARY TO ${HELLO}
? "replaced: " + TYPE([Reverse("ab")])
? SET("LIBRARY")`);
    expect(said.slice(-4)).toEqual([
      'hello: L reverse: C',
      `${named(HELLO)}, ${named(REVERSE)}`,
      'replaced: U',
      named(HELLO),
    ]);
  }, 60_000);

  it('lets everything go when nothing follows TO', async () => {
    const said = await run(`SET LIBRARY TO ${REVERSE}
SET LIBRARY TO
? "[" + SET("LIBRARY") + "]"
? TYPE([Reverse("ab")])`);
    expect(said.slice(1)).toEqual(['[]', 'U']);
  }, 60_000);

  it('loading the same library twice names it once', async () => {
    const said = await run(`SET LIBRARY TO ${REVERSE}
SET LIBRARY TO ${REVERSE} ADDITIVE
? SET("LIBRARY")`);
    expect(said.at(-1)).toBe(named(REVERSE));
  }, 60_000);

  it('hands a variable passed with @ to a parameter declared R, and keeps what it stored', async () => {
    // refparm.c is ours, not Microsoft's: neither sample takes a reference. Measured in Visual
    // FoxPro 9 with the same build of it; CodeMine's cmRegGetValue ("I,C,R") is the real case.
    const said = await run(`SET LIBRARY TO ${REFPARM}
ON ERROR ? "err", ERROR()
cVar = "old"
x = SWAPREF("new", @cVar)
? "swap", x, cVar
nVar = 5
? "bump", BUMPREF(@nVar), nVar, VARTYPE(nVar)
uVar = "text"
? "set", SETREF(@uVar), uVar, VARTYPE(uVar)
cVar = "kept"
x = SWAPREF("new", cVar)
? "byval", cVar`);
    const printed = said.slice(1).filter((l) => !l.startsWith('Error '));
    expect(printed.map((l) => l.replace(/\s+/g, ' ').trim())).toEqual(['swap old new', 'bump .T. 6 N', 'set .T. 42 N', 'err 9', 'byval kept']);
  }, 60_000);

  it('hands a number to a parameter declared I the way the product does, past the top of a long too', async () => {
    // measured with refparm.fll's INTOF: a registry root is written 2147483650 for
    // HKEY_LOCAL_MACHINE, and the library has to be handed 0x80000002 for it
    const said = await run(`SET LIBRARY TO ${REFPARM}
? "a", INTOF(5), INTOF(-7), INTOF(2147483647)
? "b", INTOF(2147483648), INTOF(2147483650), INTOF(4294967295)
? "c", INTOF(3.7), INTOF(-3.7)`);
    expect(said.slice(1).map((l) => l.replace(/\s+/g, ' ').trim())).toEqual(['a 5 -7 2147483647', 'b -2147483648 -2147483646 -1', 'c 3 -3']);
  }, 60_000);

  it('a name with no extension is a .fll', async () => {
    const said = await run(`SET LIBRARY TO ${REVERSE.replace(/\.fll$/, '')}
? "[" + Reverse("ab") + "]"`);
    expect(said.at(-1)).toBe('[ba]');
  }, 60_000);
});

describe.skipIf(!have)('a library that cannot be loaded', () => {
  // Measured in Visual FoxPro 9: `SET LIBRARY TO nosuchthing.fll` raises 1726, "API library is
  // not found." What follows the colon here is this runtime's own, because Windows answers the
  // same number whether the .fll is missing or something it needs is, and a person reading it
  // needs to know which.
  it('says so as a FoxPro error rather than taking the runtime down with it', async () => {
    const said = await run(`SET LIBRARY TO ${HELLO.replace('hello', 'nosuchthing')}
? "the program carried on"`);
    expect(said.at(-1)).toMatch(/^Error 1726 in FLLSAMPLE line \d+: API library is not found: /);
  }, 60_000);

  it('is refused the same way when the file is not a FoxPro library at all', async () => {
    const said = await run(`SET LIBRARY TO ${HOST}
? "the program carried on"`);
    expect(said.at(-1)).toMatch(/does not export @DispatchAPI@4/);
  }, 60_000);

  it('answers 1726 where there is no host at all, and the program carries on', async () => {
    // an unpackaged checkout, or anything that is not Windows: the runtime says the library is
    // not found, which is a thing a program can handle, and nothing else changes
    const none: LibraryHost = {
      available: () => false,
      load() {
        throw new Error('there is no host here');
      },
      call() {
        throw new Error('there is no host here');
      },
      unload() {},
      releaseAll() {},
    };
    const said = await run(
      `ON ERROR ? "caught " + TRANSFORM(ERROR())
SET LIBRARY TO ${HELLO}
? "still running"`,
      none,
    );
    // the runtime logs every error it raises, handled or not, so the report of it stands
    // above the two lines the program itself printed
    expect(said.slice(-2)).toEqual(['caught 1726', 'still running']);
  }, 60_000);
});

describe.skipIf(!have)('a library given something it did not ask for', () => {
  it('is called with what the program passed and the host stays up', async () => {
    // reverse.c reads p[0] and nothing else. Too few parameters, too many, and one of the
    // wrong type all have to come back as an answer or an error - never as a dead host - and
    // the library has to still work afterwards.
    const said = await run(`SET LIBRARY TO ${REVERSE}
? "none: " + TYPE([Reverse()])
? "number: " + TYPE([Reverse(42)])
? "many: " + TYPE([Reverse("ab", "cd", 1, 2, 3)])
? "still here: [" + Reverse("ab") + "]"`);
    expect(said.at(-1)).toBe('still here: [ba]');
  }, 60_000);
});
