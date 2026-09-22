/**
 * The Visual FoxPro Solution sample, imported from the files Microsoft ships.
 *
 * It is the application that opens every other sample, it subclasses a class library, and its
 * forms are built out of custom properties and custom methods - which is exactly the shape that
 * kept failing at run time with "Property ... is not found" and "Unknown member ...". This test
 * reads the real `.scx` and `.vcx` off disk, so it fails when the importer loses something rather
 * than when a fixture I wrote agrees with me.
 *
 * It skips itself when Visual FoxPro is not installed.
 */

import { existsSync, readFileSync } from 'node:fs';
import { beforeAll, describe, expect, it } from 'vitest';
import { importFormTable } from '@shared/vfp/importForm';
import type { DbfReadResult, DbfTableData } from '@shared/vfp/dbfTypes';
import type { ControlNode, FormNode } from '@shared/form/schema';
import { loadFoxVm, type FoxVmModule } from '../../src/wasm/foxvm/loader';

const DIR = 'C:/Program Files (x86)/Microsoft Visual FoxPro 9/Samples/Solution';
const installed = existsSync(`${DIR}/solution.scx`);

let vm: FoxVmModule;

beforeAll(async () => {
  if (installed) vm = await loadFoxVm();
});

/** Reads a DBF-based design file and its memo, the way the IDE does. */
function table(name: string, memo: string): DbfTableData {
  const dbf = new Uint8Array(readFileSync(`${DIR}/${name}`));
  const memoBytes = existsSync(`${DIR}/${memo}`) ? new Uint8Array(readFileSync(`${DIR}/${memo}`)) : undefined;
  const result = vm.read_dbf(dbf, memoBytes) as DbfReadResult;
  if (!result.ok) throw new Error(result.error);
  return result;
}

/** Every node of the form, so a member can be looked for wherever it ended up. */
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

describe.skipIf(!installed)('the Solution sample', () => {
  it('keeps the custom properties and methods its class library declares', () => {
    const libraries = [{ name: 'solution.vcx', table: table('solution.vcx', 'solution.vct') }];
    const { doc } = importFormTable(table('solution.scx', 'solution.sct'), 'solution', libraries);

    const all = nodes(doc.form);
    // c_solutions declares these in its RESERVED3 memo; two of them have no value anywhere else
    const properties = [
      'centerform',
      'fixedformborder',
      'cOldPath',
      'cDirectory',
      'cPoint',
      'cSep',
      'cDate',
      'cCurrency',
      'cTalk',
      'cDeleted',
      'lCalledBySolution',
      'autosetdefault',
    ];
    for (const name of properties) {
      expect(
        all.some((o) => has(o, 'props', name)),
        `${name} should be a property of some object`,
      ).toBe(true);
    }
    for (const name of ['getdirectory', 'addtopath', 'savehelp', 'restorehelp']) {
      expect(
        all.some((o) => has(o, 'methods', name)),
        `${name} should be a method of some object`,
      ).toBe(true);
    }
  });
});
