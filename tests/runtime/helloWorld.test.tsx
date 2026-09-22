import { readFileSync } from 'node:fs';
import { act, render, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeAll, beforeEach, describe, expect, it } from 'vitest';
import { App } from '@renderer/App';
import { setApi } from '@renderer/api/foxdev';
import { createMemoryApi, type MemoryApi } from '@renderer/api/memoryApi';
import { useProjectStore } from '@renderer/stores/projectStore';
import { useDocumentsStore } from '@renderer/stores/documentsStore';
import { useSessionStore } from '@renderer/runtime/session';
import { createProjectSource } from '@renderer/runtime/projectSource';
import { runCommand } from '@renderer/shell/commands/registry';
import { loadFoxVm } from '../../src/wasm/foxvm/loader';

/** The real sample project: this is the Milestone-2 acceptance test. */
const P = '/proj';
const sample = (file: string) => readFileSync(`resources/samples/${file}`, 'utf8');

let api: MemoryApi;

beforeAll(async () => {
  await loadFoxVm();
});

beforeEach(() => {
  api = createMemoryApi({
    [`${P}/HelloWorld.fxproject`]: sample('HelloWorld.fxproject'),
    [`${P}/HelloWorld.fxf`]: sample('HelloWorld.fxf'),
    [`${P}/Main.fxm`]: sample('Main.fxm'),
    [`${P}/main.prg`]: sample('main.prg'),
  });
  setApi(api);
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

const outputText = () => screen.getByRole('list', { name: 'Output' }).textContent ?? '';
/** Programs and forms come from the open project, exactly as the IDE commands supply them. */
const source = createProjectSource();

describe('running the HelloWorld sample', () => {
  it('runs the project main form and the Say Hi button builds the greeting', async () => {
    await openProject();
    await act(() => runCommand('program.runMain'));

    const dialog = await screen.findByRole('dialog', { name: 'Hello, World' });
    // Init ran THISFORM.txtName.SetFocus()
    const name = within(dialog).getByRole('textbox', { name: 'txtName' });
    await waitFor(() => expect(document.activeElement).toBe(name));

    await userEvent.type(name, 'jorge');
    await userEvent.click(within(dialog).getByRole('checkbox', { name: 'Shout it' }));
    await userEvent.click(within(dialog).getByRole('button', { name: 'Say Hi' }));

    expect(await within(dialog).findByText('HELLO, JORGE!')).toBeInTheDocument();
  });

  it('closes the form with THISFORM.Release()', async () => {
    await openProject();
    await act(() => runCommand('program.runMain'));
    const dialog = await screen.findByRole('dialog', { name: 'Hello, World' });

    await userEvent.click(within(dialog).getByRole('button', { name: 'Close' }));
    await waitFor(() => expect(screen.queryByRole('dialog', { name: 'Hello, World' })).toBeNull());
  });

  it('runs main.prg, which opens the form and parks in READ EVENTS until CLEAR EVENTS', async () => {
    await openProject();
    act(() => void useDocumentsStore.getState().openProgram(sample('main.prg'), `${P}/main.prg`));
    // the program parks in READ EVENTS, so this promise only settles after CLEAR EVENTS
    const parked = runCommand('program.doProgram');

    expect(await screen.findByRole('dialog', { name: 'Hello, World' })).toBeInTheDocument();
    await waitFor(() => expect(screen.getByTestId('run-status')).toHaveTextContent('Waiting for events'));

    await act(() => useSessionStore.getState().execute(source, 'CLEAR EVENTS'));
    await act(() => parked);
    await waitFor(() => expect(screen.getByTestId('run-status')).toHaveTextContent('Ready'));
  });

  it('shows a MESSAGEBOX as a dialog and resumes with the pressed button', async () => {
    await openProject();
    await act(() => runCommand('program.runMain'));
    await screen.findByRole('dialog', { name: 'Hello, World' });

    const run = act(() => useSessionStore.getState().execute(source, 'IF MESSAGEBOX("Save changes?", 4, "FoxDev") = 6\n? "yes"\nELSE\n? "no"\nENDIF'));
    const box = await screen.findByRole('alertdialog', { name: 'FoxDev' });
    expect(within(box).getByText('Save changes?')).toBeInTheDocument();
    await userEvent.click(within(box).getByRole('button', { name: 'No' }));
    await run;

    expect(outputText()).toContain('no');
  });

  it('reports a runtime error with its program and line, then cancels', async () => {
    await openProject();
    await act(() => runCommand('program.runMain'));
    await screen.findByRole('dialog', { name: 'Hello, World' });

    const run = act(() => useSessionStore.getState().execute(source, '? nosuchvar'));
    const error = await screen.findByRole('alertdialog', { name: 'Program Error' });
    expect(within(error).getByText("Variable 'NOSUCHVAR' is not found.")).toBeInTheDocument();
    await userEvent.click(within(error).getByRole('button', { name: 'Cancel' }));
    await run;

    expect(outputText()).toContain("Variable 'NOSUCHVAR' is not found.");
  });
});

describe('Command Window', () => {
  it('evaluates an expression and echoes the line', async () => {
    await openProject();
    const command = screen.getByRole('textbox', { name: 'Command Window' });

    await userEvent.type(command, '? 1 + 1{Enter}');
    await waitFor(() => expect(outputText()).toContain('2'));
    expect(outputText()).toContain('. ? 1 + 1');
    expect(command).toHaveValue('');
  });

  it('opens a form with DO FORM and recalls history with the up arrow', async () => {
    await openProject();
    const command = screen.getByRole('textbox', { name: 'Command Window' });

    await userEvent.type(command, 'DO FORM HelloWorld{Enter}');
    expect(await screen.findByRole('dialog', { name: 'Hello, World' })).toBeInTheDocument();

    await userEvent.click(command);
    await userEvent.keyboard('{ArrowUp}');
    expect(command).toHaveValue('DO FORM HelloWorld');
  });

  it('reports a syntax error in Output without a dialog', async () => {
    await openProject();
    const command = screen.getByRole('textbox', { name: 'Command Window' });

    await userEvent.type(command, 'IF x{Enter}');
    await waitFor(() => expect(outputText()).toMatch(/ENDIF/));
    expect(screen.queryByRole('alertdialog')).toBeNull();
  });
});

