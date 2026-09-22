/**
 * A class library (`.fxc`): several class definitions in one file, the way a `.vcx` holds them.
 *
 * A class is a form with a lineage. Everything the form designer already knows how to edit - the
 * properties, the method source, the tree of children - is the same shape here, so a class
 * definition *is* a `FormNode` with the four things a class has on top of it: the Visual FoxPro
 * base class it ultimately derives from, the class it was subclassed from, where that class
 * lives, and what it is for.
 *
 * The one thing a class needs that a form does not is a record of what it owns. A definition
 * carries every value an instance of it would have, inherited ones included, so it can be
 * instantiated without reading the parent library at all - and `own` says which of those the
 * class writes itself. That is what lets the designer show an inherited value greyed and an
 * override marked, and it is why the answer survives a parent library that has gone missing.
 */

import { z } from 'zod';
import {
  controlNodeSchema,
  documentMetaSchema,
  methodsSchema,
  propsSchema,
  type ControlNode,
  type DocumentMeta,
  type FormNode,
} from '../form/schema';

export const CLASSLIB_SCHEMA_ID = 'foxdev-classlib' as const;
export const CLASSLIB_VERSION = 1 as const;

/** What one object of a class declares for itself, rather than taking from the parent class. */
export interface ClassMembers {
  /** Property names the class writes; every other value on the object came from the parent. */
  props?: string[];
  /** Method and event names the class writes code for. */
  methods?: string[];
}

/**
 * One class. `name` is what `NEWOBJECT()`, `ADD OBJECT` and a subclass's `parentClass` ask for;
 * the rest of the `FormNode` is the object an instance of it starts life as.
 */
export interface ClassDefinition extends FormNode {
  /** The Visual FoxPro base class underneath the whole chain: `container`, `form`, `textbox`. */
  baseClass: string;
  /** The class this one was subclassed from. Absent when it derives straight from `baseClass`. */
  parentClass?: string;
  /** The library `parentClass` lives in, when that is not this one. `CLASSLOC` in a `.vcx`. */
  parentLibrary?: string;
  /** What the class is for, as the Class Info dialog asks for it. */
  description?: string;
  /**
   * Object path -> the members declared here, for every object of the class that declares any.
   * The path starts with the class's own name and reads down the tree - `mover.lstSource` - the
   * same way `meta.vfp.reserved` keys do. Only meaningful when there is a `parentClass`: a class
   * with none owns everything it has, so ask through `ownsProperty` rather than reading this.
   */
  own?: Record<string, ClassMembers>;
  meta?: DocumentMeta;
}

export interface ClassLibraryDocument {
  $schema: 'foxdev-classlib';
  version: 1;
  /** The library's name - the file stem, which is what `SET CLASSLIB TO` and `CLASSLOC` use. */
  name: string;
  classes: ClassDefinition[];
  meta?: DocumentMeta;
}

export const classMembersSchema: z.ZodType<ClassMembers> = z.object({
  props: z.array(z.string()).optional(),
  methods: z.array(z.string()).optional(),
});

export const classDefinitionSchema: z.ZodType<ClassDefinition> = z.object({
  name: z.string().min(1),
  props: propsSchema,
  methods: methodsSchema,
  children: z.array(controlNodeSchema),
  baseClass: z.string().min(1),
  parentClass: z.string().optional(),
  parentLibrary: z.string().optional(),
  description: z.string().optional(),
  own: z.record(z.string(), classMembersSchema).optional(),
  meta: documentMetaSchema.optional(),
});

export const classLibraryDocumentSchema: z.ZodType<ClassLibraryDocument> = z.object({
  $schema: z.literal(CLASSLIB_SCHEMA_ID),
  version: z.literal(CLASSLIB_VERSION),
  name: z.string().min(1),
  classes: z.array(classDefinitionSchema),
  meta: documentMetaSchema.optional(),
});

/** The dotted path of an object inside a class, which is how `own` is keyed. */
export function classMemberPath(className: string, ...names: string[]): string {
  return [className, ...names].join('.');
}

/**
 * Whether the class writes this property itself.
 *
 * The rule, once, in one place: a class that derives straight from a base class owns everything
 * it has; a subclass owns what `own` lists and inherits the rest. A document that says nothing
 * about an object of a subclass is saying that object came from the parent whole.
 */
export function ownsProperty(cls: ClassDefinition, path: string, prop: string): boolean {
  return owns(cls, path, prop, 'props');
}

/** The same question about a method or an event: is this code the class's, or its parent's? */
export function ownsMethod(cls: ClassDefinition, path: string, method: string): boolean {
  return owns(cls, path, method, 'methods');
}

function owns(cls: ClassDefinition, path: string, member: string, kind: keyof ClassMembers): boolean {
  if (!cls.parentClass) return true;
  const declared = cls.own?.[path]?.[kind] ?? [];
  return declared.some((name) => name.toLowerCase() === member.toLowerCase());
}

/** A class by name, however it is spelled: Visual FoxPro matches class names without case. */
export function findClass(doc: ClassLibraryDocument, name: string): ClassDefinition | undefined {
  return doc.classes.find((c) => c.name.toLowerCase() === name.toLowerCase());
}

/** A class definition as a plain form node, which is what the designer and the runtime take. */
export function classAsNode(cls: ClassDefinition): FormNode {
  return { name: cls.name, props: cls.props, methods: cls.methods, children: cls.children as ControlNode[] };
}
