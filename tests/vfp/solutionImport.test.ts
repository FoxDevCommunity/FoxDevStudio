/**
 * The Solution sample through the path the IDE actually uses: find the class libraries a form
 * refers to, load them, and import the form against them.
 *
 * The unit tests pass the library in by hand, which proves the merge and nothing about whether
 * the library is ever found. This drives the same code File > Import VFP Project drives, against
 * a real Visual FoxPro installation, and asserts on the document that would be written to disk.
 *
 * It skips itself when Visual FoxPro is not installed.
 */

import { existsSync, readFileSync, readdirSync } from 'node:fs';
import { beforeAll, describe, expect, it } from 'vitest';
import { basename, dirname, join } from '@shared/paths';
import { importFormFile } from '@shared/vfp/importForm';
import type { DbfReadResult, DbfTableData } from '@shared/vfp/dbfTypes';
import type { ControlNode, FormNode } from '@shared/form/schema';
import { setApi } from '@renderer/api/foxdev';
import { createMemoryApi } from '@renderer/api/memoryApi';
import { loadClassLibraries } from '@renderer/vfp/classLibraries';
import { loadFoxVm, type FoxVmModule } from '../../src/wasm/foxvm/loader';

const DIR = 'C:/Program Files (x86)/Microsoft Visual FoxPro 9/Samples/Solution';
const installed = existsSync(`${DIR}/solution.scx`);

let vm: FoxVmModule;

beforeAll(async () => {
  if (!installed) return;
  vm = await loadFoxVm();
  // the importer reads through the api; here it reads the real installation
  const api = createMemoryApi();
  api.files.readBytes = async (path: string) => new Uint8Array(readFileSync(path));
  api.files.listDir = async (dir: string) => readdirSync(dir);
  api.files.exists = async (path: string) => existsSync(path);
  setApi(api);
});

/** What `convertItem` does to turn a path into a table: the DBF plus its memo, whatever its case. */
async function readTable(path: string): Promise<DbfTableData> {
  const bytes = new Uint8Array(readFileSync(path));
  const stem = basename(path).replace(/\.[^.]+$/, '').toLowerCase();
  // each design format has its own memo extension, and solution.scx and solution.vcx share a
  // stem: taking the wrong one reads every memo field as rubbish
  const ext = path.slice(path.lastIndexOf('.') + 1).toLowerCase();
  const wanted = { scx: ['sct'], vcx: ['vct'], mnx: ['mnt'], pjx: ['pjt'] }[ext] ?? ['fpt'];
  const match = readdirSync(dirname(path)).find((n) => {
    const dot = n.lastIndexOf('.');
    return dot > 0 && n.slice(0, dot).toLowerCase() === stem && wanted.includes(n.slice(dot + 1).toLowerCase());
  });
  const memo = match ? new Uint8Array(readFileSync(join(dirname(path), match))) : undefined;
  const result = vm.read_dbf(bytes, memo) as DbfReadResult;
  if (!result.ok) throw new Error(result.error);
  return result;
}

function nodes(form: FormNode): (FormNode | ControlNode)[] {
  const out: (FormNode | ControlNode)[] = [form];
  const walk = (children: ControlNode[] | undefined) => {
    for (const c of children ?? []) {
      out.push(c);
      walk(c.children);
    }
  };
  walk(form.children);
  return out;
}

const has = (o: FormNode | ControlNode, kind: 'props' | 'methods', name: string) =>
  Object.keys(o[kind]).some((k) => k.toLowerCase() === name.toLowerCase());

describe.skipIf(!installed)('importing the Solution sample the way the IDE does', () => {
  it('finds solution.vcx and brings its custom members into the form', async () => {
    const table = await readTable(`${DIR}/solution.scx`);
    const { libraries, missing } = await loadClassLibraries(table, DIR, readTable, []);

    expect(missing).toEqual([]);
    expect(libraries.map((l) => l.name.toLowerCase())).toContain('solution.vcx');

    const forms = importFormFile(table, 'solution', libraries);
    const all = forms.flatMap((f) => nodes(f.imported.doc.form));

    for (const name of ['lCalledBySolution', 'centerform', 'fixedformborder', 'cOldPath', 'cDirectory', 'cPoint']) {
      expect(
        all.some((o) => has(o, 'props', name)),
        `${name} should be a property of some object in the imported form`,
      ).toBe(true);
    }
    for (const name of ['getdirectory', 'addtopath']) {
      expect(
        all.some((o) => has(o, 'methods', name)),
        `${name} should be a method of some object in the imported form`,
      ).toBe(true);
    }
  });
});
