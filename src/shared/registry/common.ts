import { rgb } from '../form/color';
import type { PropValue } from '../form/schema';
import type { EnumValue, EventMeta, PropCategory, PropEditor, PropertyMeta } from './types';

export function prop(
  name: string,
  editor: PropEditor,
  category: PropCategory,
  def: PropValue,
  extra: Partial<Omit<PropertyMeta, 'name' | 'editor' | 'category' | 'default'>> = {},
): PropertyMeta {
  return { name, editor, category, default: def, ...extra };
}

export function enumProp(name: string, category: PropCategory, def: number | string, values: EnumValue[], description?: string): PropertyMeta {
  return { name, editor: 'enum', category, default: def, enumValues: values, description };
}

export function ev(name: string, params?: string, description?: string): EventMeta {
  return params === undefined ? { name, description } : { name, params, description };
}

export function enumValues(labels: string[], start = 0): EnumValue[] {
  return labels.map((label, i) => ({ value: start + i, label: `${start + i} - ${label}` }));
}

// ---- VFP default colors ----
export const COLOR_BLACK = rgb(0, 0, 0);
export const COLOR_WHITE = rgb(255, 255, 255);
export const COLOR_BUTTONFACE = rgb(236, 233, 216);
export const COLOR_DISABLED = rgb(128, 128, 128);
/** Windows "Selected Items" colours: VFP derives the SelectedItem and Highlight defaults from these. */
export const COLOR_HIGHLIGHT = rgb(0, 0, 128);
export const COLOR_HIGHLIGHT_TEXT = COLOR_WHITE;

// ---- shared property sets ----
export function layoutProps(width: number, height: number): PropertyMeta[] {
  return [
    prop('Left', 'number', 'Layout', 0),
    prop('Top', 'number', 'Layout', 0),
    prop('Width', 'number', 'Layout', width, { min: 0 }),
    prop('Height', 'number', 'Layout', height, { min: 0 }),
  ];
}

export const fontProps: PropertyMeta[] = [
  prop('FontName', 'font', 'Appearance', 'Arial'),
  prop('FontSize', 'number', 'Appearance', 9, { min: 1, max: 128 }),
  prop('FontBold', 'boolean', 'Appearance', false),
  prop('FontItalic', 'boolean', 'Appearance', false),
  prop('FontUnderline', 'boolean', 'Appearance', false),
  prop('FontStrikethru', 'boolean', 'Appearance', false),
];

export function colorProps(back: number = COLOR_WHITE, fore: number = COLOR_BLACK): PropertyMeta[] {
  return [
    prop('BackColor', 'color', 'Appearance', back),
    prop('ForeColor', 'color', 'Appearance', fore),
    prop('DisabledBackColor', 'color', 'Appearance', back),
    prop('DisabledForeColor', 'color', 'Appearance', COLOR_DISABLED),
  ];
}

export const behaviorProps: PropertyMeta[] = [
  prop('Enabled', 'boolean', 'Behavior', true),
  prop('Visible', 'boolean', 'Behavior', true),
  prop('ToolTipText', 'text', 'Behavior', ''),
  prop('MousePointer', 'number', 'Behavior', 0, { min: 0, max: 99 }),
  prop('HelpContextID', 'number', 'Other', 0),
  prop('Comment', 'multiline', 'Other', ''),
  prop('Tag', 'text', 'Other', ''),
];

export const tabProps: PropertyMeta[] = [
  prop('TabIndex', 'number', 'Behavior', 0, { readOnly: true }),
  prop('TabStop', 'boolean', 'Behavior', true),
];

export const backStyleProp = enumProp('BackStyle', 'Appearance', 1, enumValues(['Transparent', 'Opaque']));
export const borderStyleProp = enumProp('BorderStyle', 'Appearance', 1, enumValues(['None', 'Fixed Single']));
export const specialEffectProp = enumProp('SpecialEffect', 'Appearance', 0, enumValues(['3D', 'Plain']));
export const alignmentProp = (def: number, automatic = false) =>
  enumProp('Alignment', 'Appearance', def, enumValues(automatic ? ['Left', 'Right', 'Center', 'Automatic'] : ['Left', 'Right', 'Center']));

/** VFP 8 visual themes. Declared only on the controls the Themes property applies to. */
export const themesProp = prop('Themes', 'boolean', 'Appearance', true);

