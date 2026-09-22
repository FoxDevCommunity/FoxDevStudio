import type { ControlDescriptor } from '../types';
import * as c from '../common';

export const shape: ControlDescriptor = {
  type: 'Shape',
  baseClass: 'shape',
  displayName: 'Shape',
  toolboxGroup: 'Standard',
  icon: 'Square',
  namePrefix: 'Shape',
  defaultSize: { Width: 100, Height: 100 },
  defaultEvent: 'Click',
  properties: c.props(
    c.layoutProps(100, 100),
    c.prop('Curvature', 'number', 'Appearance', 0, { min: 0, max: 99 }),
    c.prop('FillColor', 'color', 'Appearance', c.COLOR_WHITE),
    c.enumProp('FillStyle', 'Appearance', 1, c.enumValues(['Solid', 'Transparent', 'Horizontal Line', 'Vertical Line', 'Upward Diagonal', 'Downward Diagonal', 'Cross', 'Diagonal Cross'])),
    c.prop('BorderWidth', 'number', 'Appearance', 1, { min: 0, max: 8192 }),
    c.prop('BorderColor', 'color', 'Appearance', c.COLOR_BLACK),
    c.enumProp('BorderStyle', 'Appearance', 1, c.enumValues(['Transparent', 'Solid', 'Dash', 'Dot', 'Dash-Dot', 'Dash-Dot-Dot', 'Inside Solid'])),
    c.backStyleProp,
    c.specialEffectProp,
    c.prop('BackColor', 'color', 'Appearance', c.COLOR_WHITE),
    c.drawingProps,
    c.prop('PolyPoints', 'number', 'Appearance', 0, { min: 0, description: 'Sides of the polygon a shape is drawn as' }),
    c.prop('Rotation', 'number', 'Appearance', 0, { min: 0, max: 359 }),
    c.behaviorProps,
  ),
  events: c.events(c.baseEvents, c.mouseEvents, c.dragEvents),
};
