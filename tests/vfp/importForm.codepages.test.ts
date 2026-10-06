import { readFileSync } from 'node:fs';
import { beforeAll, expect, it } from 'vitest';
import { importFormTable } from '@shared/vfp/importForm';
import { loadFoxVm, type FoxVmModule } from '../../src/wasm/foxvm/loader';
import { buildDbf, buildFpt } from '../data/dbfFixture';

const cases = readFileSync('crates/foxvm/tests/codepage_reference.txt', 'utf8')
  .trim().split('\n').filter((line) => !line.startsWith('#')).map((line) => {
    const [page, marker, ...points] = line.split(/\s+/);
    const letters = points.map((point, index) => ({ text: String.fromCodePoint(parseInt(point!, 16)), byte: index + 128 }))
      .filter(({ text }) => /\p{L}/u.test(text)).slice(0, 3);
    return { page: Number(page), marker: parseInt(marker!, 16),
      expected: letters.map(({ text }) => text).join(''),
      raw: String.fromCharCode(...letters.map(({ byte }) => byte)) };
  });
let vm: FoxVmModule;
beforeAll(async () => { vm = await loadFoxVm(); });

it.each(cases)('imports a synthetic CP$page caption through WASM', ({ marker, raw, expected }) => {
  const dbf = buildDbf([
    { name: 'OBJNAME', kind: 'C', width: 20 },
    { name: 'BASECLASS', kind: 'C', width: 20 },
    { name: 'CLASS', kind: 'C', width: 20 },
    { name: 'PARENT', kind: 'C', width: 20 },
    { name: 'PROPERTIES', kind: 'M', width: 4 },
  ], [['Demo', 'form', 'form', '', '1']]);
  dbf[29] = marker;
  const memo = buildFpt([`Caption = "Demo ${raw}"\r\n`], 512);
  const table = vm.read_dbf(dbf, memo);
  if (!table.ok) throw new Error(table.error);
  const { doc } = importFormTable(table, 'demo');
  expect(doc.form.props['Caption']).toBe(`Demo ${expected}`);
});