/** CommandButton, CheckBox and OptionButton: the picture's position relative to the caption. */
export const picturePositionProp = enumProp(
  'PicturePosition',
  'Appearance',
  13,
  enumValues([
    'Left Top',
    'Left Center',
    'Left Bottom',
    'Right Top',
    'Right Center',
    'Right Bottom',
    'Above Left',
    'Above Center',
    'Above Right',
    'Below Left',
    'Below Center',
    'Below Right',
    'Center Top',
    'Center Center',
    'No Text',
  ]),
);

/** Class used for the members a container creates itself: Grid columns, PageFrame pages, group buttons. */
export const memberClassProps: PropertyMeta[] = [
  prop('MemberClass', 'text', 'Other', ''),
  prop('MemberClassLibrary', 'text', 'Other', ''),
];

/** ComboBox, ListBox and Grid: colours of the selected item (the selected cell in a grid). */
export const selectedItemColorProps: PropertyMeta[] = [
  prop('SelectedItemBackColor', 'color', 'Appearance', COLOR_HIGHLIGHT),
  prop('SelectedItemForeColor', 'color', 'Appearance', COLOR_HIGHLIGHT_TEXT),
];

/** TextBox and EditBox: colours of the selected text. */
export const selectedTextColorProps: PropertyMeta[] = [
  prop('SelectedBackColor', 'color', 'Appearance', COLOR_HIGHLIGHT),
  prop('SelectedForeColor', 'color', 'Appearance', COLOR_HIGHLIGHT_TEXT),
];

export const dataProps: PropertyMeta[] = [
  prop('ControlSource', 'expression', 'Data', ''),
  prop('Value', 'text', 'Data', ''),
];

/**
 * What every object answers about itself. Visual FoxPro keeps these out of the property sheet -
 * a program reads them, the designer does not set them - so they are hidden rather than absent.
 */
export const identityProps: PropertyMeta[] = [
  prop('Name', 'text', 'Other', '', { readOnly: true, hidden: true, description: 'The name the object answers to' }),
  prop('Class', 'text', 'Other', '', { readOnly: true, hidden: true }),
  prop('BaseClass', 'text', 'Other', '', { readOnly: true, hidden: true }),
  prop('ParentClass', 'text', 'Other', '', { readOnly: true, hidden: true }),
  prop('ClassLibrary', 'text', 'Other', '', { readOnly: true, hidden: true }),
  prop('Application', 'text', 'Other', '', { readOnly: true, hidden: true, description: 'The application object' }),
  prop('Parent', 'text', 'Other', '', { readOnly: true, hidden: true, description: 'The object this one is in' }),
  prop('hWnd', 'number', 'Other', 0, { readOnly: true, hidden: true }),
];

/**
 * Dragging, dropping and the way a control follows its form when the form is resized. The OLE
 * ones say what a control does with a drag that comes from outside the application, which this
 * runtime has nothing to receive; the anchor is what moves a control when its form changes size.
 */
export const dragDropProps: PropertyMeta[] = [
  prop('Anchor', 'number', 'Layout', 0, {
    min: 0,
    description: 'Which edges of the parent the control keeps its distance from: 1 top, 2 left, 4 bottom, 8 right',
  }),
  prop('DragIcon', 'picture', 'Behavior', ''),
  enumProp('DragMode', 'Behavior', 0, enumValues(['Manual', 'Automatic'])),
  prop('MouseIcon', 'picture', 'Behavior', ''),
  enumProp('OLEDragMode', 'Behavior', 0, enumValues(['Manual', 'Automatic'])),
  prop('OLEDragPicture', 'picture', 'Behavior', ''),
  prop('OLEDropEffects', 'number', 'Behavior', 3),
  prop('OLEDropHasData', 'number', 'Behavior', -1, { hidden: true }),
  enumProp('OLEDropMode', 'Behavior', 0, enumValues(['None', 'Enabled', 'Manual'])),
  enumProp('OLEDropTextInsertion', 'Behavior', 1, enumValues(['Overwrite', 'Insert'])),
  prop('RightToLeft', 'boolean', 'Appearance', false),
  prop('WhatsThisHelpID', 'number', 'Other', -1),
];

