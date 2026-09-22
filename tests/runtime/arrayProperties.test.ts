/**
 * Array properties an object adds for itself, and the base classes a program can ask for.
 *
 * Visual FoxPro records a form's own members in a memo of its own, one per line: `oToolbar` is a
 * property, `*setfields` a method, and `^oWindows[1,0]` an array. Only the first two were read,
 * so the SDI Form sample's Add Window button failed on its first line with "Unknown member
 * OWINDOWS" - `ALEN(thisform.oWindows)` on a property that was never dimensioned - and then on
 * `CreateObject('form')`, which fell past every class we answer to and reached COM as "Invalid
 * class string".
 */

import { describe, expect, it } from 'vitest';
import { Desktop } from '@shared/runtime/objectModel';
import type { FormNode } from '@shared/form/schema';

describe('an array property an object adds for itself', () => {
  it('exists before anything runs, at the shape it was declared', () => {
    const desktop = new Desktop();
    const node: FormNode = { name: 'SDIFORM', props: {}, methods: {}, children: [] };
    const form = desktop.instantiate(node, -1, { arrays: { 'SDIFORM.oWindows': [1, 0], 'SDIFORM.aIcon': [5, 2] } });

    expect(form.getArray('owindows')).toEqual({ $arr: [false], $cols: 0 });
    const icons = form.getArray('aIcon');
    expect(icons?.$arr).toHaveLength(10);
    expect(icons?.$cols).toBe(2);
  });

  it('is reached by a row and a column, or by one running subscript', () => {
    const desktop = new Desktop();
    const form = desktop.instantiate({ name: 'F', props: {}, methods: {}, children: [] }, -1, {
      arrays: { 'F.aIcon': [3, 2] },
    });

    form.setElement('aIcon', [2, 1], 'third');
    expect(form.getArray('aIcon')?.$arr[2]).toBe('third');
    form.setElement('aIcon', [4], 'fourth');
    expect(form.getArray('aIcon')?.$arr[3]).toBe('fourth');

    // and a subscript outside the shape is refused rather than growing it
    expect(() => form.setElement('aIcon', [2, 3], 'x')).toThrow();
    expect(() => form.setElement('aIcon', [7], 'x')).toThrow();
    expect(() => form.setElement('aIcon', [0, 1], 'x')).toThrow();
  });
});

describe('a Visual FoxPro base class', () => {
  it('is a class CREATEOBJECT can ask for, and comes back hidden', () => {
    const desktop = new Desktop();
    const form = desktop.createBaseObject('form');
    expect(form?.type).toBe('Form');
    expect(form?.get('Visible')).toBe(false);

    const label = desktop.createBaseObject('Label');
    expect(label?.type).toBe('Label');
    // it stands on its own: nothing contains it until something adds it
    expect(label?.parent ?? null).toBeNull();

    expect(desktop.createBaseObject('Word.Application')).toBeUndefined();
  });
});
