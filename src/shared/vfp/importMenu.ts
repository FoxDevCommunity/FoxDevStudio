/**
 * Converts a Visual FoxPro menu (`.mnx`) table into a FoxDev menu document.
 *
 * An `.mnx` is a DBF like the form formats, but it describes a flat list that is read in file
 * order. `OBJTYPE` says what each row is:
 *
 * - **1** (or **4** for a shortcut menu) opens the file. It carries `SETUP`, `CLEANUP` and the
 *   menu's `LOCATION`; the menu's name is the file's, not the table's.
 * - **2** declares a menu, named by `LEVELNAME`. The first one is the menu bar itself; every
 *   later one belongs to the row *immediately above it*, which is the item that opens it. The
 *   names are not the link (`_msm_file` opens `pad_mfile` in VFP's own dvdmenu.mnx) so pairing
 *   is positional, exactly as VFP writes it.
 * - **3** is one prompt, belonging to the menu its `LEVELNAME` names.
 *
 * `LEVELNAME` is a 10-character field, so long menu names arrive truncated on both sides of the
 * link and still match. `COMMAND`, `MESSAGE` and `SKIPFOR` are FoxPro expressions rather than
 * text, which is why a message arrives wrapped in quotes.
 *
 * Nothing is thrown. Rows that cannot be placed are reported and dropped, so an unfamiliar file
 * produces a partial menu and a list of what was lost rather than nothing.
 */

import { nanoid } from 'nanoid';
import { MENU_SCHEMA_ID, MENU_VERSION, type MenuDocument, type MenuHotkey, type MenuItem, type MenuLocation } from '../menu/schema';
import { hasField, num, text } from './dbfTypes';
import type { DbfTableData } from './dbfTypes';
import type { VfpImportWarning } from './importForm';

export interface ImportedMenu {
  doc: MenuDocument;
  warnings: VfpImportWarning[];
}

/** `OBJTYPE` values the importer acts on. */
const MENU_HEADER = 1;
const SHORTCUT_HEADER = 4;
const LEVEL = 2;
const ITEM = 3;

/** The header's `LOCATION`, in the order VFP's General Options dialog lists them. */
const LOCATIONS: MenuLocation[] = ['Replace', 'Append', 'Before', 'After'];

interface MenuRow {
  objType: number;
  name: string;
  prompt: string;
  command: string;
  procedure: string;
  /** The menu this row belongs to, or names when it declares one. Ten characters at most. */
  levelName: string;
  itemNum: number;
  setup: string;
  cleanup: string;
  message: string;
  skipFor: string;
  keyName: string;
  keyLabel: string;
  location: number;
  /** Name of the Visual FoxPro system bar this item stands for, when it is one. */
  resName: string;
}

export function isMenuTable(table: DbfTableData): boolean {
  return hasField(table, 'OBJTYPE') && hasField(table, 'LEVELNAME') && hasField(table, 'PROMPT');
}

function readRows(table: DbfTableData): MenuRow[] {
  const out: MenuRow[] = [];
  for (const record of table.records) {
    if (record.deleted) continue;
    out.push({
      objType: num(table, record, 'OBJTYPE'),
      name: text(table, record, 'NAME'),
      prompt: text(table, record, 'PROMPT'),
      command: text(table, record, 'COMMAND'),
      procedure: text(table, record, 'PROCEDURE'),
      levelName: text(table, record, 'LEVELNAME'),
      itemNum: Number(text(table, record, 'ITEMNUM')) || num(table, record, 'ITEMNUM'),
      setup: text(table, record, 'SETUP'),
      cleanup: text(table, record, 'CLEANUP'),
      message: text(table, record, 'MESSAGE'),
      skipFor: text(table, record, 'SKIPFOR'),
      keyName: text(table, record, 'KEYNAME'),
      keyLabel: text(table, record, 'KEYLABEL'),
      location: num(table, record, 'LOCATION'),
      resName: text(table, record, 'RESNAME'),
    });
  }
  return out;
}

