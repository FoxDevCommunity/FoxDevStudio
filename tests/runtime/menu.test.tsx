import { readFileSync } from 'node:fs';
import { act, render, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeAll, beforeEach, describe, expect, it } from 'vitest';
import { App } from '@renderer/App';
import { setApi } from '@renderer/api/foxdev';
import { createMemoryApi } from '@renderer/api/memoryApi';
import { useProjectStore } from '@renderer/stores/projectStore';
import { useDocumentsStore } from '@renderer/stores/documentsStore';
import { useSessionStore } from '@renderer/runtime/session';
import { createProjectSource } from '@renderer/runtime/projectSource';
import { parseMenuDocument, stringifyMenuDocument } from '@shared/menu/serialize';
import { loadFoxVm } from '../../src/wasm/foxvm/loader';

const P = '/proj';
const sample = (file: string) => readFileSync(`resources/samples/${file}`, 'utf8');
const source = createProjectSource();
const outputText = () => screen.getByRole('list', { name: 'Output' }).textContent ?? '';

beforeAll(async () => {
  await loadFoxVm();
});

function seed(menuText = sample('Main.fxm')) {
  setApi(
    createMemoryApi({
      [`${P}/HelloWorld.fxproject`]: sample('HelloWorld.fxproject'),
      [`${P}/HelloWorld.fxf`]: sample('HelloWorld.fxf'),
      [`${P}/Main.fxm`]: menuText,
      [`${P}/main.prg`]: sample('main.prg'),
    }),
  );
}

beforeEach(() => {
  seed();
  useSessionStore.getState().cancel();
  useSessionStore.setState({ output: [], history: [] });
  useProjectStore.getState().close();
  useDocumentsStore.getState().closeAll();
});

async function openProject() {
  render(<App />);
  await screen.findByTestId('welcome');
  await act(() => useProjectStore.getState().openProject(`${P}/HelloWorld.fxproject`));
  await screen.findByRole('tree', { name: 'Project items' });
}

