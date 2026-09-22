import { act, fireEvent, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it } from 'vitest';
import { render } from '@testing-library/react';
import { EditorView } from '@codemirror/view';
import { App } from '@renderer/App';
import { setApi } from '@renderer/api/foxdev';
import { createMemoryApi, type MemoryApi } from '@renderer/api/memoryApi';
import { useProjectStore } from '@renderer/stores/projectStore';
import { useDocumentsStore } from '@renderer/stores/documentsStore';
import { useSettingsStore } from '@renderer/stores/settingsStore';
import { runCommand, isCommandEnabled } from '@renderer/shell/commands/registry';
import { stringifyFormDocument } from '@shared/form/serialize';
import { stringifyMenuDocument } from '@shared/menu/serialize';
import { stringifyProjectDocument } from '@shared/project/serialize';
import { sampleForm, sampleMenu, sampleProject } from '../helpers/fixtures';
import { click } from '../helpers/pointer';

let api: MemoryApi;
const P = '/proj';

beforeEach(() => {
  api = createMemoryApi({
    [`${P}/Sample.fxproject`]: stringifyProjectDocument(sampleProject()),
    [`${P}/Form1.fxf`]: stringifyFormDocument(sampleForm()),
    [`${P}/Main.fxm`]: stringifyMenuDocument(sampleMenu()),
    [`${P}/main.prg`]: 'DO FORM Form1',
  });
  api.recent$ = [`${P}/Sample.fxproject`];
  setApi(api);
  useProjectStore.getState().close();
  useDocumentsStore.getState().closeAll();
  useSettingsStore.setState({ theme: 'light', gridSize: 8, snapToGrid: true, showGrid: true });
});

async function openSampleProject() {
  render(<App />);
  await screen.findByTestId('welcome');
  await act(() => useProjectStore.getState().openProject(`${P}/Sample.fxproject`));
  await screen.findByRole('tree', { name: 'Project items' });
}

