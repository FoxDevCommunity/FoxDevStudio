import type { ControlNode, ControlType, FormNode, PropValue } from '../form/schema';
import type { ControlDescriptor, EventMeta, ObjectDescriptor, PropertyMeta } from './types';
import * as c from './common';
import * as d from './descriptors';
import { BASE_CLASS_MEMBERS } from './baseClassMembers';

export type { ControlDescriptor, ObjectDescriptor, PropertyMeta, EventMeta, EnumValue, PropEditor, PropCategory, ToolboxGroup } from './types';
export { BASE_CLASS_MEMBERS } from './baseClassMembers';

/**
 * What every object has, whatever it is: what it says about itself, how it takes a drag, and
 * how it follows the form it is on. Every descriptor is given these, so each file lists only
 * what belongs to its own control.
 */
function withUniversal<T extends ObjectDescriptor>(descriptor: T): T {
  return {
    ...descriptor,
    properties: c.props(descriptor.properties, c.identityProps, c.dragDropProps),
    events: c.events(descriptor.events, c.oleDragEvents),
  };
}

/** How a property is best edited when nothing here says anything about it. */
function guessEditor(value: PropValue): PropertyMeta['editor'] {
  if (typeof value === 'boolean') return 'boolean';
  if (typeof value === 'number') return 'number';
  return 'text';
}

/**
 * The metadata to show a property with, once the product has said what the property holds.
 *
 * An editor our descriptor names is kept only while it suits the value that is really there: a
 * CheckBox's Value was written down as a logical and the product holds a number in it, and a
 * tick box over a number is worse than no opinion at all.
 */
function suits(meta: PropertyMeta, value: PropValue): boolean {
  if (meta.editor === 'boolean') return typeof value === 'boolean';
  if (meta.editor === 'number' || meta.editor === 'color') return typeof value === 'number';
  if (meta.editor === 'enum') return (meta.enumValues ?? []).some((v) => v.value === value);
  return typeof value === 'string' || value === null;
}

/**
 * A descriptor as Visual FoxPro 9 itself has it.
 *
 * Which properties an object of a class holds, what they start out at, and which events and
 * methods it answers to are the product's answers, read from `baseClassMembers.ts` - measured
 * out of vfp9.exe rather than written down from the reference pages. The descriptor beside it
 * says only how the designer should show each one: the editor, the category, the enumerated
 * labels. Membership kept by hand is what drifts, and it had: the day it was first measured the
 * two disagreed in 345 places.
 *
 * A class the measurement does not cover - what an OLE drag carries, the COM server a project
 * builds - keeps the list its descriptor writes, because there is nothing better to hold it to.
 */
function fromProduct<T extends ObjectDescriptor>(name: string, descriptor: T): T {
  const measured = BASE_CLASS_MEMBERS[name];
  if (!measured) return withUniversal(descriptor);
  const universal = c.props(descriptor.properties, c.identityProps, c.dragDropProps, c.containerProps);
  const known = new Map(universal.map((p) => [p.name.toUpperCase(), p]));
  const events = new Map(c.events(descriptor.events, c.oleDragEvents).map((e) => [e.name.toUpperCase(), e]));
  const refuses = new Map(Object.entries(measured.readOnly).map(([p, code]) => [p.toUpperCase(), code]));
  const properties: PropertyMeta[] = [];
  for (const [propName, bare] of Object.entries(measured.properties)) {
    // A descriptor describes a control on a form, and a control on a form is in a container: the
    // colours it takes from the system's 3-D palette are resolved against the one it is in, so a
    // Label answers white on its own and 15790320 once it is placed. Drawing it white put a white
    // box on every form the product draws in the colour of the form.
    const value = propName in measured.contained ? measured.contained[propName]! : bare;
    const meta = known.get(propName.toUpperCase());
    const shown = meta && suits(meta, value) ? meta : { editor: guessEditor(value), category: 'Other' as const };
    const refused = refuses.get(propName.toUpperCase());
    properties.push({ ...shown, name: meta?.name ?? propName, default: value, readOnly: refused !== undefined, refusesWrite: refused });
  }
  // a property whose value is not a constant keeps whatever default our own descriptor gives it,
  // because there is no measurement of one to take
  for (const propName of measured.computed) {
    const meta = known.get(propName.toUpperCase());
    const base = meta ?? { name: propName, editor: 'text' as const, category: 'Other' as const, default: null, hidden: true };
    const refused = refuses.get(propName.toUpperCase());
    properties.push({ ...base, readOnly: refused !== undefined || base.readOnly === true, refusesWrite: refused });
  }
  const eventList: EventMeta[] = measured.events.map((e) => events.get(e.toUpperCase()) ?? { name: e });
  return {
    ...descriptor,
    properties: properties.sort((a, b) => a.name.localeCompare(b.name)),
    events: eventList.sort((a, b) => a.name.localeCompare(b.name)),
    methods: [...measured.methods].sort((a, b) => a.localeCompare(b)),
  };
}

export const FORM_DESCRIPTOR: ObjectDescriptor = fromProduct('Form', d.form);

/**
 * The object classes that are not controls: nothing of them is drawn, but a program reaches
 * them by name and reads what they hold, so they are described the same way.
 */
