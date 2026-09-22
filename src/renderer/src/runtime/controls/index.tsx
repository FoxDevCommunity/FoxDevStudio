/**
 * Live control renderers, one per VFP base class. Each one draws a single `RuntimeObject`
 * from the properties the wrapper already resolved, and routes DOM events back through
 * `desktop.dispatch`, which is where compiled FoxPro code actually runs.
 *
 * Ported from the Milestone-1 preview renderers: the visuals are unchanged, but state now
 * lives on the object (`obj.set('Value', v, 'interactive')`) instead of a React context, and
 * the event set matches VFP more closely (KeyPress, RightClick, MouseEnter/MouseLeave, and a
 * Valid that can refuse to let go of the focus).
 */

import type { CSSProperties, FC, KeyboardEvent as ReactKeyboardEvent, MouseEvent as ReactMouseEvent, ReactNode } from 'react';
import { useCallback, useMemo, useState } from 'react';
import { Button, Checkbox, Combobox, Dropdown, Input, Label, Option, Radio, RadioGroup, SpinButton, Tab, TabList, Table, TableBody, TableCell, TableHeader, TableHeaderCell, TableRow, Textarea } from '@fluentui/react-components';
import type { ControlNode, ControlType, PropValue } from '@shared/form/schema';
import type { FocusableElement, RuntimeObject } from '@shared/runtime/objectModel';
import type { EventOutcome } from '@shared/runtime/scheduler';
import type { VmValue } from '@shared/runtime/values';
import { colorCss, designSurfaces, fontStyle } from '../../designer/surfaces';
import { pictureProblem, pictureUrl } from '../pictureUrl';
import { useSessionStore } from '../session';
import { ImageList, TreeView } from '@shared/runtime/oleObjects';
import { OleTreeView } from './OleTreeView';

export interface RuntimeProps {
  obj: RuntimeObject;
  /** Already resolved by the caller (`useRuntimeObject`), so renderers never call `resolved()`. */
  props: Record<string, PropValue>;
  renderChildren(parent: RuntimeObject): ReactNode;
}

/**
 * A caption as VFP shows it. `\<` marks the access key: the character after it is underlined
 * and the marker itself is not shown, so `\<Close` reads Close with the C underlined. A doubled
 * `\\<` is a literal backslash and less-than.
 */
export function caption(text: unknown): ReactNode {
  const raw = String(text ?? '');
  if (!raw.includes('\\<')) return raw;
  const LITERAL = '';
  const escaped = raw.split('\\\\<').join(LITERAL);
  const restore = (s: string) => s.split(LITERAL).join('\\<');
  const mark = escaped.indexOf('\\<');
  if (mark < 0) return restore(escaped);
  const before = restore(escaped.slice(0, mark));
  const key = escaped.charAt(mark + 2);
  const after = restore(escaped.slice(mark + 3));
  return (
    <>
      {before}
      <u>{key}</u>
      {after}
    </>
  );
}

const fill: CSSProperties = { width: '100%', height: '100%', minWidth: 0, boxSizing: 'border-box' };

/** VFP's KeyPress nKeyCode: the character code for printable keys, else the legacy key code. */
function keyCodeOf(e: ReactKeyboardEvent): number {
  return e.key.length === 1 ? e.key.charCodeAt(0) : e.keyCode || 0;
}

/** VFP's nShiftAltCtrl bitmask: Shift 1, Ctrl 2, Alt 4. */
function shiftAltCtrl(e: ReactKeyboardEvent): number {
  return (e.shiftKey ? 1 : 0) + (e.ctrlKey ? 2 : 0) + (e.altKey ? 4 : 0);
}

/**
 * VFP asks Valid before it lets the focus go; a handler returning .F. keeps the user in the
 * control and LostFocus never fires. The handler may be asynchronous (it can re-enter the VM),
 * so settle a promise off to the side rather than blocking the blur.
 */
