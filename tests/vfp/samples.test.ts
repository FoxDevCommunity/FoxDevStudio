/**
 * Every sample Visual FoxPro ships with, read and compiled the way the IDE reads and compiles it.
 *
 * The Solution tests drive one sample end to end. This one is broad rather than deep: every
 * `.prg` in every sample is compiled, every `.scx` and `.vcx` is imported and the source of
 * every method in it compiled, and every `.mnx` is imported. It is the widest body of real
 * FoxPro on the machine, so it is what says whether the language this runtime reads is the
 * language people wrote.
 *
 * What it still cannot read is listed in `samples-known.txt`, one line each. The test fails on
 * anything that is not in that list, so the list only ever shrinks; run with
 * `SAMPLES_REPORT=<file>` to write the current list out again.
 *
 * The Foundation Classes this product *ships* are read the same way, and they are not optional:
 * they are on screen the moment anyone opens a form built on them, so a fault in one is a fault
 * in the product rather than in a sample. That half runs whether or not Visual FoxPro is
 * installed; the rest skips itself when it is not.
 */

import { existsSync, readFileSync, readdirSync, statSync, writeFileSync } from 'node:fs';
import { beforeAll, describe, expect, it } from 'vitest';
import { basename, dirname } from '@shared/paths';
import { importClassLibrary, importFormTable } from '@shared/vfp/importForm';
import { importMenuTable } from '@shared/vfp/importMenu';
import { formMethodSources } from '@shared/runtime/programSource';
import { setApi } from '@renderer/api/foxdev';
import { createMemoryApi } from '@renderer/api/memoryApi';
import { loadClassLibraries } from '@renderer/vfp/classLibraries';
import type { DbfReadResult, DbfTableData } from '@shared/vfp/dbfTypes';
import { loadFoxVm, type FoxVmModule } from '../../src/wasm/foxvm/loader';

const SAMPLES = 'C:/Program Files (x86)/Microsoft Visual FoxPro 9/Samples';
const KNOWN = 'tests/vfp/samples-known.txt';
/** Where a library named by a form may be: beside it, and among the bundled Foundation Classes. */
const FFC = 'resources/ffc';
const installed = existsSync(`${SAMPLES}/Solution/main.prg`);

/** A diagnostic as the compiler reports it. */
interface Diagnostic {
  line: number;
  message: string;
  severity: string;
}

let vm: FoxVmModule;

// for both describes: the Foundation Classes test needs the VM and the api on every
// machine, whether or not Visual FoxPro is installed on it
beforeAll(async () => {
  vm = await loadFoxVm();
  // the importer looks for the libraries a form names through the api; here it looks on disk
  const api = createMemoryApi();
  api.files.exists = async (path: string) => existsSync(path);
  api.files.listDir = async (dir: string) => readdirSync(dir);
  setApi(api);
});

/** Every file under `dir` whose extension is one of `want`, in a stable order. */
function filesUnder(dir: string, want: Set<string>): string[] {
  const out: string[] = [];
  const walk = (at: string): void => {
    for (const entry of readdirSync(at).sort()) {
      const path = `${at}/${entry}`;
      if (statSync(path).isDirectory()) walk(path);
      else if (want.has(entry.slice(entry.lastIndexOf('.') + 1).toLowerCase())) out.push(path);
    }
  };
  walk(dir);
  return out;
}

/** A design file and the memo beside it, whatever case either was written in. */
function table(path: string): DbfTableData | string {
  const stem = basename(path).replace(/\.[^.]+$/, '').toLowerCase();
  const ext = path.slice(path.lastIndexOf('.') + 1).toLowerCase();
  const wanted = { scx: ['sct'], vcx: ['vct'], mnx: ['mnt'], pjx: ['pjt'] }[ext] ?? ['fpt'];
  const beside = readdirSync(dirname(path)).find((n) => {
    const dot = n.lastIndexOf('.');
    return dot > 0 && n.slice(0, dot).toLowerCase() === stem && wanted.includes(n.slice(dot + 1).toLowerCase());
  });
  const memo = beside ? new Uint8Array(readFileSync(`${dirname(path)}/${beside}`)) : new Uint8Array();
  const result = vm.read_dbf(new Uint8Array(readFileSync(path)), memo) as DbfReadResult;
  return result.ok ? result : result.error;
}

/** A library named by a form, read from wherever `loadClassLibraries` found it. */
async function readTable(path: string): Promise<DbfTableData> {
  const read = table(path);
  if (typeof read === 'string') throw new Error(`${path}: ${read}`);
  return read;
}

/** The errors compiling one piece of source, with where they came from in front. */
function errorsIn(source: string, where: string, kind: 'program' | 'method'): string[] {
  const { diagnostics } = vm.check(source, kind) as { diagnostics: Diagnostic[] };
  return diagnostics.filter((d) => d.severity === 'error').map((d) => `${where}:${d.line}: ${d.message}`);
}

