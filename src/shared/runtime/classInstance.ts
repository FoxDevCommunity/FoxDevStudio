/**
 * Turning a `DEFINE CLASS` definition into the node tree the object model already knows how to
 * bring to life. A class defined in source and a form drawn in the designer end up as the same
 * kind of thing, which is why `CREATEOBJECT("form1")` produces a window you can interact with.
 */

import { nanoid } from 'nanoid';
import type { ControlNode, ControlType, FormNode, PropValue } from '../form/schema';
import { getDescriptor, getPropertyMeta, FORM_DESCRIPTOR, type ControlDescriptor, type ObjectDescriptor } from '../registry';
import { baseClassToControlType } from '../vfp/importForm';
import { isNonVisualBaseClass, resolveInheritance, type VfpClassDef } from './classDef';
import { isArray, vmToProp, type VmValue } from './values';

export interface ClassInstanceWarning {
  object: string;
  message: string;
}

/** A `DIMENSION` in a class body: an array property, which a node's sparse props cannot hold. */
export interface ClassArray {
  /** Member names from the class down to the object that owns it; empty for the class itself. */
  path: string[];
  name: string;
  values: VmValue[];
}

export interface BuiltClass {
  /** The node tree, shaped like a form so the desktop can instantiate it. */
  form: FormNode;
  /**
   * Property values that are expressions, to work out once the object exists: keyed the way
   * `workOutProperties` reads them, the class's own name first, then the member path.
   */
  expressions: Record<string, string>;
  /** True when the class has no visual representation and must not be shown. */
  nonVisual: boolean;
  /** Array properties to declare on the live objects once the tree is instantiated. */
  arrays: ClassArray[];
  warnings: ClassInstanceWarning[];
}

/** Where an `applyProperties` pass is writing: the node itself plus where it sits in the tree. */
interface BuildContext {
  arrays: ClassArray[];
  path: string[];
  expressions: Record<string, string>;
}

/** Applies property entries to a node's sparse props, keeping VFP's case-insensitive names. */
function applyProperties(
  target: { name: string; props: Record<string, PropValue>; children?: ControlNode[] },
  descriptor: ObjectDescriptor,
  entries: readonly { name: string; value: VmValue; expression?: string }[],
  ctx: BuildContext,
): void {
  // VFP allows a dotted path so a member of the object can be set from the same WITH list:
  // `ADD OBJECT pgf AS pageframe WITH PageCount = 3, Page1.Caption = "x"`. Those members do not
  // exist until the count property has been read, so they wait for a second pass.
  const dotted: { name: string; value: VmValue }[] = [];

  for (const entry of entries) {
    if (entry.name.includes('.')) {
      dotted.push(entry);
      continue;
    }
    // worked out when the object is made; a later constant for the same property wins over it
    const key = ['*', ...ctx.path, entry.name].join('.');
    if (entry.expression !== undefined) {
      ctx.expressions[key] = entry.expression;
      continue;
    }
    delete ctx.expressions[key];
    // `DIMENSION aRGB[3]` arrives as an array value; a node's props hold only scalars, so it
    // is recorded and declared on the live object instead.
    if (isArray(entry.value)) {
      ctx.arrays.push({ path: ctx.path, name: entry.name, values: entry.value.$arr });
      continue;
    }
    const value = vmToProp(entry.value);
    if (entry.name.toLowerCase() === 'name') {
      if (typeof value === 'string' && value.length > 0) target.name = value;
      continue;
    }
    const declared = descriptor.properties.find((p) => p.name.toLowerCase() === entry.name.toLowerCase());
    if (!declared) {
      // VFP lets a class add its own properties; keep them so code can read them back
      target.props[entry.name] = value;
      continue;
    }
    const meta = getPropertyMeta(descriptor, declared.name);
    if (meta && meta.default === value) continue; // stay sparse
    target.props[declared.name] = value;
  }

  makeAutoChildren(target, descriptor);
  for (const entry of dotted) {
    const dot = entry.name.indexOf('.');
    applyToMember(target, entry.name.slice(0, dot), entry.name.slice(dot + 1), entry.value, ctx);
  }
}

/**
 * Creates the children a container makes for itself: a PageFrame its Pages, an OptionGroup its
 * buttons, a Grid its Columns. VFP creates them along with the object, so `Page1` can be named
 * straight away in the same WITH list. Children already built from a class body are left alone.
 */
function makeAutoChildren(
  target: { props: Record<string, PropValue>; children?: ControlNode[] },
  descriptor: ObjectDescriptor,
): void {
  const auto = 'container' in descriptor ? (descriptor as ControlDescriptor).container?.autoChildren : undefined;
  if (!auto || (target.children && target.children.length > 0)) return;

  const wanted = auto.countProp
    ? (target.props[auto.countProp] ?? getPropertyMeta(descriptor, auto.countProp)?.default)
    : undefined;
  const count = wanted === undefined ? auto.count : Math.max(0, Math.trunc(Number(wanted) || 0));

  target.children ??= [];
  for (let i = 0; i < count; i++) target.children.push(autoChild(auto.type, i));
}