function leaveControl(obj: RuntimeObject): void {
  const outcome = obj.desktop.dispatch(obj, 'Valid');
  if (outcome instanceof Promise) {
    void (async () => finishLeave(obj, await outcome))();
    return;
  }
  finishLeave(obj, outcome);
}

/** What the status bar says while the focus is here: the control's own text, or its Message. */
function enterControl(obj: RuntimeObject): void {
  const said = obj.get('StatusBarText');
  if (typeof said === 'string' && said !== '') {
    obj.desktop.setStatusText(said);
    return;
  }
  const outcome = obj.desktop.dispatch(obj, 'Message');
  if (outcome instanceof Promise) {
    void outcome.then((o) => obj.desktop.setStatusText(typeof o?.value === 'string' ? o.value : ''));
    return;
  }
  obj.desktop.setStatusText(typeof outcome?.value === 'string' ? outcome.value : '');
}

function finishLeave(obj: RuntimeObject, outcome: EventOutcome | null): void {
  if (outcome?.value === false) {
    // FoxPro 2.x said why with ErrorMessage; VFP keeps it for the programs that still do
    void obj.desktop.dispatch(obj, 'ErrorMessage');
    obj.setFocus();
    return;
  }
  void obj.desktop.dispatch(obj, 'LostFocus');
}

interface PointerHandlers {
  onClick(): void;
  onDoubleClick(): void;
}
interface FocusHandlers {
  onFocus(): void;
  onBlur(): void;
}
interface CommonHandlers {
  onKeyDown(e: ReactKeyboardEvent): void;
  onContextMenu(e: ReactMouseEvent): void;
  onMouseEnter(): void;
  onMouseLeave(): void;
}

interface Events {
  /** Click/DblClick; left off widgets whose own change handler already speaks for the click. */
  pointer: PointerHandlers;
  /** GotFocus and the Valid/LostFocus pair; only for controls that can hold the focus. */
  focus: FocusHandlers;
  /** Keyboard and mouse-tracking events, safe on every control. */
  common: CommonHandlers;
  /** Escape hatch for events a renderer raises itself (a list row's Click, a page's Activate). */
  dispatch(event: string, target?: RuntimeObject, args?: VmValue[]): void;
}

/** Maps DOM events onto VFP events for one object. Stable while the object is. */
function useEvents(obj: RuntimeObject): Events {
  return useMemo(() => {
    const dispatch = (event: string, target: RuntimeObject = obj, args?: VmValue[]): void => void target.desktop.dispatch(target, event, args);
    return {
      dispatch,
      pointer: {
        onClick: () => dispatch('Click'),
        onDoubleClick: () => dispatch('DblClick'),
      },
      focus: {
        onFocus: () => {
          dispatch('GotFocus');
          // FoxPro 2.x put what Message answered in the status bar; VFP keeps it beside
          // StatusBarText, which says the same thing without being asked
          enterControl(obj);
        },
        onBlur: () => leaveControl(obj),
      },
      common: {
        onKeyDown: (e: ReactKeyboardEvent) => dispatch('KeyPress', obj, [keyCodeOf(e), shiftAltCtrl(e)]),
        onContextMenu: (e: ReactMouseEvent) => {
          e.preventDefault();
          dispatch('RightClick');
        },
        onMouseEnter: () => dispatch('MouseEnter'),
        onMouseLeave: () => dispatch('MouseLeave'),
      },
    };
  }, [obj]);
}

/**
 * Hands the control's focusable element to the object so `SetFocus()` can reach it. Wrapped in
 * a callback of our own rather than passing `obj.bindElement` straight to `ref`, which would
 * make the whole object look like a ref to React's linter.
 */
function useBind(obj: RuntimeObject): (el: FocusableElement | null) => void {
  return useCallback((el: FocusableElement | null) => obj.bindElement(el), [obj]);
}

