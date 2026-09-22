import type { ControlDescriptor } from '../types';
import * as c from '../common';

export const spinner: ControlDescriptor = {
  type: 'Spinner',
  baseClass: 'spinner',
  displayName: 'Spinner',
  toolboxGroup: 'Standard',
  icon: 'ArrowSort',
  namePrefix: 'Spinner',
  defaultSize: { Width: 100, Height: 24 },
  defaultEvent: 'InteractiveChange',
  properties: c.props(
    c.layoutProps(100, 24),
    c.prop('ControlSource', 'expression', 'Data', ''),
    c.prop('Value', 'number', 'Data', 0),
    c.prop('Increment', 'number', 'Behavior', 1),
    c.prop('KeyboardHighValue', 'number', 'Behavior', 2147483647),
    c.prop('KeyboardLowValue', 'number', 'Behavior', -2147483647),
    c.prop('SpinnerHighValue', 'number', 'Behavior', 2147483647),
    c.prop('SpinnerLowValue', 'number', 'Behavior', -2147483647),
    c.prop('InputMask', 'text', 'Data', ''),
    c.alignmentProp(3, true),
    c.borderStyleProp,
    c.specialEffectProp,
    c.themesProp,
    c.fontProps,
    c.colorProps(),
    c.behaviorProps,
    c.tabProps,
  ),
  events: c.events(c.baseEvents, c.mouseEvents, c.focusEvents, c.changeEvents, c.dragEvents, [c.ev('UpClick', ''), c.ev('DownClick', '')]),
};