/** Everything the samples say that this runtime cannot read yet, one line each. */
function known(): Set<string> {
  return new Set(
    readFileSync(KNOWN, 'utf8')
      .split('\n')
      .map((l) => l.trim())
      .filter((l) => l !== '' && !l.startsWith('#')),
  );
}

/**
 * Checks a run against the list. Anything not on it is a failure; a run asked for a report
 * writes what it found so the list can be made again.
 */
function against(errors: string[], report: string | undefined, part: string): string[] {
  if (report) writeFileSync(`${report}.${part}`, errors.join('\n'));
  const allowed = known();
  return errors.filter((e) => !allowed.has(e));
}

describe.skipIf(!installed)('the samples Visual FoxPro ships with', () => {
  it('compiles every program in them', () => {
    const errors: string[] = [];
    for (const path of filesUnder(SAMPLES, new Set(['prg', 'mpr', 'qpr']))) {
      const source = readFileSync(path, 'latin1');
      errors.push(...errorsIn(source, path.slice(SAMPLES.length + 1), 'program'));
    }
    expect(against(errors, process.env['SAMPLES_REPORT'], 'programs')).toEqual([]);
  });

  it('imports every form and class library, and compiles what is written in them', async () => {
    const errors: string[] = [];
    for (const path of filesUnder(SAMPLES, new Set(['scx', 'vcx']))) {
      const where = path.slice(SAMPLES.length + 1);
      const read = table(path);
      if (typeof read === 'string') {
        errors.push(`${where}: ${read}`);
        continue;
      }
      const name = basename(path).replace(/\.[^.]+$/, '');
      // the classes a form is built from are found the way the IDE finds them: beside the file,
      // and among the Foundation Classes
      const { libraries } = await loadClassLibraries(read, dirname(path), readTable, [dirname(path), FFC]);
      const definitions = path.toLowerCase().endsWith('.vcx')
        ? importClassLibrary(read, name, libraries).map((c) => c.imported)
        : [importFormTable(read, name, libraries)];
      const seen = new Set<string>();
      for (const imported of definitions) {
        for (const warning of imported.warnings) errors.push(`${where}: ${warning.message}`);
        for (const method of formMethodSources(imported.doc)) {
          // a class library defines the same method on every class built from another, so the
          // same source is reached more than once
          const at = `${where} ${method.objectPath}.${method.event}`;
          if (seen.has(at)) continue;
          seen.add(at);
          errors.push(...errorsIn(method.source, at, 'method'));
        }
      }
    }
    expect(against(errors, process.env['SAMPLES_REPORT'], 'forms')).toEqual([]);
  });

  it('imports every menu', () => {
    const errors: string[] = [];
    for (const path of filesUnder(SAMPLES, new Set(['mnx']))) {
      const where = path.slice(SAMPLES.length + 1);
      const read = table(path);
      if (typeof read === 'string') {
        errors.push(`${where}: ${read}`);
        continue;
      }
      const imported = importMenuTable(read, basename(path).replace(/\.[^.]+$/, ''));
      for (const warning of imported.warnings) errors.push(`${where}: ${warning.message}`);
    }
    expect(against(errors, process.env['SAMPLES_REPORT'], 'menus')).toEqual([]);
  });
});

/**
 * The Foundation Classes as we ship them.
 *
 * Every class library in `resources/ffc` is imported and every method in it compiled. These are
 * not samples: a form built on them puts them on screen, so anything that will not compile here
 * is a fault a user meets rather than one they read about. Two got out this way - a DIMENSION
 * written with parentheses, and a control whose Init declines - because nothing was reading
 * them.
 */
describe('the Foundation Classes this product ships', () => {
  it('imports every class library and compiles what is written in them', async () => {
    const errors: string[] = [];
    for (const path of filesUnder(FFC, new Set(['vcx']))) {
      const where = path.slice(FFC.length + 1);
      const read = table(path);
      if (typeof read === 'string') {
        errors.push(`${where}: ${read}`);
        continue;
      }
      const name = basename(path).replace(/\.[^.]+$/, '');
      const { libraries } = await loadClassLibraries(read, dirname(path), readTable, [FFC]);
      const seen = new Set<string>();
      for (const { imported } of importClassLibrary(read, name, libraries)) {
        for (const warning of imported.warnings) errors.push(`${where}: ${warning.message}`);
        for (const method of formMethodSources(imported.doc)) {
          const at = `${where} ${method.objectPath}.${method.event}`;
          if (seen.has(at)) continue;
          seen.add(at);
          errors.push(...errorsIn(method.source, at, 'method'));
        }
      }
    }
    expect(against(errors, process.env['SAMPLES_REPORT'], 'ffc')).toEqual([]);
  });
});