const RLabel: FC<RuntimeProps> = ({ obj, props: r }) => {
  const ev = useEvents(obj);
  return (
    <Label style={{ ...fill, ...fontStyle(r), color: colorCss(r['ForeColor']), background: r['BackStyle'] === 0 ? 'transparent' : colorCss(r['BackColor']), display: 'block', textAlign: r['Alignment'] === 1 ? 'right' : r['Alignment'] === 2 ? 'center' : 'left', whiteSpace: r['WordWrap'] ? 'normal' : 'nowrap', overflow: 'hidden' }} {...ev.pointer} {...ev.common}>
      {caption(r['Caption'])}
    </Label>
  );
};

const RTextBox: FC<RuntimeProps> = ({ obj, props: r }) => {
  const ev = useEvents(obj);
  const bind = useBind(obj);
  return (
    <Input
      ref={bind}
      style={fill}
      size="small"
      aria-label={obj.name}
      type={r['PasswordChar'] ? 'password' : 'text'}
      value={String(r['Value'] ?? '')}
      readOnly={r['ReadOnly'] === true}
      disabled={r['Enabled'] === false}
      maxLength={Number(r['MaxLength']) > 0 ? Number(r['MaxLength']) : undefined}
      onChange={(_e, d) => obj.set('Value', d.value, 'interactive')}
      {...ev.pointer}
      {...ev.focus}
      {...ev.common}
    />
  );
};

const REditBox: FC<RuntimeProps> = ({ obj, props: r }) => {
  const ev = useEvents(obj);
  const bind = useBind(obj);
  return (
    <Textarea
      ref={bind}
      style={fill}
      textarea={{ style: { height: '100%' } }}
      aria-label={obj.name}
      value={String(r['Value'] ?? '')}
      readOnly={r['ReadOnly'] === true}
      disabled={r['Enabled'] === false}
      onChange={(_e, d) => obj.set('Value', d.value, 'interactive')}
      {...ev.pointer}
      {...ev.focus}
      {...ev.common}
    />
  );
};

const RCommandButton: FC<RuntimeProps> = ({ obj, props: r }) => {
  const ev = useEvents(obj);
  const bind = useBind(obj);
  return (
    <Button
      ref={bind}
      // a VFP caption never wraps: the button is exactly the size the form gave it
      style={{ ...fill, ...fontStyle(r), padding: '0 4px', minWidth: 0, whiteSpace: 'nowrap', overflow: 'hidden' }}
      appearance={r['Default'] ? 'primary' : 'secondary'} disabled={r['Enabled'] === false} {...ev.pointer} {...ev.focus} {...ev.common}>
      {caption(r['Caption'])}
    </Button>
  );
};

/**
 * A caption is one line unless `WordWrap` says otherwise, which is Visual FoxPro's default and
 * not the browser's: left to wrap, "Wrap around" on a check box becomes two lines and pushes
 * itself out of the control it was measured for.
 */
/**
 * A button of a group draws in its own font where it has one, and in the group's where it does
 * not, which is how a Visual FoxPro option group inherits.
 */
function ownFont(b: RuntimeObject, group: Record<string, PropValue>): Record<string, PropValue> {
  const out: Record<string, PropValue> = { ...group };
  for (const name of ['FontName', 'FontSize', 'FontBold', 'FontItalic', 'FontUnderline', 'FontStrikethru']) {
    const own = b.get(name);
    if (own !== undefined) out[name] = own;
  }
  return out;
}

function captionWrap(r: Record<string, PropValue>): CSSProperties {
  return r['WordWrap'] ? { whiteSpace: 'normal' } : { whiteSpace: 'nowrap', overflow: 'hidden' };
}

