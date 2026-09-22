/**
 * Naming the ActiveX controls a Visual FoxPro form holds.
 *
 * The class id is inside the control's persisted state, so this reads the `.scx` Microsoft ships
 * rather than a blob I made up: the Solution sample's main form carries a TreeView and an
 * ImageList, and neither says so anywhere else.
 */

import { existsSync, readFileSync } from 'node:fs';
import { beforeAll, describe, expect, it } from 'vitest';
import { importFormTable } from '@shared/vfp/importForm';
import { oleClassId, oleControlClass, oleServerFile } from '@shared/vfp/oleControl';
import type { DbfReadResult, DbfTableData } from '@shared/vfp/dbfTypes';
import type { ControlNode, FormNode } from '@shared/form/schema';
import { loadFoxVm, type FoxVmModule } from '../../src/wasm/foxvm/loader';

const DIR = 'C:/Program Files (x86)/Microsoft Visual FoxPro 9/Samples/Solution';
const installed = existsSync(`${DIR}/solution.scx`);
let vm: FoxVmModule;

beforeAll(async () => {
  if (installed) vm = await loadFoxVm();
});

function table(name: string, memo: string): DbfTableData {
  const dbf = new Uint8Array(readFileSync(`${DIR}/${name}`));
  const memoBytes = existsSync(`${DIR}/${memo}`) ? new Uint8Array(readFileSync(`${DIR}/${memo}`)) : undefined;
  const result = vm.read_dbf(dbf, memoBytes) as DbfReadResult;
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

describe('OLE control classes', () => {
  it('reads the file an OLE2 memo names', () => {
    expect(oleServerFile('OLEObject = C:\\WINNT\\System32\\MSCOMCTL.OCX\n')).toBe('MSCOMCTL.OCX');
    expect(oleServerFile('')).toBe('');
  });

  it('does not name a control it does not recognise', () => {
    expect(oleClassId(new Uint8Array(64))).toBeNull();
  });

  it('knows both class ids the Common Controls have been shipped under', () => {
    expect(oleControlClass('{C741FDB6-2630-11D1-B16A-00C0F0283628}')?.emulated).toBe('TreeView');
    expect(oleControlClass('{c74190b6-8589-11d1-b16a-00c0f0283628}')?.emulated).toBe('TreeView');
  });
});

describe.skipIf(!installed)('the Solution sample', () => {
  it('names the TreeView and the ImageList on its Tree page', () => {
    const libraries = [{ name: 'solution.vcx', table: table('solution.vcx', 'solution.vct') }];
    const { doc } = importFormTable(table('solution.scx', 'solution.sct'), 'solution', libraries);

    const byName = (name: string) => nodes(doc.form).find((n) => n.name.toLowerCase() === name);
    expect(byName('oletree')?.props['OleClass']).toBe('MSComctlLib.TreeCtrl.2');
    expect(byName('oleimages')?.props['OleClass']).toBe('MSComctlLib.ImageListCtrl.2');
  });
});
