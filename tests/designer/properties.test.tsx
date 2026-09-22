import { act, fireEvent, screen, within } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import userEvent from '@testing-library/user-event';
import { findNode } from '@shared/form/tree';
import { useDocumentsStore } from '@renderer/stores/documentsStore';
import { control, renderDesigner } from '../helpers/designer';
import { click } from '../helpers/pointer';

const row = (name: string) => document.querySelector<HTMLElement>(`[data-prop="${name}"]`)!;
const props = () => screen.getByTestId('properties-window');

describe('properties window', () => {
  it('shows the form when nothing is selected and the control after selecting it', () => {
    const { store } = renderDesigner();
    expect(within(props()).getByRole('combobox', { name: 'Object' })).toHaveValue('$form');
    expect(within(row('Caption')).getByRole('textbox')).toHaveValue('Sample');
    expect(row('Caption').querySelector('.fx-prop-name')).toHaveClass('fx-changed');
    expect(row('BorderStyle').querySelector('.fx-prop-name')).not.toHaveClass('fx-changed');
    click(control('Command1'), { x: 20, y: 20 });
    expect(within(props()).getByRole('combobox', { name: 'Object' })).toHaveValue('Command1');
    expect(within(row('Caption')).getByRole('textbox')).toHaveValue('OK');
    expect(within(row('Name')).getByRole('textbox')).toHaveValue('Command1');
    // the object dropdown drives the selection too
    fireEvent.change(within(props()).getByRole('combobox', { name: 'Object' }), { target: { value: 'Text1' } });
    expect(store.getState().selection).toEqual(['Text1']);
  });

  it('edits text, number, boolean, enum and color properties', async () => {
    const { store } = renderDesigner();
    const node = () => findNode(store.getState().doc.form, 'Command1')!;
    click(control('Command1'), { x: 20, y: 20 });

    const caption = within(row('Caption')).getByRole('textbox');
    await userEvent.clear(caption);
    await userEvent.type(caption, 'Go{Enter}');
    expect(node().props['Caption']).toBe('Go');
    expect(control('Command1')).toHaveTextContent('Go');

    const width = within(row('Width')).getByRole('spinbutton');
    await userEvent.clear(width);
    await userEvent.type(width, '120');
    fireEvent.blur(width);
    expect(node().props['Width']).toBe(120);
    expect(control('Command1')).toHaveStyle({ width: '120px' });

    await userEvent.selectOptions(within(row('Enabled')).getByRole('combobox'), 'F');
    expect(node().props['Enabled']).toBe(false);
    await userEvent.selectOptions(within(row('SpecialEffect')).getByRole('combobox'), '2');
    expect(node().props['SpecialEffect']).toBe(2);

    const color = within(row('ForeColor')).getByRole('textbox');
    await userEvent.clear(color);
    await userEvent.type(color, 'RGB(255,0,0){Enter}');
    expect(node().props['ForeColor']).toBe(255);
    expect(within(row('ForeColor')).getByRole('textbox')).toHaveValue('RGB(255,0,0)');
    fireEvent.change(within(row('BackColor')).getByLabelText('BackColor swatch'), { target: { value: '#0000ff' } });
    expect(node().props['BackColor']).toBe(16711680);

    // invalid input reverts, Escape reverts
    await userEvent.clear(color);
    await userEvent.type(color, 'purple{Enter}');
    expect(node().props['ForeColor']).toBe(255);
    expect(color).toHaveValue('RGB(255,0,0)');
    await userEvent.type(caption, 'zzz{Escape}');
    expect(caption).toHaveValue('Go');
  });

  it('renames through the Name row and rejects clashes', async () => {
    const { store } = renderDesigner();
    click(control('Command1'), { x: 20, y: 20 });
    const name = within(row('Name')).getByRole('textbox');
    await userEvent.clear(name);
    await userEvent.type(name, 'cmdOk{Enter}');
    expect(findNode(store.getState().doc.form, 'Command1')!.name).toBe('cmdOk');
    await userEvent.clear(name);
    await userEvent.type(name, 'Command2{Enter}');
    expect(findNode(store.getState().doc.form, 'Command1')!.name).toBe('cmdOk');
  });

  it('edits a multi-selection: common properties only, mixed values blank, one undo step', async () => {
    const { store } = renderDesigner();
    act(() => store.getState().select(['Command1', 'Text1']));
    expect(within(props()).getByRole('combobox', { name: 'Object' })).toHaveValue('');
    expect(row('Name')).toBeNull();
    expect(row('Default')).toBeNull(); // buttons only
    expect(row('Left')).not.toBeNull();
    expect(within(row('Left')).getByRole('spinbutton')).toHaveValue(16);
    expect(within(row('Width')).getByRole('spinbutton')).toHaveValue(null); // 84 vs 200
    const left = within(row('Left')).getByRole('spinbutton');
    await userEvent.clear(left);
    await userEvent.type(left, '40{Enter}');
    expect(findNode(store.getState().doc.form, 'Command1')!.props['Left']).toBe(40);
    expect(findNode(store.getState().doc.form, 'Text1')!.props['Left']).toBe(40);
    const before = store.getState().history.past.length;
    act(() => store.getState().undo());
    expect(store.getState().history.past.length).toBe(before - 1);
    expect(findNode(store.getState().doc.form, 'Text1')!.props['Left']).toBe(16);
  });

  it('filters by tab and text; Methods tab lists events and opens code on double-click', async () => {
    useDocumentsStore.getState().closeAll();
    renderDesigner();
    click(control('Command1'), { x: 20, y: 20 });
    await userEvent.click(within(props()).getByRole('tab', { name: 'Data' }));
    expect(row('Caption')).toBeNull();
    await userEvent.click(within(props()).getByRole('tab', { name: 'Layout' }));
    expect(row('Caption')).not.toBeNull();
    expect(row('Enabled')).toBeNull();
    await userEvent.click(within(props()).getByRole('tab', { name: 'All' }));
    await userEvent.type(within(props()).getByRole('textbox', { name: 'Filter properties' }), 'font');
    expect(row('FontName')).not.toBeNull();
    expect(row('Caption')).toBeNull();

    await userEvent.click(within(props()).getByRole('tab', { name: 'Methods' }));
    const clickRow = document.querySelector('[data-event="Click"]')!;
    expect(clickRow).toHaveTextContent('[User Procedure]');
    // a CommandButton has no DblClick in the product, so the list must not show one
    expect(document.querySelector('[data-event="DblClick"]')).toBeNull();
    expect(document.querySelector('[data-event="RightClick"]')).not.toHaveTextContent('[User Procedure]');
    fireEvent.doubleClick(document.querySelector('[data-event="Init"]')!);
    expect(Object.values(useDocumentsStore.getState().docs)[0]).toMatchObject({ kind: 'method', controlId: 'Command1', method: 'Init' });
  });
});
