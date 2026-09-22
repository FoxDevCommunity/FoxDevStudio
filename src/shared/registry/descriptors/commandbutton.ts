import type { ControlDescriptor } from '../types';
import * as c from '../common';

export const commandbutton: ControlDescriptor = {
  type: 'CommandButton',
  baseClass: 'commandbutton',
  displayName: 'Command Button',
  toolboxGroup: 'Standard',
  icon: 'Button',
  namePrefix: 'Command',
  defaultSize: { Width: 84, Height: 27 },
  defaultEvent: 'Click',
  properties: c.props(
    c.layoutProps(84, 27),
    c.prop('Caption', 'text', 'Appearance', 'Command1'),
    c.prop('Default', 'boolean', 'Behavior', false),
    c.prop('Cancel', 'boolean', 'Behavior', false),
    c.prop('Picture', 'picture', 'Appearance', ''),
    c.prop('DownPicture', 'picture', 'Appearance', ''),
    c.prop('DisabledPicture', 'picture', 'Appearance', ''),
    c.picturePositionProp,
    c.themesProp,
    c.enumProp('SpecialEffect', 'Appearance', 0, c.enumValues(['3D', 'Plain', 'Hot Tracking'])),
    c.prop('StatusBarText', 'text', 'Behavior', ''),
    c.fontProps,
    c.colorProps(c.COLOR_BUTTONFACE),
    c.prop('PictureMargin', 'number', 'Appearance', 0, { min: 0 }),
    c.prop('PictureSpacing', 'number', 'Appearance', 0, { min: 0 }),
    c.prop('PictureVal', 'text', 'Appearance', '', { hidden: true, description: 'The picture as bytes rather than a file' }),
    c.behaviorProps,
    c.tabProps,
  ),
  events: c.events(c.baseEvents, c.mouseEvents, c.focusEvents, c.dragEvents),
};
