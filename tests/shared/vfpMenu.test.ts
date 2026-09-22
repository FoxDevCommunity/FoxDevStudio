import { describe, expect, it } from 'vitest';
import type { DbfFieldInfo, DbfRecordData, DbfTableData } from '@shared/vfp/dbfTypes';
import { importMenuTable, isMenuTable, toHotkey, unquote } from '@shared/vfp/importMenu';

/**
 * Edge cases on a table built to the field list of a real VFP 9 `.mnx`, in file order. The
 * genuine files are covered by `tests/vfp/importMenu.real.test.ts`; these pin the shapes that
 * are awkward to find in one.
 */
const FIELD_NAMES = [
  'OBJTYPE',
  'OBJCODE',
  'NAME',
  'PROMPT',
  'COMMAND',
  'MESSAGE',
  'PROCTYPE',
  'PROCEDURE',
  'SETUPTYPE',
  'SETUP',
  'CLEANTYPE',
  'CLEANUP',
  'MARK',
  'KEYNAME',
  'KEYLABEL',
  'SKIPFOR',
  'NAMECHANGE',
  'NUMITEMS',
  'LEVELNAME',
  'ITEMNUM',
  'COMMENT',
  'LOCATION',
  'SCHEME',
  'SYSRES',
  'RESNAME',
] as const;

const NUMERIC = new Set(['OBJTYPE', 'OBJCODE', 'PROCTYPE', 'SETUPTYPE', 'CLEANTYPE', 'NUMITEMS', 'LOCATION', 'SCHEME', 'SYSRES']);

const FIELDS: DbfFieldInfo[] = FIELD_NAMES.map((name) => {
  if (NUMERIC.has(name)) return { name, kind: 'N', length: 2, decimals: 0 };
  if (name === 'MARK') return { name, kind: 'C', length: 1, decimals: 0 };
  if (name === 'NAMECHANGE') return { name, kind: 'L', length: 1, decimals: 0 };
  // LEVELNAME really is ten characters wide, which truncates long menu names in the file
  if (name === 'LEVELNAME') return { name, kind: 'C', length: 10, decimals: 0 };
  if (name === 'ITEMNUM') return { name, kind: 'C', length: 3, decimals: 0 };
  return { name, kind: 'M', length: 4, decimals: 0 };
});

type RowSpec = Partial<Record<(typeof FIELD_NAMES)[number], string | number>> & { deleted?: boolean };

function row(spec: RowSpec): DbfRecordData {
  const { deleted = false, ...fields } = spec;
  return { deleted, values: FIELD_NAMES.map((name) => fields[name] ?? (NUMERIC.has(name) ? 0 : '')) };
}

function table(...records: DbfRecordData[]): DbfTableData {
  return { ok: true, version: 0x30, codepage: 1252, fields: FIELDS, records };
}

/** The header, the bar, then whatever rows the test needs. */
function menu(...records: DbfRecordData[]): DbfTableData {
  return table(row({ OBJTYPE: 1, OBJCODE: 22 }), row({ OBJTYPE: 2, OBJCODE: 1, NAME: '_msysmenu', LEVELNAME: '_msysmenu' }), ...records);
}

