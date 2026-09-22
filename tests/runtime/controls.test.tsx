import { Profiler } from 'react';
import { act, fireEvent, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';
import { CONTROL_TYPES, type ControlNode, type FormDocument } from '@shared/form/schema';
import { getDescriptor, getPropertyMeta } from '@shared/registry';
import { Desktop, type RuntimeObject } from '@shared/runtime/objectModel';
import type { EventOutcome } from '@shared/runtime/scheduler';
import type { VmValue } from '@shared/runtime/values';
import { designSurfaces } from '@renderer/designer/surfaces';
import { runtimeRenderers } from '@renderer/runtime/controls';
import { RuntimeControl } from '@renderer/runtime/RuntimeControl';
import { renderWithProviders } from '../helpers/render';
import { sampleForm } from '../helpers/fixtures';

type DispatchCall = [path: string, event: string, args: VmValue[] | undefined];

function control(id: string, type: ControlNode['type'], props: ControlNode['props'], children?: ControlNode[]): ControlNode {
  return { id, type, name: id, props, methods: {}, children };
}

/** The M1 preview fixture plus one control of every remaining type, so all 21 renderers run. */
function buildDoc(): FormDocument {
  const doc = sampleForm();
  const pageframe = doc.form.children.find((c) => c.id === 'Pageframe1')!;
  pageframe.children!.find((p) => p.id === 'Page2')!.children!.push(control('Label2', 'Label', { Left: 8, Top: 8, Caption: 'Second page' }));
  doc.form.children.push(
    control('Check1', 'CheckBox', { Left: 200, Top: 64, Caption: 'Agree' }),
    control('Combo1', 'ComboBox', { Left: 200, Top: 90, RowSourceType: 1, RowSource: 'Red,Green,Blue', Style: 2 }),
    control('List1', 'ListBox', { Left: 300, Top: 200, RowSourceType: 1, RowSource: 'One,Two' }),
    control('Grid1', 'Grid', { Left: 0, Top: 260, Width: 300, Height: 40 }, [control('Column1', 'Column', {}, [control('Header1', 'Header', { Caption: 'Name' })])]),
    control('Hidden1', 'Label', { Visible: false, Caption: 'secret' }),
    control('Off1', 'CommandButton', { Enabled: false, Caption: 'Disabled', Left: 300, Top: 16 }),
    control('LabelA', 'Label', { Left: 0, Top: 0, Caption: 'Alpha' }),
    control('LabelB', 'Label', { Left: 0, Top: 20, Caption: 'Beta' }),
    control('Edit1', 'EditBox', { Left: 340, Top: 64, Value: 'notes' }),
    control('Spinner1', 'Spinner', { Left: 340, Top: 90, Value: 3, Increment: 2, SpinnerLowValue: 0, SpinnerHighValue: 10 }),
    control('Opt1', 'OptionGroup', { Left: 340, Top: 120 }, [control('Option1', 'OptionButton', { Caption: 'Yes', Left: 0, Top: 0 }), control('Option2', 'OptionButton', { Caption: 'No', Left: 0, Top: 18 })]),
    control('Cmdgroup1', 'CommandGroup', { Left: 340, Top: 180 }, [control('Cmd1', 'CommandButton', { Caption: 'Inner', Left: 0, Top: 0 })]),
    control('Container1', 'Container', { Left: 400, Top: 180 }, [control('Label9', 'Label', { Left: 2, Top: 2, Caption: 'Boxed' })]),
    control('Shape1', 'Shape', { Left: 420, Top: 0, Width: 20, Height: 20 }),
    control('Line1', 'Line', { Left: 440, Top: 0, Width: 0, Height: 30 }),
    control('Image1', 'Image', { Left: 460, Top: 0, Width: 20, Height: 20 }),
  );
  return doc;
}

/** A live desktop with a recording dispatch, rendered as a bare list of top-level controls. */
function setup(outcomes: Record<string, EventOutcome | Promise<EventOutcome>> = {}) {
  const desktop = new Desktop();
  const calls: DispatchCall[] = [];
  desktop.dispatch = (obj, event, args) => {
    calls.push([obj.path(), event, args]);
    return outcomes[`${obj.path()}.${event}`] ?? null;
  };
  const form = desktop.instantiate(buildDoc().form, -1);
  const view = renderWithProviders(<div>{form.children.map((c) => <RuntimeControl key={c.handle} obj={c} />)}</div>);
  const child = (name: string): RuntimeObject => form.child(name)!;
  const fired = (): string[] => calls.map(([path, event]) => `${path}.${event}`);
  return { desktop, form, calls, fired, child, view };
}

describe('runtime controls', () => {
  it('has a renderer and a design surface for every control type', () => {
    for (const t of CONTROL_TYPES) {
      expect(runtimeRenderers[t], t).toBeTypeOf('function');
      expect(designSurfaces[t], t).toBeTypeOf('function');
    }
  });

  it('draws the live object tree, honouring Visible, Enabled and non-visual controls', () => {
    setup();
    expect(screen.getByRole('button', { name: 'OK' })).toBeInTheDocument();
    expect(screen.getByRole('textbox', { name: 'Text1' })).toHaveValue('hello');
    expect(screen.getByRole('textbox', { name: 'Edit1' })).toHaveValue('notes');
    expect(screen.getByRole('tab', { name: 'First' })).toHaveAttribute('aria-selected', 'true');
    expect(screen.getByText('Inside')).toBeInTheDocument();
    expect(screen.getByText('Boxed')).toBeInTheDocument();
    expect(screen.getByRole('columnheader', { name: 'Name' })).toBeInTheDocument();
    expect(screen.getByRole('spinbutton', { name: 'Spinner1' })).toHaveValue('3');
    expect(screen.getByRole('radio', { name: 'Yes' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Disabled' })).toBeDisabled();
    // Visible .F. and non-visual controls occupy no box at all.
    expect(screen.queryByText('secret')).toBeNull();
    expect(document.querySelector('[data-runtime-name="Hidden1"]')).toBeNull();
    expect(document.querySelector('[data-runtime-name="Timer1"]')).toBeNull();
    expect(document.querySelector('[data-runtime-name="Command1"]')).toHaveStyle({ position: 'absolute', left: '16px', top: '16px' });
  });

  // Measured in Visual FoxPro 9 on a ListBox with RowSourceType 1 and RowSource "One,Two":
  // the row source is the list the box starts with, AddItem puts a row into that same list
  // (at the position it names), RemoveItem takes one out, and Clear empties it without
  // touching RowSource. Writing RowSource, or RowSourceType, builds the list again from the
  // string, which throws away what AddItem had put there.
  it('adds a row to the list the row source built', async () => {
    const { child } = setup();
    const list = child('List1');
    expect(screen.getByRole('option', { name: 'One' })).toBeInTheDocument();

    list.addItem('Apples');
    list.addItem('Oranges');
    list.addItem('Bananas', 1);
    await screen.findByRole('option', { name: 'Bananas' });
    expect(screen.getAllByRole('option').map((o) => o.textContent)).toEqual(['Bananas', 'One', 'Two', 'Apples', 'Oranges']);

    list.setElement('Selected', [2], true);
    await waitFor(() => expect(screen.getByRole('option', { name: 'One' })).toHaveAttribute('aria-selected', 'true'));

    list.removeItem(1);
    await waitFor(() => expect(screen.queryByRole('option', { name: 'Bananas' })).toBeNull());
    expect(screen.getAllByRole('option').map((o) => o.textContent)).toEqual(['One', 'Two', 'Apples', 'Oranges']);
    list.clearItems();
    await waitFor(() => expect(screen.queryAllByRole('option')).toHaveLength(0));
    // the string is what the list was built from, not a picture of what it holds
    expect(list.get('RowSource')).toBe('One,Two');
    list.set('RowSource', 'One,Two');
    await screen.findByRole('option', { name: 'One' });
  });

  it('draws a control added while the form is running', async () => {
    const { desktop, form } = setup();
    await desktop.callMethod(form.child('Container1')!.handle, 'AddObject', ['Late1', 'commandbutton']);
    const added = form.child('Container1')!.child('Late1')!;
    added.set('Caption', 'Added');
    // VFP adds the object hidden so the code that follows can place it first
    expect(screen.queryByRole('button', { name: 'Added' })).toBeNull();
    added.set('Visible', true);
    expect(await screen.findByRole('button', { name: 'Added' })).toBeInTheDocument();
  });

  it('dispatches Click through the desktop', async () => {
    const { fired } = setup();
    await userEvent.click(screen.getByRole('button', { name: 'OK' }));
    expect(fired()).toContain('Command1.Click');
    expect(fired()).toContain('Command1.GotFocus');
  });

  it('writes typing back to the object as an interactive change', async () => {
    const { child, fired } = setup();
    const text1 = child('Text1');
    const set = vi.spyOn(text1, 'set');
    const box = screen.getByRole('textbox', { name: 'Text1' });
    await userEvent.clear(box);
    await userEvent.type(box, 'abc');
    expect(text1.get('Value')).toBe('abc');
    expect(box).toHaveValue('abc');
    expect(set).toHaveBeenLastCalledWith('Value', 'abc', 'interactive');
    expect(fired()).toContain('Text1.InteractiveChange');
  });

  it('toggles a check box', async () => {
    const { child } = setup();
    const check = screen.getByRole('checkbox', { name: 'Agree' });
    await userEvent.click(check);
    expect(check).toBeChecked();
    expect(child('Check1').get('Value')).toBe(true);
  });

  it('selects a list box row: sets the value, then fires Click', async () => {
    const { child, fired } = setup();
    await userEvent.click(screen.getByRole('option', { name: 'Two' }));
    expect(child('List1').get('Value')).toBe('Two');
    expect(screen.getByRole('option', { name: 'Two' })).toHaveAttribute('aria-selected', 'true');
    expect(fired().filter((e) => e.startsWith('List1.'))).toEqual(expect.arrayContaining(['List1.InteractiveChange', 'List1.Click']));
    expect(fired().indexOf('List1.InteractiveChange')).toBeLessThan(fired().indexOf('List1.Click'));
  });

  it('sets ActivePage and activates the page when a tab is clicked', async () => {
    const { child, fired } = setup();
    await userEvent.click(screen.getByRole('tab', { name: 'Second' }));
    expect(child('Pageframe1').get('ActivePage')).toBe(2);
    expect(fired()).toContain('Pageframe1.Page2.Activate');
    expect(screen.getByText('Second page')).toBeInTheDocument();
    expect(screen.queryByText('Inside')).toBeNull();
  });

  it('switches the visible page when code assigns ActivePage', () => {
    const { child } = setup();
    expect(screen.getByText('Inside')).toBeInTheDocument();
    act(() => child('Pageframe1').set('ActivePage', 2));
    expect(screen.getByText('Second page')).toBeInTheDocument();
    expect(screen.queryByText('Inside')).toBeNull();
  });

  it('re-renders only the object whose property changed', () => {
    const desktop = new Desktop();
    const form = desktop.instantiate(buildDoc().form, -1);
    const a = form.child('LabelA')!;
    const b = form.child('LabelB')!;
    const counts = { a: 0, b: 0 };
    renderWithProviders(
      <div>
        <Profiler id="a" onRender={() => void counts.a++}>
          <RuntimeControl obj={a} />
        </Profiler>
        <Profiler id="b" onRender={() => void counts.b++}>
          <RuntimeControl obj={b} />
        </Profiler>
      </div>,
    );
    const mounted = { ...counts };
    act(() => a.set('Caption', 'Changed'));
    expect(screen.getByText('Changed')).toBeInTheDocument();
    expect(screen.getByText('Beta')).toBeInTheDocument();
    expect(counts.a).toBe(mounted.a + 1);
    expect(counts.b).toBe(mounted.b);
  });

  it('keeps the focus when Valid returns .F., and releases it otherwise', async () => {
    const refused = setup({ 'Text1.Valid': { value: false, nodefault: false } });
    const stay = vi.spyOn(refused.child('Text1'), 'setFocus');
    await userEvent.click(screen.getByRole('textbox', { name: 'Text1' }));
    await userEvent.tab();
    expect(refused.fired()).toContain('Text1.Valid');
    expect(stay).toHaveBeenCalled();
    expect(refused.fired()).not.toContain('Text1.LostFocus');

    refused.view.unmount();
    const allowed = setup();
    const go = vi.spyOn(allowed.child('Text1'), 'setFocus');
    await userEvent.click(screen.getByRole('textbox', { name: 'Text1' }));
    await userEvent.tab();
    expect(go).not.toHaveBeenCalled();
    expect(allowed.fired()).toContain('Text1.LostFocus');
  });

  it('awaits an asynchronous Valid before deciding', async () => {
    const { child, fired } = setup({ 'Text1.Valid': Promise.resolve({ value: false, nodefault: false }) });
    const stay = vi.spyOn(child('Text1'), 'setFocus');
    await userEvent.click(screen.getByRole('textbox', { name: 'Text1' }));
    await userEvent.tab();
    await act(async () => await Promise.resolve());
    expect(stay).toHaveBeenCalled();
    expect(fired()).not.toContain('Text1.LostFocus');
  });

  it('reports KeyPress as [keyCode, shiftAltCtrl] and RightClick on a context menu', () => {
    const { calls, fired } = setup();
    const box = screen.getByRole('textbox', { name: 'Text1' });
    fireEvent.keyDown(box, { key: 'a', shiftKey: true, ctrlKey: true });
    fireEvent.keyDown(box, { key: 'Enter', keyCode: 13 });
    fireEvent.contextMenu(box);
    const keys = calls.filter(([path, event]) => path === 'Text1' && event === 'KeyPress').map(([, , args]) => args);
    expect(keys).toEqual([
      [97, 3],
      [13, 0],
    ]);
    expect(fired()).toContain('Text1.RightClick');
  });

  it('tracks the mouse over a control', async () => {
    const { fired } = setup();
    const ok = screen.getByRole('button', { name: 'OK' });
    await userEvent.hover(ok);
    await userEvent.unhover(ok);
    expect(fired()).toContain('Command1.MouseEnter');
    expect(fired()).toContain('Command1.MouseLeave');
  });
});

describe('a caption that would not fit', () => {
  it('stays on one line unless the control asks to wrap', () => {
    const desktop = new Desktop();
    const node = {
      name: 'Form1',
      props: {},
      methods: {},
      children: [
        control('chkTight', 'CheckBox', { Caption: 'Wrap around', Width: 40 }),
        control('chkWrapped', 'CheckBox', { Caption: 'Wrap around', Width: 40, WordWrap: true }),
      ],
    };
    const form = desktop.instantiate(node as never, -1);
    const view = renderWithProviders(
      <div>
        {form.children.map((c) => (
          <RuntimeControl key={c.handle} obj={c} />
        ))}
      </div>,
    );

    const labels = [...view.container.querySelectorAll('label span, .fui-Checkbox__label')] as HTMLElement[];
    const wraps = labels.map((l) => l.style.whiteSpace).filter(Boolean);
    // VFP's WordWrap default is false: a caption is one line and is clipped, not reflowed
    expect(wraps).toEqual(['nowrap', 'normal']);
  });

  it('asks for a font that exists in place of one Windows dropped', async () => {
    const { fontFamilyCss } = await import('@renderer/designer/surfaces');
    expect(fontFamilyCss('MS Sans Serif')).toContain('Microsoft Sans Serif');
    expect(fontFamilyCss('Tahoma')).toContain('"Tahoma"');
  });
});

describe('AutoSize', () => {
  it('lets a control be as wide as what is written on it, never narrower than its stored size', () => {
    const desktop = new Desktop();
    const node = {
      name: 'Form1',
      props: {},
      methods: {},
      children: [
        control('chkGrows', 'CheckBox', { Caption: 'Wrap around', Width: 80, Height: 15, AutoSize: true }),
        control('chkFixed', 'CheckBox', { Caption: 'Wrap around', Width: 80, Height: 15 }),
      ],
    };
    const form = desktop.instantiate(node as never, -1);
    const view = renderWithProviders(
      <div>
        {form.children.map((c) => (
          <RuntimeControl key={c.handle} obj={c} />
        ))}
      </div>,
    );
    const box = (name: string) => view.container.querySelector(`[data-runtime-name="${name}"]`) as HTMLElement;

    expect(box('chkGrows').style.width).toBe('max-content');
    expect(box('chkGrows').style.minWidth).toBe('80px');
    expect(box('chkFixed').style.width).toBe('80px');
  });
});

describe('RightToLeft', () => {
  /**
   * A container must not turn its contents around. `RightToLeft` says which way the text on the
   * control itself reads; CSS `direction` cascades, where the property does not, so a container
   * that passes it on mirrors everything placed inside it. A bare PageFrame answers .T. in an
   * ordinary English installation - the product was asked - so a form with a page frame on it is
   * the ordinary case, not an exotic one, and that is how every form on the page frame came to
   * be drawn back to front.
   */
  it('turns a control around and never what it contains', () => {
    // if this ever stops being true the test below proves nothing, so it is asserted here
    expect(getPropertyMeta(getDescriptor('PageFrame'), 'RightToLeft')?.default).toBe(true);

    const desktop = new Desktop();
    const node = {
      name: 'Form1',
      props: {},
      methods: {},
      children: [
        control('pgf', 'PageFrame', { PageCount: 1 }, [control('Page1', 'Page', { Caption: 'One' }, [control('lblInside', 'Label', { Caption: 'Inside' })])]),
        control('txtMirrored', 'TextBox', { RightToLeft: true, Value: 'ltr' }),
      ],
    };
    const form = desktop.instantiate(node as never, -1);
    const view = renderWithProviders(
      <div>
        {form.children.map((c) => (
          <RuntimeControl key={c.handle} obj={c} />
        ))}
      </div>,
    );
    const box = (name: string) => view.container.querySelector(`[data-runtime-name="${name}"]`) as HTMLElement;

    expect(box('pgf').style.direction).toBe('');
    expect(box('lblInside').style.direction).toBe('');
    // a control that is not a container still reads the way it says it does
    expect(box('txtMirrored').style.direction).toBe('rtl');
  });

  it('leaves every control the product ships at its defaults reading left to right', () => {
    const { view } = setup();
    for (const el of [...view.container.querySelectorAll('[data-runtime-name]')] as HTMLElement[]) {
      expect(el.style.direction, el.getAttribute('data-runtime-name') ?? '').not.toBe('rtl');
    }
  });
});

describe('a list with more than one column', () => {
  /**
   * `AddListItem(cItem, nItemID, nColumn)` fills one column of one row. The Solutions launcher
   * builds its filter this way - the category in column one, its number in column two - and
   * treating each call as a row of its own put the numbers between the names on screen and made
   * every second row unusable. What is shown is column one; what `Value` becomes is the column
   * `BoundColumn` names.
   */
  it('keeps a row together and answers with its bound column', async () => {
    const desktop = new Desktop();
    const node = {
      name: 'Form1',
      props: {},
      methods: {},
      children: [control('cboFilter', 'ComboBox', { Left: 0, Top: 0, Width: 200, Height: 23, Style: 2, ColumnCount: 2, BoundColumn: 2, ColumnWidths: '180,0' })],
    };
    const form = desktop.instantiate(node as never, -1);
    const combo = form.child('cboFilter')!;
    combo.addItem('All', 1, 1, true);
    combo.addItem('-1', 1, 2, true);
    combo.addItem('ActiveX', 2, 1, true);
    combo.addItem('20', 2, 2, true);

    expect(combo.items.map((i) => i.text)).toEqual(['All', 'ActiveX']);
    expect(combo.column(combo.items[1]!, 2)).toBe('20');

    const view = renderWithProviders(
      <div>
        {form.children.map((c) => (
          <RuntimeControl key={c.handle} obj={c} />
        ))}
      </div>,
    );
    await userEvent.click(view.getByRole('combobox'));
    expect(screen.getAllByRole('option').map((o) => o.textContent)).toEqual(['All', 'ActiveX']);

    await userEvent.click(screen.getByRole('option', { name: 'ActiveX' }));
    // the person picked ActiveX; the program is handed the number beside it
    expect(combo.get('Value')).toBe('20');
  });

  it('leaves a list the colour of its items, never black', () => {
    const desktop = new Desktop();
    const node = { name: 'Form1', props: {}, methods: {}, children: [control('lst', 'ListBox', { Left: 0, Top: 0, Width: 200, Height: 100 })] };
    const form = desktop.instantiate(node as never, -1);
    const view = renderWithProviders(
      <div>
        {form.children.map((c) => (
          <RuntimeControl key={c.handle} obj={c} />
        ))}
      </div>,
    );
    // a ListBox has no BackColor in the product - the rows are ItemBackColor - and an absent
    // colour was being painted black, which is what turned every list into a black rectangle
    const box = view.container.querySelector('[role="listbox"]') as HTMLElement;
    expect(box.style.background).not.toBe('rgb(0, 0, 0)');
    expect(box.style.background).not.toBe('#000000');
  });
});
