import type { ControlDescriptor } from '../types';
import * as c from '../common';

export const rowSourceTypeProp = c.enumProp(
  'RowSourceType',
  'Data',
  0,
  c.enumValues(['None', 'Value', 'Alias', 'SQL Statement', 'Query (.qpr)', 'Array', 'Fields', 'Files', 'Structure', 'Popup', 'Collection']),
);

export const combobox: ControlDescriptor = {
  type: 'ComboBox',
  baseClass: 'combobox',
  displayName: 'Combo Box',
  toolboxGroup: 'Standard',
  icon: 'ChevronDown',
  namePrefix: 'Combo',
  defaultSize: { Width: 100, Height: 24 },
  defaultEvent: 'InteractiveChange',
  properties: c.props(
    c.layoutProps(100, 24),
    c.dataProps,
    rowSourceTypeProp,
    c.prop('RowSource', 'expression', 'Data', ''),
    c.prop('BoundColumn', 'number', 'Data', 1, { min: 1 }),
    c.prop('ColumnCount', 'number', 'Data', 0, { min: 0 }),
    c.prop('DisplayValue', 'text', 'Data', ''),
    c.enumProp('Style', 'Appearance', 0, [
      { value: 0, label: '0 - Dropdown Combo' },
      { value: 2, label: '2 - Dropdown List' },
    ]),
    c.prop('IncrementalSearch', 'boolean', 'Behavior', true),
    c.prop('NumberOfElements', 'number', 'Data', 0, { min: 0 }),
    c.prop('Sorted', 'boolean', 'Data', false),
    c.prop('ReadOnly', 'boolean', 'Behavior', false),
    c.prop('Picture', 'picture', 'Appearance', ''),
    c.enumProp(
      'PictureSelectionDisplay',
      'Appearance',
      0,
      c.enumValues(['None', 'Clip', 'Isometric', 'Stretch']),
      "How the item's picture is displayed in the text box portion of the combo box",
    ),
    c.selectedItemColorProps,
    c.themesProp,
    c.borderStyleProp,
    c.specialEffectProp,
    c.listProps,
    c.editingProps,
    c.fontProps,
    c.fontExtraProps,
    c.colorProps(),
    c.behaviorProps,
    c.tabProps,
  ),
  events: c.events(c.baseEvents, c.mouseEvents, c.focusEvents, c.changeEvents, c.dragEvents, [c.ev('DropDown', '')]),
};
