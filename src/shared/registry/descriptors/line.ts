import type { ControlDescriptor } from '../types';
import * as c from '../common';

export const line: ControlDescriptor = {
  type: 'Line',
  baseClass: 'line',
  displayName: 'Line',
  toolboxGroup: 'Standard',
  icon: 'Line',
  namePrefix: 'Line',
  fixedAspect: 'line',
  defaultSize: { Width: 100, Height: 0 },
  defaultEvent: 'Click',
  properties: c.props(
    c.layoutProps(100, 0),
    c.prop('BorderWidth', 'number', 'Appearance', 1, { min: 0, max: 8192 }),
    c.prop('BorderColor', 'color', 'Appearance', c.COLOR_BLACK),
    c.enumProp('BorderStyle', 'Appearance', 1, c.enumValues(['Transparent', 'Solid', 'Dash', 'Dot', 'Dash-Dot', 'Dash-Dot-Dot', 'Inside Solid'])),
    c.enumProp('LineSlant', 'Appearance', '\\', [
      { value: '\\', label: '\\ - Backslash' },
      { value: '/', label: '/ - Slash' },
    ]),
    c.drawingProps,
    c.behaviorProps,
  ),
  events: c.events(c.baseEvents, c.mouseEvents, c.dragEvents),
};
