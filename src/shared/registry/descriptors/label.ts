import type { ControlDescriptor } from '../types';
import * as c from '../common';

export const label: ControlDescriptor = {
  type: 'Label',
  baseClass: 'label',
  displayName: 'Label',
  toolboxGroup: 'Standard',
  icon: 'TextT',
  namePrefix: 'Label',
  defaultSize: { Width: 100, Height: 17 },
  defaultEvent: 'Click',
  properties: c.props(
    c.layoutProps(100, 17),
    c.prop('Caption', 'text', 'Appearance', 'Label1'),
    c.alignmentProp(0),
    c.prop('AutoSize', 'boolean', 'Layout', false),
    c.prop('WordWrap', 'boolean', 'Appearance', false),
    c.backStyleProp,
    c.enumProp('BorderStyle', 'Appearance', 0, c.enumValues(['None', 'Fixed Single'])),
    c.fontProps,
    c.colorProps(c.COLOR_BUTTONFACE),
    c.behaviorProps,
  ),
  events: c.events(c.baseEvents, c.mouseEvents, c.dragEvents),
};
