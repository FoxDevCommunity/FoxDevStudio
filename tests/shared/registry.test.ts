import { describe, expect, it } from 'vitest';
import { CONTROL_TYPES } from '@shared/form/schema';
import type { ObjectDescriptor } from '@shared/registry';
import { BASE_CLASS_MEMBERS, CONTROL_DESCRIPTORS, FORM_DESCRIPTOR, TOOLBOX_TYPES, canContain, getProp, getPropertyMeta, isPropChanged, resolveProps } from '@shared/registry';
import { sampleForm } from '../helpers/fixtures';
import { findNode } from '@shared/form/tree';

describe('control registry', () => {
  it('has a complete, consistent descriptor for every control type', () => {
    const prefixes = new Set<string>();
    for (const type of CONTROL_TYPES) {
      const d = CONTROL_DESCRIPTORS[type];
      expect(d, type).toBeDefined();
      expect(d.type).toBe(type);
      expect(d.namePrefix).toMatch(/^[A-Za-z]+$/);
      expect(prefixes.has(d.namePrefix), `duplicate prefix ${d.namePrefix}`).toBe(false);
      prefixes.add(d.namePrefix);
      expect(d.events.some((e) => e.name === d.defaultEvent), `${type} defaultEvent ${d.defaultEvent}`).toBe(true);
      const names = d.properties.map((p) => p.name);
      expect(new Set(names).size, `${type} duplicate properties`).toBe(names.length);
      for (const p of d.properties) {
        expect(p.default, `${type}.${p.name} default`).not.toBeUndefined();
        if (p.editor === 'enum') expect(p.enumValues!.some((v) => v.value === p.default), `${type}.${p.name} enum default`).toBe(true);
        if (p.editor === 'boolean') expect(typeof p.default).toBe('boolean');
        if (p.editor === 'number' || p.editor === 'color') expect(typeof p.default).toBe('number');
      }
      if (d.container?.autoChildren) expect(canContain(type, d.container.autoChildren.type)).toBe(true);
    }
    expect(FORM_DESCRIPTOR.events.some((e) => e.name === 'Init')).toBe(true);
    for (const t of TOOLBOX_TYPES) expect(CONTROL_DESCRIPTORS[t].hideInToolbox).toBeFalsy();
  });

  it('reaches every declared property by its exact name, with unique enum values', () => {
    const all: [string, ObjectDescriptor][] = [['Form', FORM_DESCRIPTOR], ...CONTROL_TYPES.map((t): [string, ObjectDescriptor] => [t, CONTROL_DESCRIPTORS[t]])];
    for (const [name, d] of all) {
      const seen = new Map<string, string>();
      for (const p of d.properties) {
        expect(getPropertyMeta(d, p.name), `${name}.${p.name} not reachable`).toBe(p);
        const lower = p.name.toLowerCase();
        expect(seen.get(lower), `${name} declares ${p.name} and ${seen.get(lower)}`).toBeUndefined();
        seen.set(lower, p.name);
        if (p.editor !== 'enum') {
          expect(p.enumValues, `${name}.${p.name} has enumValues but is not an enum`).toBeUndefined();
          continue;
        }
        const values = p.enumValues!.map((v) => v.value);
        expect(new Set(values).size, `${name}.${p.name} duplicate enum values`).toBe(values.length);
        expect(new Set(p.enumValues!.map((v) => v.label)).size, `${name}.${p.name} duplicate enum labels`).toBe(values.length);
        expect(values, `${name}.${p.name} default not in enumValues`).toContain(p.default);
      }
    }
  });

  /**
   * The descriptors say what Visual FoxPro 9 says, member for member.
   *
   * `baseClassMembers.ts` is measured out of vfp9.exe (`scripts/vfp-base-classes.mjs`), and a
   * descriptor is that measurement wearing the designer's clothes. A property the product does
   * not have must not appear, and one it has must start out at what it starts out at there.
   */
  it('describes each base class the way the product answers about it', () => {
    const all: [string, ObjectDescriptor][] = [['Form', FORM_DESCRIPTOR], ...CONTROL_TYPES.map((t): [string, ObjectDescriptor] => [t, CONTROL_DESCRIPTORS[t]])];
    let checked = 0;
    for (const [name, d] of all) {
      const measured = BASE_CLASS_MEMBERS[name];
      if (!measured) continue;
      checked += 1;
      const declared = new Map(d.properties.map((p) => [p.name.toUpperCase(), p]));
      const wanted = new Set([...Object.keys(measured.properties), ...measured.computed].map((p) => p.toUpperCase()));
      expect([...declared.keys()].filter((p) => !wanted.has(p)), `${name} declares what the product has not`).toEqual([]);
      // a descriptor describes a control on a form, so where the product answers differently
      // once the object is in a container - a Label's white turning into the form's grey - the
      // contained answer is the one the descriptor carries
      for (const [prop, bare] of Object.entries(measured.properties)) {
        const value = prop in measured.contained ? measured.contained[prop] : bare;
        expect(declared.get(prop.toUpperCase())?.default, `${name}.${prop} default`).toBe(value);
      }
      expect(d.events.map((e) => e.name).sort(), `${name} events`).toEqual([...measured.events].sort());
      expect(d.methods, `${name} methods`).toEqual([...measured.methods].sort((a, b) => a.localeCompare(b)));
    }
    expect(checked, 'base classes measured out of the product').toBeGreaterThan(25);
  });

  it('applies containment rules', () => {
    expect(canContain(undefined, 'CommandButton')).toBe(true);
    expect(canContain(undefined, 'Page')).toBe(false);
    expect(canContain('PageFrame', 'Page')).toBe(true);
    expect(canContain('PageFrame', 'TextBox')).toBe(false);
    expect(canContain('Page', 'TextBox')).toBe(true);
    expect(canContain('Container', 'Grid')).toBe(true);
    expect(canContain('TextBox', 'Label')).toBe(false);
    expect(canContain('Grid', 'Column')).toBe(true);
    expect(canContain('Column', 'Header')).toBe(true);
  });

  it('resolves sparse props over defaults', () => {
    const { form } = sampleForm();
    const cmd = findNode(form, 'Command1')!;
    const resolved = resolveProps(cmd);
    expect(resolved['Caption']).toBe('OK');
    expect(resolved['Width']).toBe(84);
    expect(resolved['Enabled']).toBe(true);
    // what the document leaves out comes from the class, as the product itself has it
    const label = findNode(form, 'Label1')!;
    expect(getProp(label, 'Width')).toBe(100);
    expect(getProp(label, 'Height')).toBe(17);
    expect(getProp(form, 'Width')).toBe(400);
    expect(getProp(form, 'BorderStyle')).toBe(3);
    expect(isPropChanged(cmd, 'Caption')).toBe(true);
    expect(isPropChanged(cmd, 'Enabled')).toBe(false);
  });
});