describe('IDE shell', () => {
  it('starts on the welcome page with recent projects and opens one from it', async () => {
    render(<App />);
    expect(await screen.findByTestId('welcome')).toHaveTextContent('Welcome to FoxDev Studio');
    expect(api.title$).toBe('FoxDev Studio');
    await userEvent.click(await screen.findByRole('button', { name: `${P}/Sample.fxproject` }));
    expect(await screen.findByRole('tree', { name: 'Project items' })).toBeInTheDocument();
    expect(screen.queryByTestId('welcome')).toBeNull();
    await waitFor(() => expect(api.title$).toBe('Sample - FoxDev Studio'));
    expect(screen.getByTestId('status-bar')).toHaveTextContent('Project: Sample');
  });

  it('lists project items grouped by kind and opens a form on double-click', async () => {
    await openSampleProject();
    const tree = screen.getByRole('tree', { name: 'Project items' });
    expect(within(tree).getByText('Forms (1)')).toBeInTheDocument();
    expect(within(tree).getByText('Menus (1)')).toBeInTheDocument();
    expect(within(tree).getByText('Programs (1)')).toBeInTheDocument();
    const formItem = document.querySelector('[data-item-path="Form1.fxf"]')!;
    expect(formItem).toHaveTextContent('(main)');
    fireEvent.doubleClick(formItem);
    expect(await screen.findByTestId('form-designer')).toBeInTheDocument();
    expect(screen.getByRole('tab', { name: 'Form1.fxf' })).toBeInTheDocument();
    expect(screen.getByTestId('properties-window')).toHaveTextContent('Caption');
    expect(screen.getByTestId('selection-status')).toHaveTextContent('Form1 (Form)');
    click(document.querySelector('[data-control-id="Command1"]')!, { x: 20, y: 20 });
    expect(screen.getByTestId('selection-status')).toHaveTextContent('Command1 (CommandButton) 16, 16 84 x 27');
  });

  it('shows dirty markers, saves with Ctrl+S, and prompts when closing a dirty tab', async () => {
    await openSampleProject();
    fireEvent.doubleClick(document.querySelector('[data-item-path="Form1.fxf"]')!);
    await screen.findByTestId('form-designer');
    const docs = useDocumentsStore.getState();
    const formDoc = docs.docs[docs.activeId!]!;
    act(() => {
      if (formDoc.kind === 'form') formDoc.store.getState().setProp(['Command1'], 'Caption', 'Changed');
    });
    expect(await screen.findByRole('tab', { name: 'Form1.fxf' })).toHaveTextContent('Form1.fxf*');
    await waitFor(() => expect(api.edited$).toBe(true));

    fireEvent.keyDown(window, { key: 's', ctrlKey: true });
    await waitFor(() => expect(api.files$.get(`${P}/Form1.fxf`)).toContain('"Caption": "Changed"'));
    await waitFor(() => expect(screen.getByRole('tab', { name: 'Form1.fxf' })).not.toHaveTextContent('*'));

    act(() => {
      if (formDoc.kind === 'form') formDoc.store.getState().setProp(['Command1'], 'Caption', 'Again');
    });
    api.queueDialog('message', 2); // Cancel
    await userEvent.click(screen.getByRole('button', { name: 'Close Form1.fxf' }));
    expect(screen.getByRole('tab', { name: 'Form1.fxf' })).toBeInTheDocument();
    api.queueDialog('message', 1); // Don't save
    await userEvent.click(screen.getByRole('button', { name: 'Close Form1.fxf' }));
    await waitFor(() => expect(screen.queryByRole('tab', { name: 'Form1.fxf' })).toBeNull());
    expect(api.files$.get(`${P}/Form1.fxf`)).toContain('"Caption": "Changed"');
  });

  it('runs commands from the menu bar and reports enabled state', async () => {
    await openSampleProject();
    expect(isCommandEnabled('edit.undo')).toBe(false);
    expect(isCommandEnabled('file.save')).toBe(false);
    await userEvent.click(screen.getByRole('button', { name: 'File' }));
    await userEvent.click(await screen.findByRole('menuitem', { name: /New Form/ }));
    expect(await screen.findByTestId('form-designer')).toBeInTheDocument();
    expect(screen.getByRole('tab', { name: 'Form1 (new)' })).toBeInTheDocument();
    expect(isCommandEnabled('file.save')).toBe(true);
    expect(isCommandEnabled('edit.paste')).toBe(true);
    expect(isCommandEnabled('format.align.left')).toBe(false);

    await userEvent.click(screen.getByRole('button', { name: 'View' }));
    await userEvent.click(await screen.findByRole('menuitem', { name: /Toggle Dark Theme/ }));
    expect(useSettingsStore.getState().theme).toBe('dark');
    await userEvent.click(screen.getByRole('button', { name: 'View' }));
    await userEvent.click(await screen.findByRole('menuitem', { name: /Properties Window/ }));
    await waitFor(() => expect(screen.queryByTestId('properties-window')).toBeNull());
    await act(() => runCommand('view.properties'));
    expect(screen.getByTestId('properties-window')).toBeInTheDocument();
  });

  it('opens programs as text, edits them and honours the window close request', async () => {
    await openSampleProject();
    fireEvent.doubleClick(document.querySelector('[data-item-path="main.prg"]')!);
    const host = await screen.findByTestId('code-editor');
    const view = EditorView.findFromDOM(host)!;
    expect(view.state.doc.toString()).toBe('DO FORM Form1');
    act(() => view.dispatch({ changes: { from: view.state.doc.length, insert: '\nRETURN' } }));
    expect(screen.getByRole('tab', { name: 'main.prg' })).toHaveTextContent('*');

    api.queueDialog('message', 2); // Cancel closing
    act(() => api.emitCloseRequested());
    await waitFor(() => expect(api.closeConfirmations$).toEqual([false]));
    expect(useProjectStore.getState().doc).not.toBeNull();
    api.queueDialog('message', 0); // Save and close
    act(() => api.emitCloseRequested());
    await waitFor(() => expect(api.closeConfirmations$).toEqual([false, true]));
    expect(api.files$.get(`${P}/main.prg`)).toBe('DO FORM Form1\nRETURN');
    expect(useProjectStore.getState().doc).toBeNull();
  });

  it('narrows the explorer to what a search matches, and opens the groups holding it', async () => {
    await openSampleProject();
    const tree = screen.getByRole('tree', { name: 'Project items' });
    expect(within(tree).getByText('Programs (1)')).toBeInTheDocument();

    // a subsequence of the name, not a substring: `mpg` finds main.prg
    await userEvent.type(screen.getByRole('searchbox', { name: 'Search files' }), 'mpg');
    await waitFor(() => expect(document.querySelector('[data-item-path="main.prg"]')).toBeInTheDocument());

    // the groups with no hit are gone, rather than sitting there empty
    expect(within(tree).queryByText(/^Forms/)).toBeNull();
    expect(within(tree).queryByText(/^Menus/)).toBeNull();
    expect(document.querySelector('[data-item-path="Form1.fxf"]')).toBeNull();

    await userEvent.clear(screen.getByRole('searchbox', { name: 'Search files' }));
    await waitFor(() => expect(document.querySelector('[data-item-path="Form1.fxf"]')).toBeInTheDocument());
  });

  it("lists the form's objects and methods, and goes to one when clicked", async () => {
    await openSampleProject();
    await userEvent.dblClick(document.querySelector('[data-item-path="Form1.fxf"]')!);
    const panel = await screen.findByTestId('symbol-explorer');

    // the form, its controls, and the methods each of them carries
    expect(within(panel).getByLabelText('Form1')).toBeInTheDocument();
    expect(within(panel).getByLabelText('Command1')).toBeInTheDocument();
    expect(within(panel).getByLabelText('Command1.Click')).toBeInTheDocument();

    // a subsequence search, the same as the project explorer's
    await userEvent.type(within(panel).getByRole('searchbox', { name: 'Search symbols' }), 'c1clk');
    await waitFor(() => expect(within(panel).queryByLabelText('Form1')).toBeNull());
    expect(within(panel).getByLabelText('Command1.Click')).toBeInTheDocument();

    // clicking a method opens its editor
    await userEvent.click(within(panel).getByLabelText('Command1.Click'));
    expect(await screen.findByRole('tab', { name: 'Form1.Command1.Click' })).toBeInTheDocument();
  });

  it('uses the explorer toolbar and context menu', async () => {
    await openSampleProject();
    api.queueDialog('openFile', `${P}/extra.prg`);
    await userEvent.click(screen.getByRole('button', { name: 'Add File' }));
    await waitFor(() => expect(useProjectStore.getState().doc!.items.map((i) => i.path)).toContain('extra.prg'));
    expect(screen.getByText('Programs (2)')).toBeInTheDocument();
    fireEvent.contextMenu(document.querySelector('[data-item-path="extra.prg"]')!);
    await userEvent.click(await screen.findByRole('menuitem', { name: 'Set Main' }));
    expect(useProjectStore.getState().doc!.main).toBe('extra.prg');
    fireEvent.contextMenu(document.querySelector('[data-item-path="extra.prg"]')!);
    await userEvent.click(await screen.findByRole('menuitem', { name: 'Remove from Project' }));
    await waitFor(() => expect(screen.getByText('Programs (1)')).toBeInTheDocument());
    expect(useProjectStore.getState().doc!.main).toBeUndefined();
  });
});
