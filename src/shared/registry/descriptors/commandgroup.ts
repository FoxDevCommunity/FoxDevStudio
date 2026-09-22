import type { ControlDescriptor } from '../types';
import * as c from '../common';

export const commandgroup: ControlDescriptor = {
  type: 'CommandGroup',
  baseClass: 'commandgroup',
  displayName: 'Command Group',
  toolboxGroup: 'Standard',
  icon: 'AppsList',
  namePrefix: 'Commandgroup',
  defaultSize: { Width: 84, Height: 60 },
  defaultEvent: 'Click',
  container: { accepts: ['CommandButton'], autoChildren: { type: 'CommandButton', count: 2, countProp: 'ButtonCount' } },
  properties: c.props(
    c.layoutProps(84, 60),
    c.prop('ButtonCount', 'number', 'Layout', 2, { min: 0, max: 99 }),
    c.prop('Value', 'number', 'Data', 1, { min: 0 }),
    c.prop('AutoSize', 'boolean', 'Layout', false),
    c.backStyleProp,
    c.enumProp('BorderStyle', 'Appearance', 1, c.enumValues(['None', 'Fixed Single'])),
    c.specialEffectProp,
    c.memberClassProps,
    c.themesProp,
    c.fontProps,
    c.colorProps(c.COLOR_BUTTONFACE),
    c.containerProps,
    c.prop('Buttons', 'text', 'Other', '', { readOnly: true, hidden: true, description: 'The buttons, by position' }),
    c.behaviorProps,
    c.tabProps,
  ),
  events: c.events(c.baseEvents, c.mouseEvents, c.focusEvents, c.changeEvents, c.dragEvents),
};
