import type { ControlDescriptor } from '../types';
import * as c from '../common';
import { rowSourceTypeProp } from './combobox';

export const listbox: ControlDescriptor = {
  type: 'ListBox',
  baseClass: 'listbox',
  displayName: 'List Box',
  toolboxGroup: 'Standard',
  icon: 'TextBulletList',
  namePrefix: 'List',
  defaultSize: { Width: 100, Height: 100 },
  defaultEvent: 'InteractiveChange',
  properties: c.props(
    c.layoutProps(100, 100),
    c.dataProps,
    rowSourceTypeProp,
    c.prop('RowSource', 'expression', 'Data', ''),
    c.prop('BoundColumn', 'number', 'Data', 1, { min: 1 }),
    c.prop('ColumnCount', 'number', 'Data', 0, { min: 0 }),
    c.prop('MultiSelect', 'boolean', 'Behavior', false),
    c.prop('IncrementalSearch', 'boolean', 'Behavior', true),
    c.prop('NumberOfElements', 'number', 'Data', 0, { min: 0 }),
    c.prop('Sorted', 'boolean', 'Data', false),
    c.prop('ReadOnly', 'boolean', 'Behavior', false),
    c.prop('Picture', 'picture', 'Appearance', ''),
    c.selectedItemColorProps,
    c.themesProp,
    c.borderStyleProp,
    c.specialEffectProp,
    c.listProps,
    c.fontProps,
    c.fontExtraProps,
    c.colorProps(),
    c.behaviorProps,
    c.tabProps,
  ),
  events: c.events(c.baseEvents, c.mouseEvents, c.focusEvents, c.changeEvents, c.dragEvents, [
    // a list whose items can be moved says so when one is
    c.ev('OnMoveItem', 'nSource, nShift, nCurrentIndex, nMoveBy'),
  ]),
};
