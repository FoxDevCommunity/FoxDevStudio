/**
 * A control whose Init returns .F.
 *
 * The Init page says it plainly: "To prevent a control from being created, return false (.F.)
 * from the Init event. The Destroy event will not be triggered." A *control* - the form still
 * opens, with one fewer control on it. Taking the whole form down instead is how a form built on
 * the Foundation Classes came up blank, because the documented example for this is exactly what
 * they do: return .F. when the table they want is not open.
 *
 * Measured in Visual FoxPro 9 as well as read: a form whose member Init returns .F. comes back
 * as an object with one control instead of two; a form whose own Init returns .F. comes back
 * as .F.
 */

import { describe, expect, it } from 'vitest';
import { Desktop } from '@shared/runtime/objectModel';
import type { FormNode } from '@shared/form/schema';

/** A form with two buttons, one of which will refuse to be created. */
function form(desktop: Desktop) {
  const node: FormNode = {
    name: 'Form1',
    props: { Caption: 'A form' },
    methods: {},
    children: [
      { id: 'a', type: 'CommandButton', name: 'cmdGood', props: {}, methods: {}, children: [] },
      { id: 'b', type: 'CommandButton', name: 'cmdBad', props: {}, methods: {}, children: [] },
      {
        id: 'c',
        type: 'Container',
        name: 'cntBad',
        props: {},
        methods: {},
        children: [{ id: 'd', type: 'CommandButton', name: 'cmdInside', props: {}, methods: {}, children: [] }],
      },
    ],
  };
  return desktop.instantiate(node, -1);
}

/** Answers .F. to the Init of everything named here, and records every event fired. */
function refusing(desktop: Desktop, refuse: string[], fired: string[]) {
  desktop.dispatch = (obj, event) => {
    fired.push(`${obj.name}.${event}`);
    if (event === 'Init' && refuse.includes(obj.name)) return { value: false, nodefault: false };
    return null;
  };
}

describe('a control that refuses to be created', () => {
  it('is the only thing that goes: the form opens without it', async () => {
    const desktop = new Desktop();
    const instance = form(desktop);
    const fired: string[] = [];
    refusing(desktop, ['cmdBad'], fired);

    await expect(desktop.runFormLifecycle(instance)).resolves.toBe(true);

    expect(instance.children.map((c) => c.name)).toEqual(['cmdGood', 'cntBad']);
    expect(instance.child('cmdBad')).toBeUndefined();
    expect(desktop.forms).toContain(instance);
  });

  it('has no Destroy run on it, which the reference says as well', async () => {
    const desktop = new Desktop();
    const instance = form(desktop);
    const fired: string[] = [];
    refusing(desktop, ['cmdBad'], fired);

    await desktop.runFormLifecycle(instance);

    expect(fired).toContain('cmdBad.Init');
    expect(fired.filter((e) => e.startsWith('cmdBad.'))).toEqual(['cmdBad.Init']);
    expect(fired.some((e) => e.endsWith('.Unload'))).toBe(false);
  });

  it('takes what is inside it when a container refuses', async () => {
    const desktop = new Desktop();
    const instance = form(desktop);
    const fired: string[] = [];
    refusing(desktop, ['cntBad'], fired);

    await desktop.runFormLifecycle(instance);

    expect(instance.children.map((c) => c.name)).toEqual(['cmdGood', 'cmdBad']);
    // the child was built and initialised before its container declined, and goes with it
    expect(fired).toContain('cmdInside.Init');
    expect(instance.descendants().map((o) => o.name)).not.toContain('cmdInside');
  });

  it('still cancels the form when the form itself refuses', async () => {
    const desktop = new Desktop();
    const instance = form(desktop);
    const fired: string[] = [];
    refusing(desktop, ['Form1'], fired);

    await expect(desktop.runFormLifecycle(instance)).resolves.toBe(false);
    expect(desktop.forms).not.toContain(instance);
  });
});
