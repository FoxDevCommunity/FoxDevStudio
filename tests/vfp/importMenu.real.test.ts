import { readFileSync } from 'node:fs';
import { beforeAll, describe, expect, it } from 'vitest';
import { importMenuTable } from '@shared/vfp/importMenu';
import { parseMenuDocument, stringifyMenuDocument } from '@shared/menu/serialize';
import { flattenMenu } from '@shared/menu/tree';
import type { DbfReadResult, DbfTableData } from '@shared/vfp/dbfTypes';
import type { MenuItem } from '@shared/menu/schema';
import { loadFoxVm, type FoxVmModule } from '../../src/wasm/foxvm/loader';

/**
 * These run against menus that ship with Visual FoxPro 9, not fixtures we wrote. They are the
 * only real evidence that the menu importer handles what VFP actually produces.
 */
const FIXTURES = 'crates/foxvm/tests/fixtures';
const read = (name: string) => new Uint8Array(readFileSync(`${FIXTURES}/${name}`));

let vm: FoxVmModule;

function table(stem: string): DbfTableData {
  const result = vm.read_dbf(read(`${stem}.mnx`), read(`${stem}.mnt`)) as DbfReadResult;
  if (!result.ok) throw new Error(result.error);
  return result;
}

const prompts = (items: MenuItem[]) => items.map((i) => i.prompt);

beforeAll(async () => {
  vm = await loadFoxVm();
});

describe('importing real Visual FoxPro menus', () => {
  it('imports jump.mnx: one pad with four commands', () => {
    const { doc, warnings } = importMenuTable(table('jump'), 'jump');
    expect(warnings).toEqual([]);
    expect(doc.name).toBe('jump');
    // the header's LOCATION says this menu is appended to VFP's own, not a replacement
    expect(doc.location).toBe('Append');

    expect(prompts(doc.items)).toEqual(['\\<Analyzer']);
    const analyzer = doc.items[0]!;
    expect(analyzer.result.type).toBe('submenu');
    expect(prompts(analyzer.children!)).toEqual(['Go to \\<Definition', 'Go to \\<Reference', 'Go to \\<Next', 'Go \\<Back']);
    expect(analyzer.children![0]!.result).toEqual({ type: 'command', text: 'do tex in analyzer with "D"' });
    expect(analyzer.children![0]!.hotkey).toEqual({ key: 'D', ctrl: true });
  });

  it('pairs a pad with the menu written under it, whatever the two are called', () => {
    // dvdmenu.mnx has a pad named _msm_file that opens a menu named pad_mfile
    const { doc } = importMenuTable(table('dvdmenu'), 'dvdmenu');
    const file = doc.items[0]!;
    expect(file.prompt).toBe('\\<File');
    expect(file.name).toBe('_msm_file');
    expect(file.result.type).toBe('submenu');
    expect(prompts(file.children!)).toContain('\\<New...');
    expect(file.children!.at(-1)!.result).toEqual({ type: 'procedure', text: 'POP MENU _MSYSMENU\r\nRELEASE lHasSolutionDVDMENU' });
  });

  it('carries the menu-level setup and cleanup across', () => {
    const { doc } = importMenuTable(table('dvdmenu'), 'dvdmenu');
    expect(doc.setup).toBe('PUSH MENU _MSYSMENU');
    expect(doc.cleanup).toContain('PROCEDURE report_mru');
  });

  it('reads a message as the text it is, not the expression VFP stores', () => {
    const { doc } = importMenuTable(table('dvdmenu'), 'dvdmenu');
    expect(doc.items[0]!.message).toBe('Creates, opens, saves, prints files or quits Visual FoxPro');
    // a pad's ALT key is the access key the prompt already marks, so it is not a shortcut
    expect(doc.items[0]!.hotkey).toBeUndefined();
    const save = doc.items[0]!.children!.find((c) => c.prompt === '\\<Save')!;
    expect(save.hotkey).toEqual({ key: 'S', ctrl: true });
  });

  it('imports a shortcut menu, including a nested submenu and a disabled item', () => {
    const { doc, warnings } = importMenuTable(table('shortcut'), 'shortcut');
    // it is marked as one rather than reported as a loss: a shortcut menu is shown where the
    // pointer is, which is what VFP's `DEFINE POPUP ... SHORTCUT` does
    expect(doc.shortcut).toBe(true);
    expect(warnings).toEqual([]);
    expect(doc.setup).toContain('PARAMETERS Param1');

    const shortcut = doc.items[0]!;
    expect(prompts(shortcut.children!)).toEqual([
      'This is a FoxPro Menu',
      'It has icons and submenus',
      'This item has a submenu',
      '\\-',
      'This item is disabled',
    ]);
    const nested = shortcut.children![2]!;
    expect(nested.result.type).toBe('submenu');
    expect(prompts(nested.children!)).toEqual(['Submenu bar 1', 'Submenu bar 2', 'CLEAR EVENTS']);
    expect(shortcut.children![4]!.skipFor).toBe('.t.');
  });

  it('keeps a prompt whose menu is empty as a plain bar', () => {
    // VFP leaves an empty menu behind when a prompt that opened one becomes a separator
    const separator = importMenuTable(table('shortcut'), 'shortcut').doc.items[0]!.children![3]!;
    expect(separator.prompt).toBe('\\-');
    expect(separator.result).toEqual({ type: 'bar' });
    expect(separator.children).toBeUndefined();
  });

  it('produces documents the menu serializer accepts', () => {
    for (const stem of ['jump', 'dvdmenu', 'shortcut']) {
      const { doc } = importMenuTable(table(stem), stem);
      const parsed = parseMenuDocument(stringifyMenuDocument(doc));
      expect(parsed.ok, stem).toBe(true);
      // every item id is unique, which the designer's tree relies on
      const ids = flattenMenu(doc.items).map(({ item }) => item.id);
      expect(new Set(ids).size, stem).toBe(ids.length);
    }
  });
});
