/**
 * Toolbars, their separators, and the OLE controls a Visual FoxPro form holds.
 *
 * None of these can do what they do in VFP: a toolbar docks to the main window, and an OLE
 * control hosts an ActiveX object this runtime has no way to create. They are here so a form
 * that uses them imports whole. A control kept as a box with its properties and methods intact
 * is worth more than one silently dropped, and it is what makes re-exporting possible later.
 */

import type { ControlDescriptor } from '../types';
import * as c from '../common';

export const toolbar: ControlDescriptor = {
  type: 'Toolbar',
  baseClass: 'toolbar',
  displayName: 'Toolbar',
  toolboxGroup: 'Container',
  icon: 'Toolbox',
  namePrefix: 'Toolbar',
  defaultSize: { Width: 240, Height: 28 },
  defaultEvent: 'Init',
  container: { accepts: 'visual' },
  properties: c.props(
    c.layoutProps(240, 28),
    c.prop('Caption', 'text', 'Appearance', 'Toolbar'),
    c.prop('Movable', 'boolean', 'Behavior', true),
    c.prop('Dockable', 'number', 'Behavior', 15, { min: 0, max: 15, description: 'Which edges it may be docked to' }),
    c.prop('Docked', 'boolean', 'Behavior', false, { readOnly: true, hidden: true }),
    c.prop('Panel', 'number', 'Layout', 0, { description: 'Which panel of the window it is docked in.' }),
    c.prop('PanelLink', 'number', 'Layout', 0, { description: 'Which panel it moves to when the window it is in is docked.' }),
    c.enumProp('DockPosition', 'Behavior', -1, [
      { value: -1, label: '-1 - Not docked' },
      { value: 0, label: '0 - Top' },
      { value: 1, label: '1 - Left' },
      { value: 2, label: '2 - Right' },
      { value: 3, label: '3 - Bottom' },
    ]),
    c.containerProps,
    c.colorProps(c.COLOR_BUTTONFACE),
    c.behaviorProps,
  ),
  events: c.events(c.baseEvents, c.mouseEvents, c.dragEvents, [
    c.ev('BeforeDock', 'nLocation'),
    c.ev('AfterDock', ''),
    c.ev('UnDock', ''),
    c.ev('Moved', ''),
    c.ev('Resize', ''),
    c.ev('Activate', ''),
    c.ev('Deactivate', ''),
  ]),
};

/** The gap between groups of buttons on a toolbar. It has a width and nothing else. */
export const separator: ControlDescriptor = {
  type: 'Separator',
  baseClass: 'separator',
  displayName: 'Separator',
  toolboxGroup: 'Other',
  icon: 'DividerTall',
  namePrefix: 'Separator',
  hideInToolbox: true,
  defaultSize: { Width: 8, Height: 24 },
  defaultEvent: 'Init',
  properties: c.props(
    c.prop('Left', 'number', 'Layout', 0),
    c.prop('Top', 'number', 'Layout', 0),
    c.prop('Width', 'number', 'Layout', 8, { min: 0 }),
    c.prop('Height', 'number', 'Layout', 24, { min: 0 }),
    c.prop('Visible', 'boolean', 'Layout', true),
    c.prop('Comment', 'multiline', 'Other', ''),
    c.prop('Tag', 'text', 'Other', ''),
  ),
  events: c.events(c.baseEvents),
};

/**
 * An ActiveX control. `OleClass` records which one, so the form still says what it wanted even
 * though nothing here can create it.
 */
