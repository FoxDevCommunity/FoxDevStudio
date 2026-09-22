import type { ControlDescriptor } from '../types';
import * as c from '../common';

export const image: ControlDescriptor = {
  type: 'Image',
  baseClass: 'image',
  displayName: 'Image',
  toolboxGroup: 'Standard',
  icon: 'Image',
  namePrefix: 'Image',
  defaultSize: { Width: 100, Height: 100 },
  defaultEvent: 'Click',
  properties: c.props(
    c.layoutProps(100, 100),
    c.prop('Picture', 'picture', 'Appearance', ''),
    c.enumProp('Stretch', 'Appearance', 0, c.enumValues(['Clip', 'Isometric', 'Stretch'])),
    c.backStyleProp,
    c.enumProp('BorderStyle', 'Appearance', 0, c.enumValues(['None', 'Fixed Single'])),
    c.prop('BackColor', 'color', 'Appearance', c.COLOR_WHITE),
    c.enumProp(
      'RotateFlip',
      'Appearance',
      0,
      c.enumValues([
        'None',
        'Rotate 90',
        'Rotate 180',
        'Rotate 270',
        'Flip Horizontal',
        'Rotate 90, Flip Horizontal',
        'Rotate 180, Flip Horizontal',
        'Rotate 270, Flip Horizontal',
      ]),
      'Rotation and/or flip applied to the picture (GDI+ RotateFlipType)',
    ),
    c.themesProp,
    c.prop('PictureVal', 'text', 'Appearance', '', { hidden: true }),
    c.prop('Rotation', 'number', 'Appearance', 0, { min: 0, max: 359 }),
    c.behaviorProps,
  ),
  events: c.events(c.baseEvents, c.mouseEvents, c.dragEvents),
};