const RCheckBox: FC<RuntimeProps> = ({ obj, props: r }) => {
  const ev = useEvents(obj);
  const bind = useBind(obj);
  const v = r['Value'];
  return (
    <Checkbox
      ref={bind}
      style={{ ...fill, ...fontStyle(r) }}
      label={{ children: caption(r['Caption']), style: { ...fontStyle(r), ...captionWrap(r) } }}
      checked={v === true || v === 1}
      disabled={r['Enabled'] === false}
      onChange={(_e, d) => obj.set('Value', !!d.checked, 'interactive')}
      onClick={ev.pointer.onClick}
      {...ev.focus}
      {...ev.common}
    />
  );
};

const ROptionGroup: FC<RuntimeProps> = ({ obj, props: r }) => {
  const ev = useEvents(obj);
  const bind = useBind(obj);
  const buttons = obj.children;
  const value = Number(r['Value'] ?? 0);
  return (
    <RadioGroup
      ref={bind}
      tabIndex={-1}
      aria-label={obj.name}
      style={{ ...fill, ...fontStyle(r), display: 'block', border: r['BorderStyle'] === 0 ? 'none' : '1px solid var(--colorNeutralStroke1)', padding: 2 }}
      value={String(value)}
      disabled={r['Enabled'] === false}
      onChange={(_e, d) => obj.set('Value', Number(d.value), 'interactive')}
      {...ev.focus}
      {...ev.common}
    >
      {buttons.map((b, i) => (
        <Radio key={b.handle} value={String(i + 1)} label={{ children: caption(b.get('Caption')), style: { ...fontStyle(ownFont(b, r)), ...captionWrap({ WordWrap: b.get('WordWrap') ?? false }) } }} onClick={() => ev.dispatch('Click', b)} style={{ position: 'absolute', left: Number(b.get('Left')), top: Number(b.get('Top')) }} />
      ))}
    </RadioGroup>
  );
};

/**
 * What the list shows: its item list, which is where a Value row source (RowSourceType 1) has
 * already been spread out for it and where `AddItem` puts a row. Every other row source needs
 * the data engine and leaves the list empty here.
 */
function listItems(obj: RuntimeObject): string[] {
  return obj.items.map((i) => i.text);
}

/** Choosing a row from a drop-down: the item list and Value both follow the click. */
/**
 * Picking a row.
 *
 * What the list shows is column one; what `Value` becomes is the column `BoundColumn` names,
 * which is how a list shows a person one thing and hands the program another - the Solutions
 * launcher shows a category and its code answers with the category's number.
 */
function selectOption(obj: RuntimeObject, r: Record<string, PropValue>, text: string | undefined): void {
  const shown = text ?? '';
  obj.selectItem(shown);
  const row = obj.items.find((i) => i.text === shown);
  const bound = Number(r['BoundColumn'] ?? 1);
  obj.set('Value', row ? obj.column(row, bound) : shown, 'interactive');
}

/** The item VFP considers current: the selected one, else whatever `Value` names. */
function selectedText(obj: RuntimeObject, r: Record<string, PropValue>): string {
  const selected = obj.items.find((i) => i.selected);
  if (selected) return selected.text;
  // Value holds the bound column, so the row it names is the one whose bound column matches
  const value = String(r['Value'] ?? '');
  const bound = Number(r['BoundColumn'] ?? 1);
  const row = bound > 1 ? obj.items.find((i) => obj.column(i, bound) === value) : undefined;
  return row ? row.text : value;
}

const RComboBox: FC<RuntimeProps> = ({ obj, props: r }) => {
  const ev = useEvents(obj);
  const bind = useBind(obj);
  const items = listItems(obj);
  const value = selectedText(obj, r);
  if (r['Style'] === 2) {
    return (
      <Dropdown ref={bind} style={fill} size="small" aria-label={obj.name} value={value} selectedOptions={[value]} disabled={r['Enabled'] === false} onOptionSelect={(_e, d) => selectOption(obj, r, d.optionValue)} {...ev.focus} {...ev.common}>
        {items.map((it) => (
          <Option key={it} value={it}>
            {it}
          </Option>
        ))}
      </Dropdown>
    );
  }
  return (
    <Combobox ref={bind} style={fill} size="small" aria-label={obj.name} freeform value={value} selectedOptions={[value]} disabled={r['Enabled'] === false} onOptionSelect={(_e, d) => selectOption(obj, r, d.optionValue)} onChange={(e) => obj.set('Value', e.target.value, 'interactive')} {...ev.focus} {...ev.common}>
      {items.map((it) => (
        <Option key={it} value={it}>
          {it}
        </Option>
      ))}
    </Combobox>
  );
};

