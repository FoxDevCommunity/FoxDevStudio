import { act, fireEvent, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it } from 'vitest';
import { EditorView } from '@codemirror/view';
import { findNode } from '@shared/form/tree';
import { useDocumentsStore } from '@renderer/stores/documentsStore';
import { DocumentArea } from '@renderer/shell/DocumentArea';
import { renderWithProviders } from '../helpers/render';
import { sampleForm } from '../helpers/fixtures';

const view = () => EditorView.findFromDOM(screen.getByTestId('code-editor'))!;
const setText = (text: string) => act(() => view().dispatch({ changes: { from: 0, to: view().state.doc.length, insert: text } }));

beforeEach(() => useDocumentsStore.getState().closeAll());

describe('method editor', () => {
  it('opens a control method, shows the source and writes edits back to the form', async () => {
    const formId = useDocumentsStore.getState().openForm(sampleForm(), '/p/Form1.fxf');
    const store = useDocumentsStore.getState().docs[formId]!;
    if (store.kind !== 'form') throw new Error();
    renderWithProviders(<DocumentArea />);
    act(() => void useDocumentsStore.getState().openMethod(formId, 'Command1', 'Click'));
    expect(await screen.findByRole('tab', { name: 'Form1.Command1.Click' })).toBeInTheDocument();
    expect(view().state.doc.toString()).toBe('WAIT WINDOW "ok"');
    expect(within(screen.getByTestId('method-editor')).getByRole('combobox', { name: 'Procedure' })).toHaveValue('Click');

    setText('RETURN .T.');
    setText('RETURN .F.');
    expect(findNode(store.store.getState().doc.form, 'Command1')!.methods['Click']).toBe('RETURN .F.');
    expect(screen.getByRole('tab', { name: 'Form1.fxf' })).toHaveTextContent('*');
    // the burst is one undo entry once the editor loses focus
    expect(store.store.getState().history.txn).not.toBeNull();
    act(() => void fireEvent.blur(screen.getByRole('textbox', { name: 'Click source' })));
    expect(store.store.getState().history.txn).toBeNull();
    expect(store.store.getState().history.past.map((e) => e.label)).toEqual(['Edit Click']);
    act(() => store.store.getState().undo());
    await waitFor(() => expect(view().state.doc.toString()).toBe('WAIT WINDOW "ok"'));
  });

  it('opening a method whose source came in with CRLF does not mark the form modified', async () => {
    const doc = sampleForm();
    findNode(doc.form, 'Command1')!.methods['Click'] = 'IF .T.\r\n\tWAIT WINDOW "ok"\r\nENDIF';
    const formId = useDocumentsStore.getState().openForm(doc, '/p/Form1.fxf');
    const store = useDocumentsStore.getState().docs[formId]!;
    if (store.kind !== 'form') throw new Error();
    renderWithProviders(<DocumentArea />);
    act(() => void useDocumentsStore.getState().openMethod(formId, 'Command1', 'Click'));
    await screen.findByTestId('code-editor');

    expect(view().state.doc.toString()).toBe('IF .T.\n\tWAIT WINDOW "ok"\nENDIF');
    expect(store.store.getState().history.past).toEqual([]);
    expect(screen.getByRole('tab', { name: 'Form1.fxf' })).not.toHaveTextContent('*');
  });

  it('switches procedure and object, and follows undo from the designer', async () => {
    const formId = useDocumentsStore.getState().openForm(sampleForm(), '/p/Form1.fxf');
    const store = useDocumentsStore.getState().docs[formId]!;
    if (store.kind !== 'form') throw new Error();
    renderWithProviders(<DocumentArea />);
    act(() => void useDocumentsStore.getState().openMethod(formId, 'Command1', 'Click'));
    await screen.findByTestId('method-editor');

    await userEvent.selectOptions(screen.getByRole('combobox', { name: 'Procedure' }), 'Init');
    expect(await screen.findByRole('tab', { name: 'Form1.Command1.Init' })).toBeInTheDocument();
    expect(view().state.doc.toString()).toBe('');
    setText('* init code');
    expect(findNode(store.store.getState().doc.form, 'Command1')!.methods['Init']).toBe('* init code');

    await userEvent.selectOptions(screen.getByRole('combobox', { name: 'Object' }), '$form');
    expect(await screen.findByRole('tab', { name: 'Form1.Init' })).toBeInTheDocument();
    await userEvent.selectOptions(screen.getByRole('combobox', { name: 'Object' }), 'Text1');
    expect(await screen.findByRole('tab', { name: 'Form1.Text1.Init' })).toBeInTheDocument(); // same event kept
    await userEvent.selectOptions(screen.getByRole('combobox', { name: 'Procedure' }), 'Valid');
    await userEvent.selectOptions(screen.getByRole('combobox', { name: 'Object' }), 'Timer1');
    expect(await screen.findByRole('tab', { name: 'Form1.Timer1.Timer' })).toBeInTheDocument(); // timers have no Valid -> default event
    // switching target closed the transaction, so the earlier Init edit is undoable
    expect(store.store.getState().history.txn).toBeNull();
    expect(store.store.getState().history.past.map((e) => e.label)).toEqual(['Edit Init']);
  });

  it('programs open in the code editor and stay in sync with the document store', async () => {
    const id = useDocumentsStore.getState().openProgram('DO x', '/p/main.prg');
    renderWithProviders(<DocumentArea />);
    await screen.findByTestId('code-editor');
    expect(view().state.doc.toString()).toBe('DO x');
    setText('DO y');
    expect(useDocumentsStore.getState().docs[id]).toMatchObject({ text: 'DO y' });
    act(() => useDocumentsStore.getState().setProgramText(id, 'DO z'));
    expect(view().state.doc.toString()).toBe('DO z');
    expect(screen.getByRole('textbox', { name: 'Program source' })).toBeInTheDocument();
  });
});
