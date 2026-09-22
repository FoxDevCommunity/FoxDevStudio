import { beforeEach, describe, expect, it } from 'vitest';
import { createMenuDesignerStore, selectMenuIsDirty, type MenuDesignerStore } from '@renderer/menu-designer/store/createMenuDesignerStore';
import { findMenuItem } from '@shared/menu/tree';
import { sampleMenu } from '../helpers/fixtures';

let store: MenuDesignerStore;
let n = 0;
const s = () => store.getState();
const item = (id: string) => findMenuItem(s().doc.items, id)!.item;

beforeEach(() => {
  n = 0;
  store = createMenuDesignerStore(sampleMenu(), { newId: () => `m${++n}` });
});

describe('menu designer store', () => {
  it('inserts items after the selection or as children', () => {
    const top = s().insertItem();
    expect(s().doc.items.map((i) => i.id)).toEqual(['file', 'help', top]);
    expect(item(top)).toMatchObject({ prompt: 'Item1', result: { type: 'command', text: '' } });
    expect(s().selectedId).toBe(top);

    s().select('new');
    const sep = s().insertItem('separator');
    expect(item('file').children!.map((i) => i.id)).toEqual(['new', sep, 'sep', 'exit']);
    expect(item(sep).prompt).toBe('\\-');

    const child = s().insertChild('about');
    expect(item('about').result.type).toBe('submenu');
    expect(item('about').children!.map((i) => i.id)).toEqual([child]);
    expect(item(child).prompt).toBe('Item2');
  });

  it('removes, moves, indents and outdents', () => {
    s().removeItem('new');
    expect(item('file').children!.map((i) => i.id)).toEqual(['sep', 'exit']);
    expect(s().selectedId).toBe('sep');
    expect(s().moveItem('exit', -1)).toBe(true);
    expect(item('file').children!.map((i) => i.id)).toEqual(['exit', 'sep']);
    expect(s().moveItem('exit', -1)).toBe(false);
    expect(s().outdent('sep')).toBe(true);
    expect(s().doc.items.map((i) => i.id)).toEqual(['file', 'sep', 'help']);
    expect(s().indent('help')).toBe(true);
    expect(item('sep').children!.map((i) => i.id)).toEqual(['help']);
    s().removeItem('file');
    expect(s().selectedId).toBe('sep');
  });

  it('edits fields, results, hotkeys and document fields', () => {
    s().setPrompt('new', '\\<Open');
    s().setItemField('new', 'message', 'Opens');
    s().setItemField('new', 'message', '');
    s().setEnabled('new', false);
    s().setResult('new', 'procedure', 'DO x');
    s().setHotkey('new', { key: 'O', ctrl: true });
    expect(item('new')).toEqual({ id: 'new', prompt: '\\<Open', enabled: false, result: { type: 'procedure', text: 'DO x' }, hotkey: { key: 'O', ctrl: true } });
    s().setHotkey('new', undefined);
    s().setEnabled('new', true);
    expect(item('new').hotkey).toBeUndefined();
    expect(item('new').enabled).toBeUndefined();
    s().setResult('new', 'submenu');
    expect(item('new')).toMatchObject({ result: { type: 'submenu' }, children: [] });

    s().setDocField('name', 'Main2');
    s().setDocField('setup', 'SET TALK OFF');
    s().setLocation('Append');
    expect(s().doc).toMatchObject({ name: 'Main2', setup: 'SET TALK OFF', location: 'Append' });
    s().setDocField('setup', '');
    expect(s().doc.setup).toBeUndefined();
  });

  it('undoes and redoes with selection and dirty tracking', () => {
    expect(selectMenuIsDirty(s())).toBe(false);
    s().select('exit');
    s().setPrompt('exit', 'Quit');
    const id = s().insertItem();
    expect(selectMenuIsDirty(s())).toBe(true);
    s().undo();
    expect(findMenuItem(s().doc.items, id)).toBeUndefined();
    expect(s().selectedId).toBe('exit');
    s().undo();
    expect(item('exit').prompt).toBe('E\\<xit');
    expect(selectMenuIsDirty(s())).toBe(false);
    s().redo();
    s().redo();
    expect(s().selectedId).toBe(id);
    s().markSaved();
    expect(selectMenuIsDirty(s())).toBe(false);
    s().load(sampleMenu());
    expect(s().selectedId).toBeNull();
  });
});
