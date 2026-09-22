import { act, fireEvent, screen, within } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import userEvent from '@testing-library/user-event';
import { findNode } from '@shared/form/tree';
import { getRect } from '@renderer/designer/store/createFormDesignerStore';
import { useDocumentsStore } from '@renderer/stores/documentsStore';
import { canvas, client, control, handle, renderDesigner } from '../helpers/designer';
import { click, drag, pointerDown, pointerMove, pointerUp } from '../helpers/pointer';

describe('designer canvas', () => {
  it('renders the form window with its controls at VFP coordinates', () => {
    renderDesigner();
    expect(screen.getByTestId('form-window')).toHaveTextContent('Sample');
    expect(client()).toHaveStyle({ width: '400px', height: '300px' });
    expect(control('Command1')).toHaveStyle({ left: '16px', top: '16px', width: '84px', height: '27px' });
    expect(control('Command1')).toHaveTextContent('OK');
    expect(control('Text1')).toHaveTextContent('hello');
    // nested: the active page shows its children, tabs show captions
    expect(within(control('Pageframe1')).getByRole('tab', { name: 'First' })).toHaveAttribute('aria-selected', 'true');
    expect(control('Label1')).toHaveTextContent('Inside');
    expect(control('Timer1')).toBeInTheDocument();
  });

  it('selects on click, multi-selects with shift, and selects the form on background click', () => {
    const { store } = renderDesigner();
    click(control('Command1'), { x: 20, y: 20 });
    expect(store.getState().selection).toEqual(['Command1']);
    expect(control('Command1')).toHaveClass('fx-selected');
    expect(handle('Command1', 'se')).toBeInTheDocument();
    click(control('Command2'), { x: 130, y: 20 }, { shiftKey: true });
    expect(store.getState().selection).toEqual(['Command1', 'Command2']);
    click(control('Command1'), { x: 20, y: 20 }, { shiftKey: true });
    expect(store.getState().selection).toEqual(['Command2']);
    click(client(), { x: 300, y: 250 });
    expect(store.getState().selection).toEqual([]);
    expect(control('Command2')).not.toHaveClass('fx-selected');
  });

  it('moves a control by dragging, snapping to the grid, as one undo step', () => {
    const { store } = renderDesigner();
    pointerDown(control('Command1'), { x: 20, y: 20 });
    pointerMove({ x: 43, y: 37 });
    expect(store.getState().drag?.kind).toBe('move');
    expect(control('Command1')).toHaveStyle({ left: '40px', top: '32px' }); // live preview
    pointerUp({ x: 43, y: 37 });
    expect(store.getState().drag).toBeNull();
    expect(getRect(findNode(store.getState().doc.form, 'Command1')!)).toMatchObject({ left: 40, top: 32 });
    expect(store.getState().history.past.map((e) => e.label)).toEqual(['Move Command1']);
    act(() => store.getState().undo());
    expect(control('Command1')).toHaveStyle({ left: '16px', top: '16px' });
  });

  it('moves a multi-selection together and ignores tiny jitters', () => {
    const { store } = renderDesigner();
    click(control('Command1'), { x: 20, y: 20 });
    drag(control('Command2'), { x: 130, y: 20 }, { x: 130, y: 52 }, { shiftKey: true });
    // shift-click added Command2 then dragged both down 32px
    expect(getRect(findNode(store.getState().doc.form, 'Command1')!).top).toBe(48);
    expect(getRect(findNode(store.getState().doc.form, 'Command2')!).top).toBe(48);
    drag(control('Command1'), { x: 20, y: 52 }, { x: 21, y: 53 });
    expect(getRect(findNode(store.getState().doc.form, 'Command1')!).top).toBe(48);
  });

  it('resizes with a handle', () => {
    const { store } = renderDesigner();
    click(control('Text1'), { x: 20, y: 70 });
    drag(handle('Text1', 'se'), { x: 216, y: 87 }, { x: 250, y: 100 });
    expect(getRect(findNode(store.getState().doc.form, 'Text1')!)).toEqual({ left: 16, top: 64, width: 232, height: 40 });
    drag(handle('Text1', 'nw'), { x: 16, y: 64 }, { x: 0, y: 40 });
    expect(getRect(findNode(store.getState().doc.form, 'Text1')!)).toEqual({ left: 0, top: 40, width: 248, height: 64 });
  });

  it('marquee-selects top-level controls', () => {
    const { store } = renderDesigner();
    pointerDown(client(), { x: 5, y: 5 });
    pointerMove({ x: 150, y: 90 });
    expect(screen.getByTestId('marquee')).toBeInTheDocument();
    pointerUp({ x: 150, y: 90 });
    expect(store.getState().selection).toEqual(['Command1', 'Command2', 'Text1']);
    expect(screen.queryByTestId('marquee')).toBeNull();
  });

  it('places a control from the toolbox by clicking, or by dragging a size', async () => {
    const { store } = renderDesigner();
    await userEvent.click(screen.getByRole('button', { name: 'Text Box' }));
    expect(store.getState().tool).toBe('TextBox');
    click(client(), { x: 43, y: 200 });
    const added = store.getState().selection[0]!;
    expect(findNode(store.getState().doc.form, added)).toMatchObject({ type: 'TextBox', name: 'Text2', props: { Left: 40, Top: 200 } });
    expect(store.getState().tool).toBeNull();

    await userEvent.click(screen.getByRole('button', { name: 'Shape' }));
    drag(client(), { x: 200, y: 200 }, { x: 290, y: 260 });
    const shape = store.getState().selection[0]!;
    expect(findNode(store.getState().doc.form, shape)!.props).toMatchObject({ Left: 200, Top: 200, Width: 88, Height: 64 });

    // into a page: the click target is the page client
    await userEvent.click(screen.getByRole('button', { name: 'Check Box' }));
    click(document.querySelector('[data-container-id="Page1"]')!, { x: 10, y: 10 });
    const chk = store.getState().selection[0]!;
    expect(findNode(store.getState().doc.form, 'Page1')!.children!.map((c) => c.id)).toContain(chk);
    expect(findNode(store.getState().doc.form, chk)!.name).toBe('Check1');
  });

  it('adds at a free spot when a toolbox button is double-clicked', async () => {
    const { store } = renderDesigner();
    await userEvent.dblClick(screen.getByRole('button', { name: 'Command Button' }));
    const id = store.getState().selection[0]!;
    expect(findNode(store.getState().doc.form, id)!.props).toMatchObject({ Left: 8, Top: 285 }); // below the page frame (100+150)+8
  });

  it('handles keyboard: nudge, resize, delete, select all, undo/redo, clipboard, escape', async () => {
    const { store } = renderDesigner();
    click(control('Command1'), { x: 20, y: 20 });
    canvas().focus();
    fireEvent.keyDown(canvas(), { key: 'ArrowRight' });
    fireEvent.keyDown(canvas(), { key: 'ArrowDown', ctrlKey: true });
    expect(getRect(findNode(store.getState().doc.form, 'Command1')!)).toMatchObject({ left: 17, top: 24 });
    fireEvent.keyDown(canvas(), { key: 'ArrowRight', shiftKey: true });
    expect(getRect(findNode(store.getState().doc.form, 'Command1')!).width).toBe(85);
    fireEvent.keyDown(canvas(), { key: 'z', ctrlKey: true });
    fireEvent.keyDown(canvas(), { key: 'z', ctrlKey: true });
    fireEvent.keyDown(canvas(), { key: 'z', ctrlKey: true });
    expect(getRect(findNode(store.getState().doc.form, 'Command1')!)).toMatchObject({ left: 16, top: 16, width: 84 });
    fireEvent.keyDown(canvas(), { key: 'y', ctrlKey: true });
    expect(getRect(findNode(store.getState().doc.form, 'Command1')!).left).toBe(17);

    fireEvent.keyDown(canvas(), { key: 'c', ctrlKey: true });
    fireEvent.keyDown(canvas(), { key: 'v', ctrlKey: true });
    expect(control('n1')).toHaveTextContent('OK');
    fireEvent.keyDown(canvas(), { key: 'Delete' });
    expect(findNode(store.getState().doc.form, 'n1')).toBeUndefined();

    fireEvent.keyDown(canvas(), { key: 'a', ctrlKey: true });
    expect(store.getState().selection).toHaveLength(5);
    fireEvent.keyDown(canvas(), { key: 'Escape' });
    expect(store.getState().selection).toEqual([]);
  });

  it('double-click opens the default method; page tabs select the page', () => {
    useDocumentsStore.getState().closeAll();
    renderDesigner();
    fireEvent.doubleClick(control('Command1'));
    let docs = Object.values(useDocumentsStore.getState().docs);
    expect(docs[0]).toMatchObject({ kind: 'method', formDocId: 'doc1', controlId: 'Command1', method: 'Click' });
    fireEvent.doubleClick(client());
    docs = Object.values(useDocumentsStore.getState().docs);
    expect(docs[1]).toMatchObject({ kind: 'method', controlId: '$form', method: 'Init' });

    const { store } = renderDesigner();
    pointerDown(screen.getAllByRole('tab', { name: 'Second' })[1]!, { x: 0, y: 0 });
    expect(store.getState().selection).toEqual(['Page2']);
    expect(store.getState().activePages['Pageframe1']).toBe(1);
  });
});
