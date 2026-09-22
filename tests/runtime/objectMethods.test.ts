/**
 * The methods an object answers to that are not events: what a form writes on itself, what a
 * drag carries, what a collection is asked about an item, and what the application object runs.
 *
 * These are the host's rather than the VM's - they act on the object model - so they are driven
 * through the object model directly, the way the VM reaches them.
 */

import { describe, expect, it } from 'vitest';
import { Desktop } from '@shared/runtime/objectModel';
import { Collection } from '@shared/runtime/collection';
import { registryObject } from '@shared/runtime/dataEnvironment';
import { HostError } from '@shared/runtime/host';
import type { FormNode } from '@shared/form/schema';
import type { VmValue } from '@shared/runtime/values';

/** The smallest form there is: one that has been instantiated and can be drawn on. */
function form(desktop: Desktop) {
  const node: FormNode = { name: 'Form1', props: { Caption: 'A form' }, methods: {}, children: [] };
  return desktop.instantiate(node, -1);
}

describe('what an object answers to', () => {
  it('writes on the form where CurrentX and CurrentY say, and moves them on', () => {
    const desktop = new Desktop();
    const shown = form(desktop);
    shown.set('CurrentX', 20);
    shown.set('CurrentY', 30);

    desktop.callMethod(shown.handle, 'Print', ['Hello']);

    const drawn = shown.drawings.at(-1);
    expect(drawn?.shape).toBe('text');
    expect(drawn?.text).toBe('Hello');
    expect(drawn?.x1).toBe(20);
    expect(drawn?.y1).toBe(30);
    // the next one starts after this one, as VFP leaves CurrentX
    expect(Number(shown.get('CurrentX'))).toBeGreaterThan(20);
    expect(Number(shown.get('CurrentY'))).toBe(30);
  });

  it('carries a control from Drag(1) to Drag(2) and drops it on what the pointer is over', () => {
    const desktop = new Desktop();
    const shown = form(desktop);
    const dropped: string[] = [];
    desktop.dispatch = (obj, event) => {
      dropped.push(`${obj.name}.${event}`);
      return null;
    };

    desktop.callMethod(shown.handle, 'Drag', [1]);
    expect(desktop.dragging).toBe(shown);
    desktop.callMethod(shown.handle, 'Drag', [2]);

    expect(desktop.dragging).toBeNull();
    expect(dropped).toEqual(['Form1.DragDrop']);
  });

  it('asks an object what it is putting on the DataObject when a drag starts', () => {
    const desktop = new Desktop();
    const shown = form(desktop);
    const started: VmValue[] = [];
    desktop.dispatch = (_obj, event, args) => {
      if (event === 'OLEStartDrag') started.push(args?.[0] ?? null);
      return null;
    };

    desktop.callMethod(shown.handle, 'OLEDrag', [true]);

    // what it was handed is an object, which is the DataObject the data goes on
    expect(started).toHaveLength(1);
    expect(started[0]).toHaveProperty('$obj');
  });

  it('keeps what SetData put on a DataObject until ClearData takes it off', () => {
    const carrier = registryObject('DataObject');
    expect(carrier).toBeDefined();

    expect(carrier!.call('SetData', ['a name'])).toBe(true);
    expect(carrier!.call('GetData', [1])).toBe('a name');
    expect(carrier!.call('GetFormat', [1])).toBe(true);
    expect(carrier!.call('ClearData', [])).toBe(true);
    expect(carrier!.call('GetData', [1])).toBe('');
  });

  it('answers what a collection calls an item, and where a name is', () => {
    const one = registryObject('Empty') ?? registryObject('DataObject')!;
    const collection = new Collection([one], ['first']);

    expect(collection.call('GetKey', [1])).toBe('first');
    expect(collection.call('GetKey', ['FIRST'])).toBe(1);
    expect(collection.call('GetKey', [9])).toBe('');
  });

  it('works out an expression and sets a variable through the application object', async () => {
    const desktop = new Desktop();
    const set: [string, VmValue][] = [];
    desktop.evaluate = async (expression) => `worked out ${expression}`;
    desktop.setVariable = (name, value) => set.push([name, value]);

    await expect(desktop.callMethod(-1, 'Eval', ['1 + 1'])).resolves.toBe('worked out 1 + 1');
    expect(desktop.callMethod(-1, 'SetVar', ['gcName', 'Jorge'])).toBe(true);
    expect(set).toEqual([['gcName', 'Jorge']]);
  });

  it('says what a verb and a clone would need, rather than pretending to do them', () => {
    const desktop = new Desktop();
    const shown = form(desktop);

    expect(() => desktop.callMethod(shown.handle, 'DoVerb', [-1])).toThrow(HostError);
    // CloneObject is refused in the product's own words, measured out of vfp9.exe: error 1953
    expect(() => desktop.callMethod(shown.handle, 'CloneObject', ['Copy1'])).toThrow(/design mode/);
  });
});