describe('importMenuTable', () => {
  it('recognises a menu table and refuses anything else', () => {
    expect(isMenuTable(menu())).toBe(true);
    const notAMenu: DbfTableData = { ok: true, version: 0x30, codepage: null, fields: [{ name: 'NAME', kind: 'C', length: 8, decimals: 0 }], records: [] };
    expect(isMenuTable(notAMenu)).toBe(false);
    const { doc, warnings } = importMenuTable(notAMenu, 'other');
    expect(doc.items).toEqual([]);
    expect(warnings[0]?.message).toContain('not a Visual FoxPro menu');
  });

  it('names the menu after the file, since the table does not carry one', () => {
    expect(importMenuTable(menu(), 'Main').doc.name).toBe('Main');
  });

  it('keeps items in file order rather than sorting by ITEMNUM', () => {
    // VFP writes them in order; ITEMNUM is its own bookkeeping and can disagree after an edit
    const doc = importMenuTable(
      menu(
        row({ OBJTYPE: 3, PROMPT: 'First', LEVELNAME: '_msysmenu', ITEMNUM: '3' }),
        row({ OBJTYPE: 3, PROMPT: 'Second', LEVELNAME: '_msysmenu', ITEMNUM: '1' }),
      ),
      'main',
    ).doc;
    expect(doc.items.map((i) => i.prompt)).toEqual(['First', 'Second']);
  });

  it('prefers a procedure over a command when a row carries both', () => {
    const doc = importMenuTable(
      menu(row({ OBJTYPE: 3, PROMPT: 'Both', LEVELNAME: '_msysmenu', COMMAND: 'DO one', PROCEDURE: 'DO two' })),
      'main',
    ).doc;
    expect(doc.items[0]!.result).toEqual({ type: 'procedure', text: 'DO two' });
  });

  it('takes a deleted row out of the menu', () => {
    const doc = importMenuTable(
      menu(
        row({ OBJTYPE: 3, PROMPT: 'Gone', LEVELNAME: '_msysmenu', deleted: true }),
        row({ OBJTYPE: 3, PROMPT: 'Here', LEVELNAME: '_msysmenu' }),
      ),
      'main',
    ).doc;
    expect(doc.items.map((i) => i.prompt)).toEqual(['Here']);
  });

  it('reports an item whose menu no pad opens instead of dropping it silently', () => {
    const { doc, warnings } = importMenuTable(
      menu(row({ OBJTYPE: 3, PROMPT: 'Here', LEVELNAME: '_msysmenu' }), row({ OBJTYPE: 3, PROMPT: 'Lost', LEVELNAME: '_mnowhere' })),
      'main',
    );
    expect(doc.items.map((i) => i.prompt)).toEqual(['Here']);
    expect(warnings.map((w) => w.message).join(' ')).toContain('_mnowhere');
  });

  it('reads the LOCATION the header records', () => {
    const at = (location: number) => importMenuTable(table(row({ OBJTYPE: 1, LOCATION: location }), row({ OBJTYPE: 2, LEVELNAME: '_msysmenu' })), 'm').doc.location;
    expect(at(0)).toBe('Replace');
    expect(at(1)).toBe('Append');
    expect(at(2)).toBe('Before');
    expect(at(3)).toBe('After');
    expect(at(99)).toBe('Replace');
  });

  it('matches a menu to its pad through the ten-character level name', () => {
    const doc = importMenuTable(
      menu(
        row({ OBJTYPE: 3, PROMPT: 'This item has a submenu', LEVELNAME: '_msysmenu' }),
        row({ OBJTYPE: 2, NAME: 'Thisitemha', LEVELNAME: 'Thisitemha' }),
        row({ OBJTYPE: 3, PROMPT: 'Inside', LEVELNAME: 'Thisitemha' }),
      ),
      'main',
    ).doc;
    expect(doc.items[0]!.result.type).toBe('submenu');
    expect(doc.items[0]!.children!.map((i) => i.prompt)).toEqual(['Inside']);
  });

  it('names an item after the system bar it stands for when it has no name of its own', () => {
    const doc = importMenuTable(menu(row({ OBJTYPE: 3, PROMPT: '\\<Save', LEVELNAME: '_msysmenu', SYSRES: 1, RESNAME: '_mfi_save' })), 'main').doc;
    expect(doc.items[0]!.name).toBe('_mfi_save');
    expect(doc.items[0]!.result).toEqual({ type: 'bar' });
  });
});

describe('menu expressions and shortcuts', () => {
  it('unwraps a message the designer wrote as a quoted literal', () => {
    expect(unquote('"Quits Visual FoxPro"')).toBe('Quits Visual FoxPro');
    expect(unquote("'single'")).toBe('single');
    // anything that is a real expression is left exactly as written
    expect(unquote('cMsg + " now"')).toBe('cMsg + " now"');
    expect(unquote('')).toBe('');
  });

  it('reads the modifiers VFP writes into KEYNAME', () => {
    expect(toHotkey('CTRL+N', 'Ctrl+N')).toEqual({ key: 'N', ctrl: true });
    expect(toHotkey('CTRL+SHIFT+S', 'Ctrl+Shift+S')).toEqual({ key: 'S', ctrl: true, shift: true });
    expect(toHotkey('ALT+F4', 'Alt+F4')).toEqual({ key: 'F4', alt: true });
    expect(toHotkey('F5', 'F5')).toEqual({ key: 'F5' });
    // the designer's own wording is kept when it is not just the key spelled back
    expect(toHotkey('DELETE', 'Del')).toEqual({ key: 'Delete', label: 'Del' });
    // a pad's access key carries no label and is already marked by \< in the prompt
    expect(toHotkey('ALT+F', '')).toBeUndefined();
    expect(toHotkey('', '')).toBeUndefined();
  });
});