/** What a container answers about what is inside it. */
export const containerProps: PropertyMeta[] = [
  prop('ControlCount', 'number', 'Other', 0, { readOnly: true, hidden: true }),
  prop('Controls', 'text', 'Other', '', { readOnly: true, hidden: true }),
  prop('Objects', 'text', 'Other', '', { readOnly: true, hidden: true }),
  prop('Count', 'number', 'Other', 0, { readOnly: true, hidden: true }),
];

/**
 * What a control that edits text keeps about the selection inside it, and how it reads a date
 * as it is typed. The selection ones are what the running control has, not what the designer
 * sets, so they are hidden.
 */
export const editingProps: PropertyMeta[] = [
  prop('SelStart', 'number', 'Data', 0, { hidden: true }),
  prop('SelLength', 'number', 'Data', 0, { hidden: true }),
  prop('SelText', 'text', 'Data', '', { hidden: true }),
  prop('Text', 'text', 'Data', '', { readOnly: true, hidden: true, description: 'What the control shows, as text' }),
  prop('HideSelection', 'boolean', 'Behavior', true),
  enumProp('IMEMode', 'Behavior', 0, enumValues(['Not Set', 'On', 'Off', 'Disabled', 'Hiragana', 'Katakana'])),
  prop('NullDisplay', 'text', 'Appearance', ''),
];

/** How a date or a time is read as it is typed. */
export const dateEntryProps: PropertyMeta[] = [
  enumProp('Century', 'Data', 2, enumValues(['Off', 'On', 'Set by SET CENTURY'])),
  enumProp('DateFormat', 'Data', 0, [
    { value: 0, label: '0 - By SET DATE' },
    { value: 1, label: '1 - American' },
    { value: 2, label: '2 - ANSI' },
    { value: 3, label: '3 - British/French' },
    { value: 4, label: '4 - German' },
  ]),
  prop('DateMark', 'text', 'Data', ''),
  enumProp('Hours', 'Data', 0, enumValues(['By SET HOURS', '12', '24'])),
  enumProp('Seconds', 'Data', 0, enumValues(['By SET SECONDS', 'Show', 'Hide'])),
  enumProp('StrictDateEntry', 'Data', 1, enumValues(['Loose', 'Strict'])),
];

/** A list of items: what is in it, which of it is showing, and which of it is chosen. */
export const listProps: PropertyMeta[] = [
  prop('List', 'text', 'Data', '', { hidden: true, description: 'The items, by position' }),
  prop('ListCount', 'number', 'Data', 0, { readOnly: true, hidden: true }),
  prop('ListIndex', 'number', 'Data', 0, { hidden: true }),
  prop('ListItem', 'text', 'Data', '', { hidden: true, description: 'The items, by item id' }),
  prop('ListItemID', 'number', 'Data', 0, { hidden: true }),
  prop('ItemData', 'number', 'Data', 0, { hidden: true }),
  prop('ItemIDData', 'number', 'Data', 0, { hidden: true }),
  prop('NewIndex', 'number', 'Data', 0, { readOnly: true, hidden: true }),
  prop('NewItemID', 'number', 'Data', 0, { readOnly: true, hidden: true }),
  prop('Selected', 'boolean', 'Data', false, { hidden: true }),
  prop('SelectedID', 'boolean', 'Data', false, { hidden: true }),
  prop('TopIndex', 'number', 'Data', 0, { hidden: true }),
  prop('TopItemID', 'number', 'Data', 0, { hidden: true }),
  prop('DisplayCount', 'number', 'Data', 0),
  prop('FirstElement', 'number', 'Data', 1, { min: 1 }),
  prop('ColumnLines', 'boolean', 'Appearance', true),
  prop('ColumnWidths', 'text', 'Appearance', ''),
  prop('ItemTips', 'boolean', 'Behavior', false),
  prop('IntegralHeight', 'boolean', 'Layout', false),
  prop('MoverBars', 'boolean', 'Behavior', false),
  enumProp('KeySort', 'Behavior', 0, enumValues(['By item id', 'By index'])),
  prop('BoundTo', 'boolean', 'Data', false),
  prop('AutoComplete', 'number', 'Behavior', 0, { min: 0, max: 2 }),
  prop('AutoCompSource', 'text', 'Behavior', ''),
  prop('AutoCompTable', 'text', 'Behavior', ''),
  prop('Text', 'text', 'Data', '', { readOnly: true, hidden: true }),
];

