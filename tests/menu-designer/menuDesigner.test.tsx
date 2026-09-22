import { act, fireEvent, screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it } from 'vitest';
import { findMenuItem } from '@shared/menu/tree';
import { createMenuDesignerStore, type MenuDesignerStore } from '@renderer/menu-designer/store/createMenuDesignerStore';
import { MenuDesignerDocument } from '@renderer/menu-designer/MenuDesignerDocument';
import { useDocumentsStore } from '@renderer/stores/documentsStore';
import { renderWithProviders } from '../helpers/render';
import { sampleMenu } from '../helpers/fixtures';

let store: MenuDesignerStore;
const item = (id: string) => findMenuItem(store.getState().doc.items, id)!.item;
const node = (id: string) => document.querySelector<HTMLElement>(`[data-menu-id="${id}"]`)!;
const editor = () => screen.getByTestId('menu-item-editor');

beforeEach(() => {
  useDocumentsStore.getState().closeAll();
  let n = 0;
  store = createMenuDesignerStore(sampleMenu(), { newId: () => `m${++n}` });
  renderWithProviders(<MenuDesignerDocument docId="d" store={store} />);
});

describe('menu designer', () => {
  it('shows the tree with mnemonics stripped and separators drawn', () => {
    const tree = screen.getByRole('tree', { name: 'Menu items' });
    expect(within(tree).getByText('File')).toBeInTheDocument();
    expect(within(tree).getByText('Exit')).toBeInTheDocument();
    expect(node('sep')).toHaveTextContent('────────');
    expect(screen.getByText('Select a menu item to edit it.')).toBeInTheDocument();
  });

  it('selects, edits prompt/result/shortcut/enabled, and undoes', async () => {
    await userEvent.click(node('new'));
    expect(store.getState().selectedId).toBe('new');
    const prompt = within(editor()).getByRole('textbox', { name: 'Prompt' });
    expect(prompt).toHaveValue('\\<New');
    await userEvent.clear(prompt);
    await userEvent.type(prompt, '\\<Open{Enter}');
    expect(item('new').prompt).toBe('\\<Open');
    expect(within(screen.getByRole('tree', { name: 'Menu items' })).getByText('Open')).toBeInTheDocument();

    await userEvent.selectOptions(within(editor()).getByRole('combobox', { name: 'Result' }), 'procedure');
    expect(item('new').result.type).toBe('procedure');
    expect(within(editor()).getByRole('textbox', { name: 'Procedure code' })).toBeInTheDocument();
    await userEvent.selectOptions(within(editor()).getByRole('combobox', { name: 'Result' }), 'command');
    const cmd = within(editor()).getByRole('textbox', { name: 'Result text' });
    await userEvent.clear(cmd);
    await userEvent.type(cmd, 'DO FORM Open{Enter}');
    expect(item('new').result).toEqual({ type: 'command', text: 'DO FORM Open' });

    const shortcut = within(editor()).getByRole('textbox', { name: 'Shortcut' });
    fireEvent.focus(shortcut);
    fireEvent.keyDown(shortcut, { key: 'o', ctrlKey: true, shiftKey: true });
    expect(item('new').hotkey).toEqual({ key: 'O', ctrl: true, shift: true });
    expect(shortcut).toHaveValue('Ctrl+Shift+O');
    await userEvent.click(within(editor()).getByRole('button', { name: 'Clear shortcut' }));
    expect(item('new').hotkey).toBeUndefined();

    await userEvent.click(within(editor()).getByRole('switch', { name: 'Enabled' }));
    expect(item('new').enabled).toBe(false);
    const msg = within(editor()).getByRole('textbox', { name: 'Message' });
    await userEvent.type(msg, 'Opens a form{Enter}');
    expect(item('new').message).toBe('Opens a form');

    act(() => store.getState().undo());
    expect(item('new').message).toBeUndefined();
    act(() => store.getState().undo());
    expect(item('new').enabled).toBeUndefined();
  });

  it('inserts, moves, indents, outdents and deletes from the toolbar', async () => {
    await userEvent.click(node('exit'));
    await userEvent.click(screen.getByRole('button', { name: 'Insert Item' }));
    expect(item('file').children!.map((c) => c.id)).toEqual(['new', 'sep', 'exit', 'm1']);
    expect(store.getState().selectedId).toBe('m1');
    expect(within(editor()).getByRole('textbox', { name: 'Prompt' })).toHaveValue('Item1');
    await userEvent.click(screen.getByRole('button', { name: 'Move Up' }));
    expect(item('file').children!.map((c) => c.id)).toEqual(['new', 'sep', 'm1', 'exit']);
    await userEvent.click(screen.getByRole('button', { name: 'Outdent' }));
    expect(store.getState().doc.items.map((c) => c.id)).toEqual(['file', 'm1', 'help']);
    await userEvent.click(screen.getByRole('button', { name: 'Indent' }));
    expect(item('file').children!.map((c) => c.id)).toEqual(['new', 'sep', 'exit', 'm1']);
    await userEvent.click(screen.getByRole('button', { name: 'Insert Submenu Item' }));
    expect(item('m1').result.type).toBe('submenu');
    expect(item('m1').children!.map((c) => c.id)).toEqual(['m2']);
    await userEvent.click(node('m1'));
    await userEvent.click(screen.getByRole('button', { name: 'Delete Item' }));
    expect(findMenuItem(store.getState().doc.items, 'm1')).toBeUndefined();
    await userEvent.click(screen.getByRole('button', { name: 'Insert Separator' }));
    expect(item(store.getState().selectedId!).prompt).toBe('\\-');
  });

  it('previews a working menu bar with underlined mnemonics and shortcuts', async () => {
    const bar = screen.getByRole('menubar', { name: 'Menu preview' });
    const file = within(bar).getByRole('button', { name: 'File' });
    expect(file.querySelector('u')).toHaveTextContent('F');
    await userEvent.click(file);
    const newItem = await screen.findByRole('menuitem', { name: /New/ });
    expect(newItem).toHaveTextContent('Ctrl+N');
    expect(screen.getByRole('separator')).toBeInTheDocument();
    await userEvent.click(newItem);
    expect(screen.getByTestId('menu-last-choice')).toHaveTextContent('Chosen: File.New');
  });

  it('edits general options', async () => {
    const opts = screen.getByTestId('menu-general-options');
    await userEvent.selectOptions(within(opts).getByRole('combobox', { name: 'Location' }), 'Append');
    expect(store.getState().doc.location).toBe('Append');
    await userEvent.type(within(opts).getByRole('textbox', { name: 'Setup code' }), 'SET TALK OFF');
    expect(store.getState().doc.setup).toBe('SET TALK OFF');
    const name = within(opts).getByRole('textbox', { name: 'Menu name' });
    await userEvent.clear(name);
    await userEvent.type(name, 'Main2{Enter}');
    expect(store.getState().doc.name).toBe('Main2');
  });
});