/** Converts a parsed `.mnx` table. `name` is the file stem, which is the menu's name in VFP. */
export function importMenuTable(table: DbfTableData, name: string): ImportedMenu {
  const warnings: VfpImportWarning[] = [];
  const doc: MenuDocument = { $schema: MENU_SCHEMA_ID, version: MENU_VERSION, name, location: 'Replace', items: [] };
  if (!isMenuTable(table)) {
    warnings.push({ object: name, kind: 'other', message: 'This table is not a Visual FoxPro menu.' });
    return { doc, warnings };
  }

  const rows = readRows(table);
  const header = rows.find((r) => r.objType === MENU_HEADER || r.objType === SHORTCUT_HEADER);
  if (header) {
    if (header.setup !== '') doc.setup = header.setup;
    if (header.cleanup !== '') doc.cleanup = header.cleanup;
    doc.location = LOCATIONS[header.location] ?? 'Replace';
    // a shortcut menu is the same table with a different header: its items are the bars of a
    // popup shown where the pointer is, rather than the pads of a bar along the top
    if (header.objType === SHORTCUT_HEADER) doc.shortcut = true;
  }

  // Items are collected per menu, and each menu is claimed by the item written just above it.
  const levels = new Map<string, MenuItem[]>();
  let bar: MenuItem[] | undefined;
  let previousItem: MenuItem | undefined;
  const orphans: MenuRow[] = [];

  for (const row of rows) {
    if (row.objType === LEVEL) {
      const items: MenuItem[] = [];
      levels.set(row.levelName.toLowerCase(), items);
      if (!bar) bar = items;
      else if (previousItem) previousItem.children = items;
      else orphans.push(row);
      previousItem = undefined;
      continue;
    }
    if (row.objType !== ITEM) continue;

    const item = toItem(row);
    const into = levels.get(row.levelName.toLowerCase());
    if (into) into.push(item);
    else orphans.push(row);
    previousItem = item;
  }

  doc.items = bar ?? [];
  finishSubmenus(doc.items);

  for (const row of orphans) {
    warnings.push({
      object: row.name || row.prompt || row.levelName,
      kind: 'parentNotFound',
      message: `"${row.prompt || row.name || row.levelName}" belongs to menu "${row.levelName}", which no item opens; it was not imported.`,
    });
  }
  return { doc, warnings };
}

/**
 * An item is a submenu only once it turns out to have items. VFP leaves an empty menu behind
 * when a prompt that used to open one becomes a separator, and those must stay separators.
 */
function finishSubmenus(items: MenuItem[]): void {
  for (const item of items) {
    if (!item.children) continue;
    if (item.children.length === 0) {
      delete item.children;
      continue;
    }
    item.result = { type: 'submenu' };
    finishSubmenus(item.children);
  }
}

function toItem(row: MenuRow): MenuItem {
  const item: MenuItem = { id: nanoid(10), prompt: row.prompt, result: { type: 'bar' } };
  if (row.name !== '') item.name = row.name;
  else if (row.resName !== '') item.name = row.resName;

  const message = unquote(row.message);
  if (message !== '') item.message = message;
  if (row.skipFor !== '') item.skipFor = row.skipFor;

  const hotkey = toHotkey(row.keyName, row.keyLabel);
  if (hotkey) item.hotkey = hotkey;

  // a submenu is decided later, once the rows that follow say whether it has any items
  if (row.procedure !== '') item.result = { type: 'procedure', text: row.procedure };
  else if (row.command !== '') item.result = { type: 'command', text: row.command };
  return item;
}

/** `MESSAGE` is an expression; the designer writes a plain string as a quoted literal. */
export function unquote(expression: string): string {
  const match = /^(["'])([^"']*)\1$/.exec(expression);
  return match ? (match[2] ?? '') : expression;
}

/**
 * VFP records a shortcut as the key it presses (`CTRL+N`) plus the text shown beside the prompt.
 * A pad also carries `ALT+F` with no text: that is the access key the prompt's `\<` already
 * marks, so it is not a shortcut and is left out.
 */
export function toHotkey(keyName: string, keyLabel: string): MenuHotkey | undefined {
  if (keyName === '' || keyLabel === '') return undefined;
  const parts = keyName
    .toUpperCase()
    .split('+')
    .map((p) => p.trim())
    .filter((p) => p !== '');
  const key = parts.pop();
  if (key === undefined || key === '') return undefined;

  const out: MenuHotkey = { key: key.length === 1 ? key : titleCaseKey(key) };
  if (parts.includes('CTRL')) out.ctrl = true;
  if (parts.includes('ALT')) out.alt = true;
  if (parts.includes('SHIFT')) out.shift = true;
  // the designer's own label wins when it is not just the key spelled back
  const formatted = [out.ctrl ? 'Ctrl' : '', out.alt ? 'Alt' : '', out.shift ? 'Shift' : '', out.key].filter(Boolean).join('+');
  if (keyLabel !== formatted) out.label = keyLabel;
  return out;
}

/** `F5` stays `F5`; `DELETE` becomes `Delete`, which is what the runtime compares against. */
function titleCaseKey(key: string): string {
  if (/^F\d{1,2}$/.test(key)) return key;
  return key.charAt(0) + key.slice(1).toLowerCase();
}