/** One container-made child, named and captioned the way VFP names them: Page1, Option2. */
function autoChild(type: ControlType, index: number): ControlNode {
  const desc = getDescriptor(type);
  const name = `${desc.namePrefix}${index + 1}`;
  const node: ControlNode = { id: nanoid(8), type, name, props: {}, methods: {}, children: [] };
  if (desc.properties.some((p) => p.name === 'Caption')) node.props['Caption'] = name;
  if (type === 'OptionButton') Object.assign(node.props, { Left: 5, Top: 5 + index * 17 });
  if (type === 'CommandButton') Object.assign(node.props, { Left: 5, Top: 5 + index * 27, Width: 74 });
  makeAutoChildren(node, desc);
  return node;
}

/**
 * Sets a property on a member the container creates for itself. The member is found by name and
 * never created, so a WITH list naming a page that does not exist is ignored rather than fatal.
 */
function applyToMember(
  target: { children?: ControlNode[] },
  memberName: string,
  property: string,
  value: VmValue,
  ctx: BuildContext,
): void {
  const child = target.children?.find((c) => c.name.toLowerCase() === memberName.toLowerCase());
  if (!child) return;
  applyProperties(child, getDescriptor(child.type), [{ name: property, value }], { ...ctx, path: [...ctx.path, child.name] });
}

/**
 * Builds the node tree for a class. `classes` is every class in scope, so a member whose class
 * is itself defined in source resolves too.
 */
export function buildClassInstance(classes: readonly VfpClassDef[], className: string): BuiltClass | null {
  const resolved = resolveInheritance(classes, className);
  if (!resolved) return null;

  const warnings: ClassInstanceWarning[] = [];
  const arrays: ClassArray[] = [];
  const expressions: Record<string, string> = {};
  const nonVisual = isNonVisualBaseClass(resolved.baseClass);
  const base = baseClassToControlType(resolved.baseClass);

  if (!nonVisual && base === null) {
    warnings.push({ object: resolved.name, message: `base class ${resolved.baseClass} is not supported; the object has properties but nothing on screen` });
  }

  const form: FormNode = { name: resolved.name, props: {}, methods: {}, children: [] };
  applyProperties(form, base === 'Form' || base === null ? FORM_DESCRIPTOR : getDescriptor(base), resolved.properties, { arrays, path: [], expressions });

  for (const member of resolved.members) {
    const node = buildMember(classes, member, warnings, arrays, expressions, resolved.name, new Set([resolved.name.toLowerCase()]));
    if (node) form.children.push(node);
  }

  return { form, nonVisual: nonVisual || base === null, arrays, expressions, warnings };
}

/** One `ADD OBJECT`, which may itself name a class defined in source. */
function buildMember(
  classes: readonly VfpClassDef[],
  member: VfpClassDef['members'][number],
  warnings: ClassInstanceWarning[],
  arrays: ClassArray[],
  expressions: Record<string, string>,
  path: string,
  seen: ReadonlySet<string>,
): ControlNode | null {
  const memberPath = `${path}.${member.name}`;
  const own = resolveInheritance(classes, member.class);

  // a member of a class defined in source: build it, then apply the WITH clause on top
  if (own && !seen.has(member.class.toLowerCase())) {
    const nested = buildClassInstance(classes, member.class);
    if (nested) {
      warnings.push(...nested.warnings);
      const type = baseClassToControlType(own.baseClass);
      if (type === null || type === 'Form') {
        warnings.push({ object: memberPath, message: `${member.class} cannot be contained in another object` });
        return null;
      }
      const node: ControlNode = { id: nanoid(8), type, name: member.name, props: { ...nested.form.props }, methods: {}, children: nested.form.children };
      for (const a of nested.arrays) arrays.push({ ...a, path: [member.name, ...a.path] });
      // the member's own class's expressions, under the member's name
      for (const [key, source] of Object.entries(nested.expressions)) expressions[['*', member.name, ...key.split('.').slice(1)].join('.')] = source;
      applyProperties(node, getDescriptor(type), member.properties, { arrays, path: [member.name], expressions });
      return node;
    }
  }

  const type = baseClassToControlType(member.class);
  if (type === null || type === 'Form') {
    warnings.push({ object: memberPath, message: `ADD OBJECT of class ${member.class} is not supported` });
    return null;
  }

  const node: ControlNode = { id: nanoid(8), type, name: member.name, props: {}, methods: {}, children: [] };
  applyProperties(node, getDescriptor(type), member.properties, { arrays, path: [member.name], expressions });
  return node;
}

/** True when the class, once inherited, is a form rather than a control or a plain object. */
export function classIsForm(classes: readonly VfpClassDef[], className: string): boolean {
  const resolved = resolveInheritance(classes, className);
  return resolved !== undefined && baseClassToControlType(resolved.baseClass) === 'Form';
}
