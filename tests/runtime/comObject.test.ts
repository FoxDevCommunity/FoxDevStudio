/**
 * COM objects as a FoxPro program holds them.
 *
 * The bridge itself is Windows and a native addon, so what is tested here is everything on this
 * side of it: that `CREATEOBJECT` of a name no program defines reaches COM, that members read,
 * write and chain through the desktop by handle, and that a COM failure arrives as a VFP error
 * rather than a crash.
 */

import { afterEach, describe, expect, it, vi } from 'vitest';
import { Desktop } from '@shared/runtime/objectModel';
import { HostError } from '@shared/runtime/host';
import type { OleValue } from '@shared/ipc/api';
import { createMemoryApi } from '@renderer/api/memoryApi';
import { setApi } from '@renderer/api/foxdev';
import { UnsupportedComObject, createComObject, notSupported } from '@renderer/runtime/comObject';

/** A stand-in for the addon: one object with a property, a method and a child object. */
function fakeCom() {
  const props = new Map<string, OleValue>([['Version', { kind: 'string', text: '16.0' }]]);
  const api = createMemoryApi();
  const calls: string[] = [];
  api.ole = {
    available: () => true,
    create: (progId) => {
      calls.push(`create ${progId}`);
      if (progId === 'Nope.Nothing') throw new Error('Class not registered');
      return 1;
    },
    active: (name) => {
      calls.push(`active ${name}`);
      if (name === 'Nope.Nothing') throw new Error('Operation unavailable');
      return 1;
    },
    get: (handle, name) => {
      calls.push(`get ${handle}.${name}`);
      if (name === 'Documents') return { kind: 'object', handle: 2 };
      return props.get(name) ?? { kind: 'null' };
    },
    set: (handle, name, value) => {
      calls.push(`set ${handle}.${name}`);
      props.set(name, value);
    },
    call: (handle, name, args) => {
      calls.push(`call ${handle}.${name}(${args.map((a) => a.text ?? a.num ?? a.flag).join(',')})`);
      if (name === 'Boom') throw new Error('OLE IDispatch exception: it went wrong');
      return { kind: 'number', num: 42 };
    },
    release: () => calls.push('release'),
    releaseAll: () => calls.push('releaseAll'),
  };
  setApi(api);
  return { calls, props };
}

afterEach(() => setApi(undefined));

describe('a COM object', () => {
  it('reads, writes and chains through the desktop', () => {
    const { calls, props } = fakeCom();
    const desktop = new Desktop();
    const word = desktop.hostHandle(createComObject('Word.Application', desktop));

    expect(desktop.getProp(word, 'Version')).toBe('16.0');
    desktop.setProp(word, 'Visible', true);
    expect(props.get('Visible')).toEqual({ kind: 'bool', flag: true });

    // oWord.Documents is another object: the desktop gives it a handle of its own
    const documents = desktop.getMember(word, 'Documents');
    expect(documents).toBe('prop');
    const value = desktop.getProp(word, 'Documents') as { $obj: number };
    expect(typeof value.$obj).toBe('number');
    expect(desktop.callMethod(value.$obj, 'Add', ['report.doc'])).toBe(42);
    expect(calls).toContain('call 2.Add(report.doc)');
  });

  it('reports a COM failure as the error VFP gives', () => {
    fakeCom();
    const desktop = new Desktop();
    const word = desktop.hostHandle(createComObject('Word.Application', desktop));

    try {
      desktop.callMethod(word, 'Boom', []);
      expect.unreachable('the call should have thrown');
    } catch (error) {
      expect(error).toBeInstanceOf(HostError);
      expect((error as HostError).code).toBe(1429);
      expect((error as HostError).message).toContain('it went wrong');
    }
  });

  it('says a class is not found when COM cannot make it', () => {
    fakeCom();
    const desktop = new Desktop();
    expect(() => createComObject('Nope.Nothing', desktop)).toThrow(/not registered/);
  });

  it('says the class is not found at all where there is no COM', () => {
    setApi(createMemoryApi());
    const desktop = new Desktop();
    const attempt = vi.fn(() => createComObject('Word.Application', desktop));
    expect(attempt).toThrow(/Class definition WORD.APPLICATION is not found/);
  });

  // Some classes are refused on purpose rather than not built yet, and the two look identical
  // from the Output window unless the runtime says which it is. The list and the reasons are in
  // docs/not-supported.md.
  it('refuses a class it will never support, by name and with the reason', () => {
    fakeCom();
    const desktop = new Desktop();
    try {
      createComObject('Agent.Control.2', desktop);
      expect.unreachable('the class should have been refused');
    } catch (error) {
      expect(error).toBeInstanceOf(HostError);
      expect((error as HostError).code).toBe(1733);
      expect((error as HostError).message).toBe(
        'Agent.Control.2 is not supported: Microsoft Agent was withdrawn from Windows after Vista and cannot be installed.',
      );
    }
    // a class nobody has refused still reaches COM
    expect(notSupported('Word.Application')).toBeUndefined();
  });

  // A form designed against a server file records the class in brackets, and that names the
  // same control: Solution/Ole/mmsample carries the MCI control as `(mci32.ocx)`.
  it('knows a refused class by the file a designed form recorded it under', () => {
    expect(notSupported('(mci32.ocx)')).toMatch(/32-bit server/);
    expect(notSupported('MCI.MMControl.1')).toMatch(/32-bit server/);
    expect(notSupported('(shdocvw.dll)')).toBeUndefined();
  });

  // A control on a form is not a CREATEOBJECT: the form has to open, so the refusal waits on
  // the members rather than stopping the control being there at all.
  it('lets the control exist and refuses every member of it', () => {
    const refused = new UnsupportedComObject('(mci32.ocx)', 'the Multimedia MCI control ships only as a 32-bit server.');
    expect(refused.member()).toBe('prop');
    expect(() => refused.get()).toThrow(/mci32.ocx is not supported/);
    expect(() => refused.set()).toThrow(/mci32.ocx is not supported/);
    expect(() => refused.call()).toThrow(/mci32.ocx is not supported/);
  });
});
