/**
 * Classes defined in FoxPro source with `DEFINE CLASS ... ENDDEFINE`.
 *
 * The compiler folds each definition into this shape and the runtime builds a live object from
 * it, the same way it builds one from a designed form. A visual class becomes a real form or
 * control; a non-visual one is an object with properties and methods and nothing on screen.
 */

import type { VmValue } from './values';

export interface VfpClassMember {
  /** `ADD OBJECT name AS class`. */
  name: string;
  /** The member's class: a VFP base class or another class defined in source. */
  class: string;
  /** `NOINIT`: do not run the member's Init when the container is created. */
  noinit: boolean;
  /** Property values from the `WITH` clause, already constant-folded. */
  properties: VfpClassProperty[];
}

/**
 * One property a class body or a WITH clause sets. `expression` is there when the value was
 * written as an expression rather than a constant: Visual FoxPro works it out when an object is
 * made (measured), so `value` is only a placeholder until then.
 */
export interface VfpClassProperty {
  name: string;
  value: VmValue;
  expression?: string;
}

export interface VfpClassDef {
  /** As written in the source; comparison is case-insensitive. */
  name: string;
  /** What it inherits from: a VFP base class, or another class in the same program. */
  baseClass: string;
  /** Class-level property assignments, in source order. */
  properties: VfpClassProperty[];
  /** Contained objects, in the order they were added. */
  members: VfpClassMember[];
  /** Methods this class defines. */
  methods: VfpClassMethod[];
  /** Module holding this class's method bodies; set by the runtime when the module is loaded. */
  module?: number;
}

export interface VfpClassMethod {
  /**
   * Name as written: `Init`, `Click`, or `image1.Click` for a method attached to a member.
   * The body lives in its owner's module under `"OWNER.NAME"`.
   */
  name: string;
  /** Class that defines the body, which is not the same class once inheritance is flattened. */
  owner: string;
  /** Module holding the body. */
  module?: number;
}

/** VFP base classes a `DEFINE CLASS` may inherit from that have no visual representation. */
export const NON_VISUAL_BASE_CLASSES = ['custom', 'session', 'exception', 'collection', 'empty', 'relation', 'cursor', 'dataenvironment'];

export function isNonVisualBaseClass(baseClass: string): boolean {
  return NON_VISUAL_BASE_CLASSES.includes(baseClass.toLowerCase());
}

/** Looks a class up by name, case-insensitively, as VFP does. */
export function findClass(classes: readonly VfpClassDef[], name: string): VfpClassDef | undefined {
  return classes.find((c) => c.name.toLowerCase() === name.toLowerCase());
}

/**
 * Flattens a class and everything it inherits from into one definition: the parent's
 * properties and members first, then the child's, so the child wins on conflict. Stops at a
 * VFP base class or at a cycle.
 */
export function resolveInheritance(classes: readonly VfpClassDef[], name: string): VfpClassDef | undefined {
  const chain: VfpClassDef[] = [];
  const seen = new Set<string>();
  let current = findClass(classes, name);

  while (current && !seen.has(current.name.toLowerCase())) {
    seen.add(current.name.toLowerCase());
    chain.unshift(current);
    current = findClass(classes, current.baseClass);
  }
  if (chain.length === 0) return undefined;

  const merged: VfpClassDef = {
    name: chain[chain.length - 1]!.name,
    baseClass: chain[0]!.baseClass,
    properties: [],
    members: [],
    methods: [],
    module: chain[chain.length - 1]!.module,
  };

  for (const def of chain) {
    for (const prop of def.properties) {
      const at = merged.properties.findIndex((p) => p.name.toLowerCase() === prop.name.toLowerCase());
      if (at >= 0) merged.properties[at] = prop;
      else merged.properties.push(prop);
    }
    for (const member of def.members) {
      const at = merged.members.findIndex((m) => m.name.toLowerCase() === member.name.toLowerCase());
      if (at >= 0) merged.members[at] = member;
      else merged.members.push(member);
    }
    for (const method of def.methods) {
      // a child overrides a parent's method of the same name, and keeps its own owner
      const at = merged.methods.findIndex((m) => m.name.toLowerCase() === method.name.toLowerCase());
      const entry = { ...method, owner: method.owner || def.name, module: method.module ?? def.module };
      if (at >= 0) merged.methods[at] = entry;
      else merged.methods.push(entry);
    }
  }
  return merged;
}
