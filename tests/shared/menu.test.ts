import { describe, expect, it } from 'vitest';
import { createEmptyMenuDocument, parseMenuDocument, stringifyMenuDocument } from '@shared/menu/serialize';
import { findMenuItem, flattenMenu, indentMenuItem, moveMenuItem, outdentMenuItem, removeMenuItem } from '@shared/menu/tree';
import { isSeparator } from '@shared/menu/schema';
import { sampleMenu } from '../helpers/fixtures';

describe('menu document', () => {
  it('round trips', () => {
    const doc = sampleMenu();
    const text = stringifyMenuDocument(doc);
    const parsed = parseMenuDocument(text);
    expect(parsed).toMatchObject({ ok: true });
    if (parsed.ok) {
      expect(parsed.doc).toEqual(doc);
      expect(stringifyMenuDocument(parsed.doc)).toBe(text);
    }
    expect(parseMenuDocument('{"$schema":"foxdev-menu","version":1,"name":"","location":"Replace","items":[]}').ok).toBe(false);
    expect(stringifyMenuDocument(createEmptyMenuDocument('M'))).toContain('"name": "M"');
  });

  it('flattens depth-first with depth and detects separators', () => {
    const { items } = sampleMenu();
    expect(flattenMenu(items).map((f) => `${f.depth}:${f.item.id}`)).toEqual(['0:file', '1:new', '1:sep', '1:exit', '0:help', '1:about']);
    expect(isSeparator(findMenuItem(items, 'sep')!.item)).toBe(true);
    expect(isSeparator(findMenuItem(items, 'new')!.item)).toBe(false);
  });

  it('moves, indents and outdents items', () => {
    const { items } = sampleMenu();
    expect(moveMenuItem(items, 'new', -1)).toBe(false);
    expect(moveMenuItem(items, 'new', 1)).toBe(true);
    expect(items[0]!.children!.map((c) => c.id)).toEqual(['sep', 'new', 'exit']);

    expect(outdentMenuItem(items, 'exit')).toBe(true);
    expect(items.map((i) => i.id)).toEqual(['file', 'exit', 'help']);
    expect(outdentMenuItem(items, 'file')).toBe(false);

    expect(indentMenuItem(items, 'help')).toBe(true); // becomes child of 'exit', which turns into a submenu
    expect(findMenuItem(items, 'exit')!.item.result.type).toBe('submenu');
    expect(findMenuItem(items, 'help')!.parent!.id).toBe('exit');
    expect(indentMenuItem(items, 'file')).toBe(false);

    expect(removeMenuItem(items, 'file')!.id).toBe('file');
    expect(findMenuItem(items, 'new')).toBeUndefined();
  });
});