const RListBox: FC<RuntimeProps> = ({ obj, props: r }) => {
  const ev = useEvents(obj);
  const bind = useBind(obj);
  const items = listItems(obj);
  const value = selectedText(obj, r);
  return (
    <div
      ref={bind}
      role="listbox"
      aria-label={obj.name}
      tabIndex={0}
      // a list has no BackColor of its own in the product: the rows are ItemBackColor and
      // ItemForeColor, and the box behind them is the window colour
      style={{ ...fill, ...fontStyle(r), overflow: 'auto', border: '1px solid var(--colorNeutralStroke1)', background: colorCss(r['ItemBackColor']), color: colorCss(r['ItemForeColor']) }}
      {...ev.focus}
      {...ev.common}
    >
      {items.map((it) => (
        <div
          key={it}
          role="option"
          aria-selected={value === it}
          style={{ padding: '1px 4px', cursor: 'default', background: value === it ? 'var(--colorBrandBackground)' : undefined, color: value === it ? 'var(--colorNeutralForegroundOnBrand)' : undefined }}
          onClick={() => {
            selectOption(obj, r, it);
            ev.dispatch('Click');
          }}
          onDoubleClick={ev.pointer.onDoubleClick}
        >
          {it}
        </div>
      ))}
    </div>
  );
};

const RSpinner: FC<RuntimeProps> = ({ obj, props: r }) => {
  const ev = useEvents(obj);
  const bind = useBind(obj);
  return (
    <SpinButton
      ref={bind}
      style={fill}
      size="small"
      aria-label={obj.name}
      value={Number(r['Value'] ?? 0)}
      step={Number(r['Increment'] ?? 1)}
      min={Number(r['SpinnerLowValue'])}
      max={Number(r['SpinnerHighValue'])}
      disabled={r['Enabled'] === false}
      onChange={(_e, d) => obj.set('Value', d.value ?? (Number(d.displayValue) || 0), 'interactive')}
      {...ev.focus}
      {...ev.common}
    />
  );
};

/** Shape, Line and Image look the same running as they do on the design surface. */
function reuseDesign(type: ControlType): FC<RuntimeProps> {
  const Surface = designSurfaces[type];
  const C: FC<RuntimeProps> = ({ obj, props: r }) => {
    const ev = useEvents(obj);
    return (
      <div style={fill} {...ev.pointer} {...ev.common}>
        <Surface node={obj.node as ControlNode} resolved={r} renderChildren={() => null} />
      </div>
    );
  };
  C.displayName = `Runtime${type}`;
  return C;
}

/** VFP's RotateFlip: the low two bits are quarter turns, bit 2 is a horizontal flip. */
function rotateFlipTransform(value: unknown): string | undefined {
  const n = Number(value ?? 0);
  if (!Number.isFinite(n) || n <= 0) return undefined;
  const parts = [`rotate(${(n % 4) * 90}deg)`];
  if (n >= 4) parts.push('scaleX(-1)');
  return parts.join(' ');
}

/**
 * An Image at runtime shows the file, unlike the designer surface which shows a placeholder.
 * A picture that cannot be loaded says so once in Output rather than leaving an empty box with
 * no explanation, because the usual cause is a path built from HOME(), which points at Visual
 * FoxPro's own installation and cannot resolve here.
 */