export const olecontrol: ControlDescriptor = {
  type: 'OleControl',
  baseClass: 'olecontrol',
  displayName: 'OLE Control',
  toolboxGroup: 'Other',
  icon: 'PuzzlePiece',
  namePrefix: 'Olecontrol',
  defaultSize: { Width: 100, Height: 100 },
  defaultEvent: 'Init',
  properties: c.props(
    c.layoutProps(100, 100),
    c.prop('OleClass', 'text', 'Other', ''),
    c.enumProp('Align', 'Layout', 0, c.enumValues(['None', 'Align Top', 'Align Bottom', 'Align Left', 'Align Right']), 'Which edge of what holds it the control lines up with.'),
    c.enumProp('AutoActivate', 'Behavior', 1, c.enumValues(['Manual', 'GotFocus', 'DoubleClick', 'Programmatic'])),
    c.prop('AutoVerbMenu', 'boolean', 'Behavior', false),
    c.prop('AutoSize', 'boolean', 'Layout', false),
    c.prop('CLSID', 'text', 'Other', '', { readOnly: true, hidden: true }),
    c.prop('ProgID', 'text', 'Other', '', { readOnly: true, hidden: true }),
    c.prop('DocumentFile', 'text', 'Data', ''),
    c.prop('Object', 'text', 'Other', '', { readOnly: true, hidden: true, description: 'The object inside the control' }),
    c.enumProp('OLETypeAllowed', 'Data', -2, [
      { value: -2, label: '-2 - Any' },
      { value: -1, label: '-1 - None' },
      { value: 0, label: '0 - Linked' },
      { value: 1, label: '1 - Embedded' },
      { value: 3, label: '3 - Control' },
    ]),
    c.prop('OLELCID', 'number', 'Other', 0),
    c.prop('DefOLELCID', 'number', 'Other', 0),
    c.prop('OLERequestPendingTimeout', 'number', 'Behavior', 5000, { min: 0 }),
    c.prop('OLEServerBusyRaiseError', 'boolean', 'Behavior', true),
    c.prop('OLEServerBusyTimeout', 'number', 'Behavior', 10000, { min: 0 }),
    c.prop('SizeBox', 'boolean', 'Appearance', false),
    c.colorProps(c.COLOR_BUTTONFACE),
    c.behaviorProps,
    c.tabProps,
  ),
  events: c.events(c.baseEvents, c.mouseEvents, c.focusEvents, c.dragEvents),
};

/** The same, bound to a General field. The binding waits on the data engine. */
export const oleboundcontrol: ControlDescriptor = {
  type: 'OleBoundControl',
  baseClass: 'oleboundcontrol',
  displayName: 'OLE Bound Control',
  toolboxGroup: 'Other',
  icon: 'PuzzlePiece',
  namePrefix: 'Oleboundcontrol',
  defaultSize: { Width: 100, Height: 100 },
  defaultEvent: 'Init',
  properties: c.props(
    c.layoutProps(100, 100),
    c.prop('OleClass', 'text', 'Other', ''),
    c.enumProp('AutoActivate', 'Behavior', 1, c.enumValues(['Manual', 'GotFocus', 'DoubleClick', 'Programmatic'])),
    c.prop('AutoVerbMenu', 'boolean', 'Behavior', false),
    c.prop('AutoSize', 'boolean', 'Layout', false),
    c.prop('CLSID', 'text', 'Other', '', { readOnly: true, hidden: true }),
    c.prop('ProgID', 'text', 'Other', '', { readOnly: true, hidden: true }),
    c.prop('DocumentFile', 'text', 'Data', ''),
    c.prop('Object', 'text', 'Other', '', { readOnly: true, hidden: true, description: 'The object inside the control' }),
    c.enumProp('OLETypeAllowed', 'Data', -2, [
      { value: -2, label: '-2 - Any' },
      { value: -1, label: '-1 - None' },
      { value: 0, label: '0 - Linked' },
      { value: 1, label: '1 - Embedded' },
      { value: 3, label: '3 - Control' },
    ]),
    c.prop('OLELCID', 'number', 'Other', 0),
    c.prop('DefOLELCID', 'number', 'Other', 0),
    c.prop('OLERequestPendingTimeout', 'number', 'Behavior', 5000, { min: 0 }),
    c.prop('OLEServerBusyRaiseError', 'boolean', 'Behavior', true),
    c.prop('OLEServerBusyTimeout', 'number', 'Behavior', 10000, { min: 0 }),
    c.prop('SizeBox', 'boolean', 'Appearance', false),
    c.prop('ControlSource', 'text', 'Data', ''),
    c.colorProps(c.COLOR_BUTTONFACE),
    c.behaviorProps,
    c.tabProps,
  ),
  events: c.events(c.baseEvents, c.mouseEvents, c.focusEvents, c.dragEvents),
};