/** The graphics settings a form and the shapes on it draw with. */
export const drawingProps: PropertyMeta[] = [
  enumProp('DrawMode', 'Appearance', 13, [
    { value: 1, label: '1 - Blackness' },
    { value: 7, label: '7 - Xor Pen' },
    { value: 11, label: '11 - Nop' },
    { value: 13, label: '13 - Copy Pen' },
    { value: 16, label: '16 - Whiteness' },
  ]),
  enumProp('DrawStyle', 'Appearance', 0, enumValues(['Solid', 'Dash', 'Dot', 'Dash-Dot', 'Dash-Dot-Dot', 'Transparent', 'Inside Solid'])),
  prop('DrawWidth', 'number', 'Appearance', 1, { min: 1 }),
];

/** The font settings beyond the six every object has. */
export const fontExtraProps: PropertyMeta[] = [
  prop('FontCharSet', 'number', 'Appearance', 1),
  prop('FontOutline', 'boolean', 'Appearance', false),
  prop('FontShadow', 'boolean', 'Appearance', false),
];

// ---- shared event sets ----
export const baseEvents: EventMeta[] = [
  ev('Init', '', 'Occurs when the object is created'),
  ev('Destroy', '', 'Occurs when the object is released'),
  ev('Error', 'nError, cMethod, nLine'),
  ev('Refresh', ''),
  ev('UIEnable', 'lEnable'),
];

export const mouseEvents: EventMeta[] = [
  ev('Click', ''),
  ev('DblClick', ''),
  ev('RightClick', ''),
  ev('MiddleClick', ''),
  ev('MouseDown', 'nButton, nShift, nXCoord, nYCoord'),
  ev('MouseUp', 'nButton, nShift, nXCoord, nYCoord'),
  ev('MouseMove', 'nButton, nShift, nXCoord, nYCoord'),
  ev('MouseEnter', 'nButton, nShift, nXCoord, nYCoord'),
  ev('MouseLeave', 'nButton, nShift, nXCoord, nYCoord'),
  ev('MouseWheel', 'nDirection, nShift, nXCoord, nYCoord'),
];

export const focusEvents: EventMeta[] = [
  ev('GotFocus', ''),
  ev('LostFocus', ''),
  ev('When', ''),
  ev('Valid', ''),
  ev('KeyPress', 'nKeyCode, nShiftAltCtrl'),
  // what FoxPro 2.x used before there were StatusBarText and Valid; VFP keeps both for the
  // programs that still say them
  ev('Message', ''),
  ev('ErrorMessage', ''),
];

export const changeEvents: EventMeta[] = [ev('InteractiveChange', ''), ev('ProgrammaticChange', '')];

export const dragEvents: EventMeta[] = [
  ev('DragDrop', 'oSource, nXCoord, nYCoord'),
  ev('DragOver', 'oSource, nXCoord, nYCoord, nState'),
];

/** A drag that comes from outside the application, which the object model reports as it would. */
export const oleDragEvents: EventMeta[] = [
  ev('OLEStartDrag', 'oDataObject, nEffect'),
  ev('OLEDragOver', 'oDataObject, nEffect, nButton, nShift, nXCoord, nYCoord, nState'),
  ev('OLEGiveFeedback', 'nEffect, eMouseCursor'),
  ev('OLESetData', 'oDataObject, eFormat'),
  ev('OLECompleteDrag', 'nEffect'),
  ev('OLEDragDrop', 'oDataObject, nEffect, nButton, nShift, nXCoord, nYCoord'),
];

export function events(...sets: EventMeta[][]): EventMeta[] {
  const seen = new Set<string>();
  const out: EventMeta[] = [];
  for (const set of sets) {
    for (const e of set) {
      if (seen.has(e.name)) continue;
      seen.add(e.name);
      out.push(e);
    }
  }
  return out.sort((a, b) => a.name.localeCompare(b.name));
}

export function props(...sets: (PropertyMeta | PropertyMeta[])[]): PropertyMeta[] {
  const seen = new Set<string>();
  const out: PropertyMeta[] = [];
  for (const set of sets) {
    for (const p of Array.isArray(set) ? set : [set]) {
      if (seen.has(p.name)) continue;
      seen.add(p.name);
      out.push(p);
    }
  }
  return out.sort((a, b) => a.name.localeCompare(b.name));
}