export const OBJECT_DESCRIPTORS: Record<string, ObjectDescriptor> = {
  DataEnvironment: fromProduct('DataEnvironment', d.dataenvironment),
  Cursor: fromProduct('Cursor', d.cursor),
  Relation: fromProduct('Relation', d.relation),
  // the container is not a control and has no drag of its own, so it is listed as it is
  Database: d.database,
  // the report listener is handed the report band by band; it draws nothing of its own
  ReportListener: fromProduct('ReportListener', d.reportlistener),
  // a set of forms is not a window itself, so it is described here rather than as a control
  FormSet: fromProduct('FormSet', d.formset),
  // the objects a program reaches that are not on a form
  Application: fromProduct('Application', d.application),
  Exception: fromProduct('Exception', d.exception),
  Project: fromProduct('Project', d.project),
  ProjectHook: fromProduct('ProjectHook', d.projecthook),
  File: fromProduct('File', d.projectfile),
  Server: d.server,
  Connection: d.connection,
  XMLAdapter: fromProduct('XMLAdapter', d.xmladapter),
  XMLTable: fromProduct('XMLTable', d.xmltable),
  XMLField: fromProduct('XMLField', d.xmlfield),
  // the cursor an adapter fills, what a drag carries, and what every control is
  CursorAdapter: fromProduct('CursorAdapter', d.cursoradapter),
  DataObject: d.dataobject,
  Control: fromProduct('Control', d.control),
};

const CONTROLS: Record<ControlType, ControlDescriptor> = {
  Label: d.label,
  TextBox: d.textbox,
  EditBox: d.editbox,
  CommandButton: d.commandbutton,
  CheckBox: d.checkbox,
  OptionGroup: d.optiongroup,
  OptionButton: d.optionbutton,
  ComboBox: d.combobox,
  ListBox: d.listbox,
  Spinner: d.spinner,
  Shape: d.shape,
  Line: d.line,
  Image: d.image,
  Container: d.container,
  PageFrame: d.pageframe,
  Page: d.page,
  Grid: d.grid,
  Column: d.column,
  Header: d.header,
  Timer: d.timer,
  CommandGroup: d.commandgroup,
  Custom: d.custom,
  Session: d.session,
  Hyperlink: d.hyperlink,
  Collection: d.collection,
  Toolbar: d.toolbar,
  Separator: d.separator,
  OleControl: d.olecontrol,
  OleBoundControl: d.oleboundcontrol,
};

export const CONTROL_DESCRIPTORS: Record<ControlType, ControlDescriptor> = Object.fromEntries(
  Object.entries(CONTROLS).map(([type, descriptor]) => [type, fromProduct(type, descriptor)]),
) as Record<ControlType, ControlDescriptor>;

/** Toolbox order (VFP Form Controls toolbar order, roughly). */
export const TOOLBOX_TYPES: ControlType[] = [
  'Label',
  'TextBox',
  'EditBox',
  'CommandButton',
  'CommandGroup',
  'OptionGroup',
  'CheckBox',
  'ComboBox',
  'ListBox',
  'Spinner',
  'Grid',
  'Image',
  'Timer',
  'PageFrame',
  'Line',
  'Shape',
  'Container',
];

export function getDescriptor(type: ControlType): ControlDescriptor {
  return CONTROL_DESCRIPTORS[type];
}

export function getObjectDescriptor(node: ControlNode | FormNode): ObjectDescriptor {
  return 'type' in node ? CONTROL_DESCRIPTORS[node.type] : FORM_DESCRIPTOR;
}

export function getPropertyMeta(desc: ObjectDescriptor, name: string): PropertyMeta | undefined {
  return desc.properties.find((p) => p.name === name);
}

/** Descriptor defaults merged with the node's sparse props. */
export function resolveProps(node: ControlNode | FormNode): Record<string, PropValue> {
  const desc = getObjectDescriptor(node);
  const out: Record<string, PropValue> = {};
  for (const p of desc.properties) out[p.name] = p.default;
  Object.assign(out, node.props);
  return out;
}

export function getProp<T extends PropValue = PropValue>(node: ControlNode | FormNode, name: string): T {
  if (name in node.props) return node.props[name] as T;
  const meta = getPropertyMeta(getObjectDescriptor(node), name);
  return (meta ? meta.default : null) as T;
}

/** True when the node overrides the default (VFP shows such rows in bold). */
export function isPropChanged(node: ControlNode | FormNode, name: string): boolean {
  if (!(name in node.props)) return false;
  const meta = getPropertyMeta(getObjectDescriptor(node), name);
  return !meta || node.props[name] !== meta.default;
}

export function isVisualType(type: ControlType): boolean {
  return !CONTROL_DESCRIPTORS[type].nonVisual;
}

/** Whether `parentType` (undefined = form) may directly contain `childType`. */
export function canContain(parentType: ControlType | undefined, childType: ControlType): boolean {
  const childDesc = CONTROL_DESCRIPTORS[childType];
  if (parentType === undefined) return !childDesc.hideInToolbox;
  const cont = CONTROL_DESCRIPTORS[parentType].container;
  if (!cont) return false;
  if (cont.accepts === 'visual') return !childDesc.hideInToolbox;
  return cont.accepts.includes(childType);
}