const RImage: FC<RuntimeProps> = ({ obj, props: r }) => {
  const ev = useEvents(obj);
  const picture = r['Picture'];
  const url = pictureUrl(picture);
  // the failure is remembered against the URL that caused it, so a new Picture starts clean
  const [failure, setFailure] = useState<{ url: string | null; problem: string } | null>(null);
  const failed = failure?.url === url ? failure.problem : null;

  const stretch = Number(r['Stretch'] ?? 0);
  const fit: CSSProperties['objectFit'] = stretch === 2 ? 'fill' : stretch === 1 ? 'contain' : 'none';
  const frame: CSSProperties = {
    ...fill,
    background: r['BackStyle'] === 0 ? 'transparent' : colorCss(r['BackColor']),
    border: r['BorderStyle'] === 1 ? '1px solid #000' : undefined,
    overflow: 'hidden',
    display: 'flex',
    alignItems: 'center',
    justifyContent: 'center',
  };

  return (
    <div style={frame} {...ev.pointer} {...ev.common}>
      {url && !failed ? (
        <img
          src={url}
          alt={String(r['ToolTipText'] || obj.name)}
          style={{ width: '100%', height: '100%', objectFit: fit, transform: rotateFlipTransform(r['RotateFlip']), transformOrigin: 'center' }}
          onError={() => {
            const problem = pictureProblem(picture);
            setFailure({ url, problem });
            useSessionStore.getState().print({ kind: 'error', text: `${obj.name}: ${problem}` });
          }}
        />
      ) : (
        <span style={{ color: 'var(--colorNeutralForeground3)', fontSize: 11, padding: 4, textAlign: 'center', overflow: 'hidden' }}>
          {failed ?? (picture ? String(picture) : '')}
        </span>
      )}
    </div>
  );
};

const RContainer: FC<RuntimeProps> = ({ obj, props: r, renderChildren }) => {
  const ev = useEvents(obj);
  return (
    <div style={{ ...fill, position: 'relative', border: r['BorderStyle'] === 0 ? 'none' : '1px solid var(--colorNeutralStroke1)', background: r['BackStyle'] === 0 ? 'transparent' : colorCss(r['BackColor']) }} {...ev.pointer} {...ev.common}>
      {renderChildren(obj)}
    </div>
  );
};

const RPageFrame: FC<RuntimeProps> = ({ obj, props: r, renderChildren }) => {
  const ev = useEvents(obj);
  const pages = obj.children;
  // ActivePage lives on the object, so code assigning it switches the visible page too.
  const active = Math.max(0, Number(r['ActivePage'] ?? 1) - 1);
  const page = pages[Math.min(active, pages.length - 1)];
  return (
    <div style={{ ...fill, display: 'flex', flexDirection: 'column', border: r['BorderStyle'] === 0 ? 'none' : '1px solid var(--colorNeutralStroke1)' }} {...ev.common}>
      {r['Tabs'] !== false && (
        <TabList
          size="small"
          selectedValue={page ? String(page.handle) : undefined}
          onTabSelect={(_e, d) => {
            const i = pages.findIndex((p) => String(p.handle) === d.value);
            const selected = pages[i];
            if (!selected) return;
            obj.set('ActivePage', i + 1);
            ev.dispatch('Activate', selected);
          }}
        >
          {pages.map((p) => (
            <Tab key={p.handle} value={String(p.handle)} disabled={p.get('Enabled') === false}>
              {caption(p.get('Caption') ?? p.name)}
            </Tab>
          ))}
        </TabList>
      )}
      {page && (
        <div data-runtime-page={page.handle} style={{ position: 'relative', flex: 1, minHeight: 0, background: colorCss(page.get('BackColor')), overflow: 'hidden' }} onClick={() => ev.dispatch('Click', page)}>
          {renderChildren(page)}
        </div>
      )}
    </div>
  );
};

