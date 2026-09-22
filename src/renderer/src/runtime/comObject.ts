/**
 * A COM object, as a FoxPro program holds it.
 *
 * `CREATEOBJECT("Word.Application")` gives back one of these. It is a `HostObject` like the
 * emulated controls are, so the desktop hands it a handle and routes property reads, writes and
 * method calls to it; each of those crosses to the main process, which is where the object
 * actually lives.
 *
 * Members are not classified in advance: COM has no way to ask what a name is without invoking
 * it, and invoking a name to find out what it is would run methods nobody called. Every unknown
 * name is reported as a property, and the value comes back as one - an object among them, which
 * is how `oWord.Documents.Add()` walks the chain.
 */

import type { Desktop } from '@shared/runtime/objectModel';
import type { HostObject } from '@shared/runtime/oleObjects';
import type { VmValue } from '@shared/runtime/values';
import type { OleValue } from '@shared/ipc/api';
import { HostError } from '@shared/runtime/host';
import { getApi } from '../api/foxdev';

/** VFP reports a failure inside an automation object as error 1429. */
const OLE_ERROR = 1429;

/**
 * The two answers COM gives when the name is not a class at all.
 *
 * `CO_E_CLASSSTRING` is what looking a ProgID up in the registry says when there is no such
 * ProgID, and `REGDB_E_CLASSNOTREG` is what creating a class id says when nothing is registered
 * under it. Either way there is no class of that name, which is not an automation failure:
 * measured, `CREATEOBJECT("NoSuchThingAtAll")` and `CREATEOBJECT("Agent.Control.2")` on a
 * machine without Microsoft Agent both raise 1733 in Visual FoxPro 9, the same error a class
 * name with no `DEFINE CLASS` behind it raises. Every other failure is the server refusing,
 * which is 1429.
 */
const NO_SUCH_CLASS = new Set([0x800401f3, 0x80040154]);

/**
 * The COM classes this runtime refuses on purpose, and why each one is refused.
 *
 * Everything else this project has not finished is a gap: it is on a list and it is going to be
 * built. These are the other thing, and the difference matters because from the Output window
 * the two look identical. "Class definition is not found" cannot tell a developer "nobody has
 * written this yet" from "this will never work, and here is why", so each of these is refused by
 * name and says its reason. The reasoning is in `docs/not-supported.md`; this table is the rule
 * the runtime keeps, rather than a branch per class scattered through the factory.
 *
 * `named` is matched against the class as the program writes it, which is a ProgID for
 * `CREATEOBJECT` and `AddObject`, and the server's own file for an OLE control that a form
 * designed long ago recorded that way.
 */
const NOT_SUPPORTED: readonly { readonly named: readonly string[]; readonly reason: string }[] = [
  {
    named: ['Agent.Control.2', 'Agent.Control'],
    reason: 'Microsoft Agent was withdrawn from Windows after Vista and cannot be installed.',
  },
  {
    named: ['Messenger.UIAutomation.1', 'Messenger.UIAutomation'],
    reason: 'Windows Messenger was shut down in 2013 and there is no service behind it.',
  },
  {
    named: ['mci32.ocx', 'MCI.MMControl.1', 'MCI.MMControl'],
    reason: 'the Multimedia MCI control ships only as a 32-bit server, which this process cannot load.',
  },
];

/**
 * The reason a class is refused, or nothing when it is not one of them.
 *
 * A form designed against a server file records the class in brackets - `(mci32.ocx)` - so the
 * brackets come off before the name is looked up. Nothing else is normalised: a ProgID is
 * matched as it is written, case aside.
 */
export function notSupported(named: string): string | undefined {
  const bare = named.trim().replace(/^\((.*)\)$/, '$1').trim();
  return NOT_SUPPORTED.find((entry) => entry.named.some((n) => n.toLowerCase() === bare.toLowerCase()))?.reason;
}

/**
 * What a program is told when it asks for one of them.
 *
 * The number is the one an unregistered class already answers with, because that is what the
 * product says for a class it cannot make and this is a class this runtime will not make; what
 * distinguishes the two is the sentence, which names the class and gives the reason.
 */
export function notSupportedError(named: string, reason: string): HostError {
  const bare = named.trim().replace(/^\((.*)\)$/, '$1').trim();
  return new HostError(1733, `${bare} is not supported: ${reason}`);
}

/**
 * The stand-in an ActiveX control of a refused class gets.
 *
 * A control that cannot be made still exists on the form - the form loads, and the rest of it
 * works - so what is refused is every member of it, one by one, with the reason. Without this a
 * program would hear "Property HWNDDISPLAY is not found", which says nothing about why.
 */
export class UnsupportedComObject implements HostObject {
  readonly lazy = true;

  constructor(
    readonly named: string,
    private readonly reason: string,
  ) {}

  get className(): string {
    return this.named;
  }

  member(): 'prop' | 'method' | 'none' {
    return 'prop';
  }

  get(): VmValue | HostObject | undefined {
    throw notSupportedError(this.named, this.reason);
  }

  set(): void {
    throw notSupportedError(this.named, this.reason);
  }

  call(): VmValue | HostObject | undefined {
    throw notSupportedError(this.named, this.reason);
  }
}