describe('running a menu', () => {
  it('installs the menu bar with DO Main.fxm', async () => {
    await openProject();
    await act(() => useSessionStore.getState().execute(source, 'DO Main.fxm'));

    const bar = await screen.findByRole('menubar', { name: 'Application menu' });
    expect(within(bar).getByRole('button', { name: 'File' })).toBeInTheDocument();
    expect(within(bar).getByRole('button', { name: 'Help' })).toBeInTheDocument();
  });

  it('runs a command item: About shows its MESSAGEBOX', async () => {
    await openProject();
    await act(() => useSessionStore.getState().execute(source, 'DO Main.fxm'));

    const bar = await screen.findByRole('menubar', { name: 'Application menu' });
    await userEvent.click(within(bar).getByRole('button', { name: 'Help' }));
    await userEvent.click(await screen.findByRole('menuitem', { name: 'About...' }));

    const box = await screen.findByRole('alertdialog');
    expect(within(box).getByText('HelloWorld 1.0')).toBeInTheDocument();
    await userEvent.click(within(box).getByRole('button', { name: 'OK' }));
  });

  it('runs a command item that opens a form', async () => {
    await openProject();
    await act(() => useSessionStore.getState().execute(source, 'DO Main.fxm'));

    const bar = await screen.findByRole('menubar', { name: 'Application menu' });
    await userEvent.click(within(bar).getByRole('button', { name: 'File' }));
    await userEvent.click(await screen.findByRole('menuitem', { name: /^New/ }));

    expect(await screen.findByRole('dialog', { name: 'Hello, World' })).toBeInTheDocument();
  });

  it('ends a program parked in READ EVENTS from the Exit item', async () => {
    await openProject();
    act(() => void useDocumentsStore.getState().openProgram(sample('main.prg'), `${P}/main.prg`));
    const parked = useSessionStore.getState().runProgram(source, 'main.prg');
    await screen.findByRole('dialog', { name: 'Hello, World' });
    await act(() => useSessionStore.getState().execute(source, 'DO Main.fxm'));
    await waitFor(() => expect(screen.getByTestId('run-status')).toHaveTextContent('Waiting for events'));

    const bar = await screen.findByRole('menubar', { name: 'Application menu' });
    await userEvent.click(within(bar).getByRole('button', { name: 'File' }));
    await userEvent.click(await screen.findByRole('menuitem', { name: 'Exit' }));

    await act(() => parked);
    await waitFor(() => expect(screen.getByTestId('run-status')).toHaveTextContent('Ready'));
  });

  it('runs the menu Setup code when the menu is installed', async () => {
    const doc = parseMenuDocument(sample('Main.fxm'));
    if (!doc.ok) throw new Error(doc.error);
    doc.doc.setup = '? "menu ready"';
    seed(stringifyMenuDocument(doc.doc));
    await openProject();

    await act(() => useSessionStore.getState().execute(source, 'DO Main.fxm'));
    await waitFor(() => expect(outputText()).toContain('menu ready'));
  });

  it('disables an item whose SKIP FOR expression is true', async () => {
    const doc = parseMenuDocument(sample('Main.fxm'));
    if (!doc.ok) throw new Error(doc.error);
    const file = doc.doc.items.find((i) => i.id === 'mFile')!;
    file.children!.find((c) => c.id === 'mFileNew')!.skipFor = '.T.';
    seed(stringifyMenuDocument(doc.doc));
    await openProject();
    await act(() => useSessionStore.getState().execute(source, 'DO Main.fxm'));
    await waitFor(() => expect(useSessionStore.getState().menuSkip['mFileNew']).toBe(true));

    const bar = await screen.findByRole('menubar', { name: 'Application menu' });
    await userEvent.click(within(bar).getByRole('button', { name: 'File' }));
    expect(await screen.findByRole('menuitem', { name: /^New/ })).toHaveAttribute('aria-disabled', 'true');
  });

  it('shows a shortcut menu where the pointer is, and waits there', async () => {
    // Visual FoxPro generates `DEFINE POPUP <name> SHORTCUT RELATIVE FROM MROW(), MCOL()` for a
    // shortcut menu and activates it straight away: there is no bar, the items are the bars of
    // one popup, and the program waits until something is chosen or the menu is dismissed.
    const doc = parseMenuDocument(sample('Main.fxm'));
    if (!doc.ok) throw new Error(doc.error);
    doc.doc.shortcut = true;
    doc.doc.items = [{ id: 'sDate', prompt: '\\<Date...', result: { type: 'command', text: '? "picked the date"' } }];
    seed(stringifyMenuDocument(doc.doc));
    await openProject();

    const run = act(() => useSessionStore.getState().execute(source, 'DO Main.fxm'));
    const popup = await screen.findByRole('menu', { name: /shortcut menu/ });
    // no menu bar was installed: a shortcut menu is not one
    expect(screen.queryByRole('menubar', { name: 'Application menu' })).toBeNull();

    await userEvent.click(within(popup).getByRole('menuitem', { name: /^Date/ }));
    await run;
    await waitFor(() => expect(outputText()).toContain('picked the date'));
    expect(useSessionStore.getState().shortcutMenu).toBeNull();
  });

  it('lets a shortcut menu be dismissed without choosing anything', async () => {
    const doc = parseMenuDocument(sample('Main.fxm'));
    if (!doc.ok) throw new Error(doc.error);
    doc.doc.shortcut = true;
    seed(stringifyMenuDocument(doc.doc));
    await openProject();

    const run = act(() => useSessionStore.getState().execute(source, 'DO Main.fxm'));
    await screen.findByRole('menu', { name: /shortcut menu/ });
    await userEvent.click(screen.getByTestId('shortcut-menu-dismiss'));
    await run;
    expect(useSessionStore.getState().shortcutMenu).toBeNull();
  });

  it('reports a missing menu as a runtime error', async () => {
    await openProject();
    const run = act(() => useSessionStore.getState().execute(source, 'DO NoSuch.fxm'));
    await waitFor(() => expect(useSessionStore.getState().errorReport).toBeTruthy());

    const error = await screen.findByRole('alertdialog', { name: 'Program Error' });
    expect(within(error).getByText(/NOSUCH/i)).toBeInTheDocument();
    await userEvent.click(within(error).getByRole('button', { name: 'Cancel' }));
    await run;
    expect(outputText()).toMatch(/NOSUCH/i);
  });
});