const RGrid: FC<RuntimeProps> = ({ obj, props: r }) => {
  const ev = useEvents(obj);
  const bind = useBind(obj);
  const columns = obj.children;
  const rows = Math.max(1, Math.floor((Number(r['Height']) - Number(r['HeaderHeight'] ?? 17) - 2) / Number(r['RowHeight'] ?? 17)));
  return (
    <div ref={bind} tabIndex={-1} style={{ ...fill, overflow: 'auto', border: '1px solid var(--colorNeutralStroke1)', background: colorCss(r['BackColor']) }} {...ev.focus} {...ev.common}>
      <Table size="extra-small" aria-label={obj.name} style={{ ...fontStyle(r), minWidth: 0 }}>
        <TableHeader>
          <TableRow>
            {columns.map((col) => {
              const header = col.children.find((c) => c.type === 'Header');
              return (
                <TableHeaderCell key={col.handle} style={{ width: Number(col.get('Width')) }}>
                  {header ? caption(header.get('Caption')) : col.name}
                </TableHeaderCell>
              );
            })}
          </TableRow>
        </TableHeader>
        <TableBody>
          {Array.from({ length: rows }).map((_, i) => (
            <TableRow key={i}>
              {columns.map((col) => (
                <TableCell key={col.handle} />
              ))}
            </TableRow>
          ))}
        </TableBody>
      </Table>
    </div>
  );
};

/** Timer is invisible; Page, Column, Header and OptionButton are drawn by their parent. */
const RNothing: FC<RuntimeProps> = () => null;

/**
 * A control that exists on the form but cannot do what it does in VFP: an ActiveX control, a
 * toolbar separator. It occupies its box so the layout around it is right, and says what it is
 * rather than looking like a bug.
 */
const RPlaceholder: FC<RuntimeProps> = ({ obj, props: r }) => (
  <div
    style={{
      ...fill,
      display: 'flex',
      alignItems: 'center',
      justifyContent: 'center',
      border: '1px dashed var(--colorNeutralStroke2)',
      color: 'var(--colorNeutralForeground3)',
      fontSize: 11,
      overflow: 'hidden',
    }}
    aria-label={obj.name}
  >
    {obj.type === 'Separator' ? '' : String(r['OleClass'] || obj.name)}
  </div>
);

/**
 * An ActiveX control. The Common Controls this runtime provides itself draw as themselves; the
 * rest say which control they are, because nothing here can host an OCX's own window.
 */
const ROleControl: FC<RuntimeProps> = (props) => {
  const control = props.obj.ole;
  if (control instanceof TreeView) return <OleTreeView obj={props.obj} tree={control} />;
  // an ImageList is a bag of pictures other controls draw; VFP shows nothing for it either
  if (control instanceof ImageList) return null;
  return <RPlaceholder {...props} />;
};

/** Live renderers, one per control type, using Fluent UI wherever a real widget exists. */
export const runtimeRenderers: Record<ControlType, FC<RuntimeProps>> = {
  Label: RLabel,
  TextBox: RTextBox,
  EditBox: REditBox,
  CommandButton: RCommandButton,
  CheckBox: RCheckBox,
  OptionGroup: ROptionGroup,
  OptionButton: RNothing,
  ComboBox: RComboBox,
  ListBox: RListBox,
  Spinner: RSpinner,
  Shape: reuseDesign('Shape'),
  Line: reuseDesign('Line'),
  Image: RImage,
  Container: RContainer,
  PageFrame: RPageFrame,
  Page: RNothing,
  Grid: RGrid,
  Column: RNothing,
  Header: RNothing,
  Timer: RNothing,
  // non-visual objects exist and run their methods, but have nothing on screen
  Custom: RNothing,
  Session: RNothing,
  Hyperlink: RNothing,
  Collection: RNothing,
  Toolbar: RContainer,
  Separator: RPlaceholder,
  OleControl: ROleControl,
  OleBoundControl: ROleControl,
  CommandGroup: RContainer,
};
