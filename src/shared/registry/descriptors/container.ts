import type { ControlDescriptor } from '../types';
import * as c from '../common';

export const container: ControlDescriptor = {
  type: 'Container',
  baseClass: 'container',
  displayName: 'Container',
  toolboxGroup: 'Container',
  icon: 'Group',
  namePrefix: 'Container',
  defaultSize: { Width: 100, Height: 100 },
  defaultEvent: 'Click',
  container: { accepts: 'visual' },
  properties: c.props(
    c.layoutProps(100, 100),
    c.backStyleProp,
    c.enumProp('BorderStyle', 'Appearance', 1, c.enumValues(['None', 'Fixed Single'])),
    c.prop('BorderWidth', 'number', 'Appearance', 1, { min: 0 }),
    c.enumProp('SpecialEffect', 'Appearance', 0, c.enumValues(['Raised', 'Sunken', 'Flat'])),
    c.prop('Picture', 'picture', 'Appearance', ''),
    c.colorProps(c.COLOR_BUTTONFACE),
    c.containerProps,
    c.behaviorProps,
    c.tabProps,
  ),
  events: c.events(c.baseEvents, c.mouseEvents, c.dragEvents),
};
