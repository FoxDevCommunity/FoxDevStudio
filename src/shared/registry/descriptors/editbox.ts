import type { ControlDescriptor } from '../types';
import * as c from '../common';

export const editbox: ControlDescriptor = {
  type: 'EditBox',
  baseClass: 'editbox',
  displayName: 'Edit Box',
  toolboxGroup: 'Standard',
  icon: 'TextBulletListSquare',
  namePrefix: 'Edit',
  defaultSize: { Width: 200, Height: 100 },
  defaultEvent: 'InteractiveChange',
  properties: c.props(
    c.layoutProps(200, 100),
    c.dataProps,
    c.prop('MaxLength', 'number', 'Data', 0, { min: 0 }),
    c.prop('ReadOnly', 'boolean', 'Behavior', false),
    c.prop('AllowTabs', 'boolean', 'Behavior', false),
    c.prop('AutoHideScrollBar', 'boolean', 'Appearance', false, {
      description: 'Whether the scroll bar is hidden until there is something to scroll.',
    }),
    c.enumProp('ScrollBars', 'Appearance', 2, [
      { value: 0, label: '0 - None' },
      { value: 2, label: '2 - Vertical' },
    ]),
    c.alignmentProp(0),
    c.borderStyleProp,
    c.specialEffectProp,
    c.prop('Margin', 'number', 'Appearance', 2, { min: 0 }),
    c.prop('SelectOnEntry', 'boolean', 'Behavior', false),
    c.prop('EnableHyperlinks', 'boolean', 'Behavior', false, { description: 'Displays URLs in the text as clickable hyperlinks' }),
    c.editingProps,
    c.prop('MemoWindow', 'text', 'Behavior', ''),
    c.prop('HScrollSmallChange', 'number', 'Behavior', 10, { min: 0 }),
    c.prop('VScrollSmallChange', 'number', 'Behavior', 10, { min: 0 }),
    c.fontProps,
    c.fontExtraProps,
    c.colorProps(),
    c.selectedTextColorProps,
    c.behaviorProps,
    c.tabProps,
  ),
  events: c.events(c.baseEvents, c.mouseEvents, c.focusEvents, c.changeEvents, c.dragEvents),
};
