import { readFileSync } from 'node:fs';
import { beforeAll, describe, expect, it } from 'vitest';
import { readProjectTable, targetPath, toProjectDocument } from '@shared/vfp/importProject';
import type { DbfReadResult, DbfTableData } from '@shared/vfp/dbfTypes';
import { text } from '@shared/vfp/dbfTypes';
import { loadFoxVm } from '../../src/wasm/foxvm/loader';

/** The real Visual FoxPro sample projects, copied in as fixtures by the reader's tests. */
const FIXTURES = 'crates/foxvm/tests/fixtures';
const read = (name: string) => new Uint8Array(readFileSync(`${FIXTURES}/${name}`));

function table(dbf: string, memo?: string): DbfTableData {
  const result = loadFoxVmSync().read_dbf(read(dbf), memo ? read(memo) : undefined) as DbfReadResult;
  if (!result.ok) throw new Error(result.error);
  return result;
}

let loadFoxVmSync: () => Awaited<ReturnType<typeof loadFoxVm>>;

beforeAll(async () => {
  const vm = await loadFoxVm();
  loadFoxVmSync = () => vm;
});

describe('importing a real Visual FoxPro project', () => {
  it('reads formsui.pjx into project items', () => {
    const imported = readProjectTable(table('formsui.pjx', 'formsui.PJT'), 'formsui.pjx');

    expect(imported.name).toBe('formsui');
    // five .prg programs plus the readme text file; the header row carries no item
    expect(imported.items.filter((i) => i.kind === 'program')).toHaveLength(5);
    expect(imported.items.every((i) => i.sourcePath.length > 0)).toBe(true);
    expect(imported.items.filter((i) => i.main)).toHaveLength(1);
    expect(imported.items.find((i) => i.main)!.sourcePath).toMatch(/\.prg$/i);
  });

  it('does not treat the project header row as an item', () => {
    const t = table('formsui.pjx', 'formsui.PJT');
    const headerRows = t.records.filter((r) => text(t, r, 'TYPE').toUpperCase() === 'H');
    expect(headerRows).toHaveLength(1);

    const imported = readProjectTable(t, 'formsui.pjx');
    expect(imported.items.some((i) => i.sourcePath.toLowerCase().endsWith('.pjx'))).toBe(false);
  });

  it('produces a FoxDev project document that points at the programs', () => {
    const imported = readProjectTable(table('formsui.pjx', 'formsui.PJT'), 'formsui.pjx');
    const doc = toProjectDocument(imported, new Set());

    expect(doc.$schema).toBe('foxdev-project');
    expect(doc.name).toBe('formsui');
    expect(doc.main).toMatch(/\.prg$/i);
    expect(doc.items.filter((i) => i.kind === 'program').length).toBe(5);
    // nothing needing conversion survived, since we converted nothing
    expect(doc.items.some((i) => i.kind === 'form' || i.kind === 'menu')).toBe(false);
  });

  it('renames converted items to their FoxDev extension', () => {
    expect(targetPath({ sourcePath: 'forms/Cust.scx', kind: 'form', excluded: false, main: false, needsConversion: true })).toBe('forms/Cust.fxf');
    expect(targetPath({ sourcePath: 'Main.mnx', kind: 'menu', excluded: false, main: false, needsConversion: true })).toBe('Main.fxm');
    expect(targetPath({ sourcePath: 'lib/util.prg', kind: 'program', excluded: false, main: false, needsConversion: false })).toBe('lib/util.prg');
  });

  it('reads a second real project', () => {
    const imported = readProjectTable(table('Grid.pjx', 'Grid.PJT'), 'Grid.pjx');
    expect(imported.name).toBe('Grid');
    expect(imported.items.length).toBeGreaterThan(0);
  });

  it('refuses a table that is not a project', () => {
    expect(() => readProjectTable(table('AI_Table.DBF'), 'AI_Table.DBF')).toThrow(/not a Visual FoxPro project/i);
  });
});

describe('files with an unfamiliar type letter', () => {
  it('keeps them in the project instead of dropping them', () => {
    // VFP writes 'x' for XML and other loose files; older projects use letters we do not map
    const t = table('formsui.pjx', 'formsui.PJT');
    const patched: DbfTableData = {
      ...t,
      records: t.records.map((r) => {
        const typeIndex = t.fields.findIndex((f) => f.name.toUpperCase() === 'TYPE');
        const nameIndex = t.fields.findIndex((f) => f.name.toUpperCase() === 'NAME');
        const name = r.values[nameIndex];
        if (typeof name !== 'string' || !name.toLowerCase().endsWith('.txt')) return r;
        const values = [...r.values];
        values[typeIndex] = 'x';
        values[nameIndex] = 'customerxml.xml';
        return { ...r, values };
      }),
    };

    const imported = readProjectTable(patched, 'formsui.pjx');
    const xml = imported.items.find((i) => i.sourcePath === 'customerxml.xml');
    expect(xml, 'an unmapped type letter must not drop the file').toBeDefined();
    expect(xml!.kind).toBe('other');
    expect(imported.skipped).toEqual([]);
  });
});
