import type { ControlDescriptor } from '../types';
import * as c from '../common';

export const timer: ControlDescriptor = {
  type: 'Timer',
  baseClass: 'timer',
  displayName: 'Timer',
  toolboxGroup: 'Other',
  icon: 'Timer',
  namePrefix: 'Timer',
  nonVisual: true,
  defaultSize: { Width: 24, Height: 24 },
  defaultEvent: 'Timer',
  properties: c.props(
    c.prop('Left', 'number', 'Layout', 0),
    c.prop('Top', 'number', 'Layout', 0),
    c.prop('Interval', 'number', 'Behavior', 0, { min: 0 }),
    c.prop('Enabled', 'boolean', 'Behavior', true),
    c.prop('Comment', 'multiline', 'Other', ''),
    c.prop('Tag', 'text', 'Other', ''),
  ),
  events: c.events(c.baseEvents, [c.ev('Timer', ''), c.ev('Reset', '')]),
};