/**
 * The error a failed COM call becomes.
 *
 * The addon puts the HRESULT at the front of the message - `0x800401f3|Invalid class string` -
 * so that it can be read back here, where what a FoxPro program sees is decided.
 */
function comError(error: unknown, named: string): HostError {
  const message = error instanceof Error ? error.message : String(error);
  const code = /^0x([0-9a-f]{8})\|/i.exec(message);
  if (code && NO_SUCH_CLASS.has(Number.parseInt(code[1] ?? '', 16))) {
    return new HostError(1733, `Class definition ${named.toUpperCase()} is not found.`);
  }
  return new HostError(OLE_ERROR, message);
}

export class ComObject implements HostObject {
  /** Members are only found by asking COM, so the desktop must not evaluate one to classify it. */
  readonly lazy = true;

  constructor(
    readonly progId: string,
    readonly comHandle: number,
    private readonly desktop: Desktop,
  ) {}

  get className(): string {
    return this.progId;
  }

  member(): 'prop' | 'method' | 'none' {
    return 'prop';
  }

  get(name: string): VmValue | HostObject | undefined {
    return this.answer(() => getApi().ole.get(this.comHandle, name, []));
  }

  set(name: string, value: VmValue): void {
    this.answer(() => {
      getApi().ole.set(this.comHandle, name, this.toOle(value));
      return { kind: 'null' };
    });
  }

  call(name: string, args: VmValue[]): VmValue | HostObject | undefined {
    return this.answer(() => getApi().ole.call(this.comHandle, name, args.map((a) => this.toOle(a))));
  }

  /** `RELEASE oWord`: the object goes when nobody holds it. */
  release(): void {
    try {
      getApi().ole.release(this.comHandle);
    } catch {
      // letting go of an object that is already gone is not a failure
    }
  }

  /** Runs a call and turns a COM failure into the error a FoxPro program can catch. */
  private answer(run: () => OleValue): VmValue | HostObject | undefined {
    let value: OleValue;
    try {
      value = run();
    } catch (error) {
      throw new HostError(OLE_ERROR, error instanceof Error ? error.message : String(error));
    }
    switch (value.kind) {
      case 'number':
        return value.num ?? 0;
      case 'string':
        return value.text ?? '';
      case 'bool':
        return value.flag ?? false;
      case 'date':
        return { $dt: (value.num ?? 0) / 1000 };
      case 'object':
        return new ComObject(this.progId, value.handle ?? 0, this.desktop);
      default:
        return null;
    }
  }

  /** A value on its way to COM. An object argument has to be a COM object to be one at all. */
  private toOle(value: VmValue): OleValue {
    if (value === null || value === undefined) return { kind: 'null' };
    if (typeof value === 'number') return { kind: 'number', num: value };
    if (typeof value === 'string') return { kind: 'string', text: value };
    if (typeof value === 'boolean') return { kind: 'bool', flag: value };
    if (typeof value === 'object' && '$dt' in value) return { kind: 'date', num: value.$dt * 1000 };
    if (typeof value === 'object' && '$obj' in value) {
      const held = this.desktop.hostObject(value);
      if (held instanceof ComObject) return { kind: 'object', handle: held.comHandle };
    }
    return { kind: 'null' };
  }
}

/**
 * Makes a COM object, or says why it could not.
 *
 * A name with no class definition behind it is where this comes in: in Visual FoxPro that is
 * exactly what CREATEOBJECT does next.
 */
export function createComObject(progId: string, desktop: Desktop): ComObject {
  const api = getApi();
  const refused = notSupported(progId);
  if (refused !== undefined) {
    throw notSupportedError(progId, refused);
  }
  if (!api.ole.available()) {
    throw new HostError(1733, `Class definition ${progId.toUpperCase()} is not found.`);
  }
  try {
    return new ComObject(progId, api.ole.create(progId), desktop);
  } catch (error) {
    throw comError(error, progId);
  }
}

/**
 * `GETOBJECT()`: the object already running under that name, or the one a file stands for.
 *
 * Unlike CREATEOBJECT() this never starts a server that is not there; when nothing answers, the
 * COM message says so.
 */
export function activeComObject(name: string, className: string, desktop: Desktop): ComObject {
  const api = getApi();
  if (!api.ole.available()) {
    throw new HostError(1733, `Class definition ${name.toUpperCase()} is not found.`);
  }
  try {
    return new ComObject(name, api.ole.active(name, className), desktop);
  } catch (error) {
    throw comError(error, className.trim() === '' ? name : className);
  }
}

/** Lets go of every COM object a run made. */
export function releaseComObjects(): void {
  try {
    if (getApi().ole.available()) getApi().ole.releaseAll();
  } catch {
    // nothing to release when there was never a bridge
  }
}

/**
 * Lets go of every Visual FoxPro library a run loaded, and of the process hosting them.
 *
 * A library is loaded for the length of a run and no longer: the next one starts with the
 * language the product ships with, as it does when Visual FoxPro is started again.
 */
export function releaseLibraries(): void {
  try {
    getApi().library.releaseAll();
  } catch {
    // nothing to release when there was never a host
  }
}
