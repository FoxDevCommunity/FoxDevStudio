/**
 * The data environment a form carries: the tables it opens when it loads, as objects a program
 * can read.
 *
 * Visual FoxPro gives every form a DataEnvironment object holding a Cursor object per table and a
 * Relation object per relation between them, and form code reads them - `THISFORM.DataEnvironment
 * .Cursor1.Alias` - as often as it reads a control. Nothing here opens a table: the session does
 * that by running the USE a program would have written. This is what the form says should be
 * opened, and what the program sees of it afterwards.
 */

import type { FormCursor } from '../form/schema';
import { DATA_METHODS } from '../language/foxproMethods';
import { CONTROL_DESCRIPTORS, OBJECT_DESCRIPTORS } from '../registry';
import { isCollection, type HostObject } from './oleObjects';
import type { VmValue } from './values';

/** An object with named values and no behaviour of its own: what the data environment is made of. */
export class DataObject implements HostObject {
  /** What a drag is carrying, by format: SetData puts it here and GetData takes it off. */
  protected readonly carried = new Map<number, VmValue>();

  readonly className: string;
  private readonly values = new Map<string, VmValue>();
  private readonly names = new Map<string, string>();

  constructor(className: string, values: Record<string, VmValue> = {}) {
    this.className = className;
    for (const [name, meta] of Object.entries(defaultsOf(className))) this.put(name, meta);
    for (const [name, value] of Object.entries(values)) this.put(name, value);
  }

  private put(name: string, value: VmValue): void {
    this.values.set(name.toUpperCase(), value);
    this.names.set(name.toUpperCase(), name);
  }

  member(name: string): 'prop' | 'method' | 'none' {
    if (this.values.has(name.toUpperCase())) return 'prop';
    return METHODS.has(name.toUpperCase()) ? 'method' : 'none';
  }

  get(name: string): VmValue | HostObject | undefined {
    const upper = name.toUpperCase();
    if (upper === 'NAME') return this.names.get('NAME') ? this.values.get('NAME') : this.className;
    if (upper === 'CLASS' || upper === 'BASECLASS') return this.className;
    return this.values.get(upper);
  }

  set(name: string, value: VmValue): void {
    this.put(this.names.get(name.toUpperCase()) ?? name, value);
  }

  call(name: string, args: VmValue[]): VmValue | HostObject | Promise<VmValue> | undefined {
    // `oAdapter.Tables(1)` is a collection read by number, which reaches the object as a call
    // rather than as a property; every collection any object holds answers the same way
    const held = this.get(name);
    if (isCollection(held)) return args.length > 0 ? held.at(Number(args[0] ?? 0)) : held;
    switch (name.toUpperCase()) {
      // what a drag is carrying: put there by the source, taken off by the target
      case 'SETDATA': {
        const format = args.length > 1 ? Number(args[1]) : TEXT_FORMAT;
        this.carried.set(format, args[0] ?? null);
        return true;
      }
      case 'GETDATA': {
        const format = args.length > 0 ? Number(args[0]) : TEXT_FORMAT;
        return this.carried.get(format) ?? '';
      }
      case 'CLEARDATA':
        this.carried.clear();
        return true;
      case 'GETFORMAT':
        return this.carried.has(args.length > 0 ? Number(args[0]) : TEXT_FORMAT);
      case 'SETFORMAT':
        // saying a format is on offer without the data itself: the offer is the format
        this.carried.set(args.length > 0 ? Number(args[0]) : TEXT_FORMAT, '');
        return true;
      default:
        break;
    }
    // the methods a data environment answers to do their work through the data engine, which
    // the program reaches with the commands themselves; here they simply succeed
    return METHODS.has(name.toUpperCase()) ? true : undefined;
  }
}

/**
 * An object of a class the registry describes: its properties start as the registry says and
 * it answers to the names the descriptor lists. This is what `CREATEOBJECT("Exception")` and
 * the rest of the classes that are not controls make.
 */
export function registryObject(className: string): DataObject | undefined {
  const found = matching(className);
  return found ? new DataObject(found, { Name: found }) : undefined;
}

/**
 * Every class a program can ask CREATEOBJECT() for: the controls a form is built from, the
 * objects Visual FoxPro provides itself, and the form. This is what ALANGUAGE(a, 3) lists.
 */
export function baseClassNames(): string[] {
  const names = new Set<string>(['Form', 'FormSet']);
  for (const name of Object.keys(CONTROL_DESCRIPTORS)) names.add(name);
  for (const name of Object.keys(OBJECT_DESCRIPTORS)) names.add(name);
  return [...names].sort((a, b) => a.localeCompare(b));
}

/** The registry class of that name, whatever case the program wrote it in. */
function matching(className: string): string | undefined {
  return Object.keys(OBJECT_DESCRIPTORS).find((k) => k.toLowerCase() === className.toLowerCase());
}

/** The methods a data environment and its cursors answer to. */
const METHODS = new Set([...DATA_METHODS, 'INIT', 'DESTROY', 'SETDATA', 'GETDATA', 'CLEARDATA', 'SETFORMAT', 'GETFORMAT']);

/**
 * What a DataObject is carrying, by format number.
 *
 * OLE drag-and-drop hands the dragged data over on a DataObject: the source puts it there in
 * OLEStartDrag and the target takes it off in OLEDragDrop. 1 is text, which is the format
 * everything uses unless it says otherwise.
 */
const TEXT_FORMAT = 1;

/** What a class of the registry says its properties start as. */
function defaultsOf(className: string): Record<string, VmValue> {
  const descriptor = OBJECT_DESCRIPTORS[matching(className) ?? className];
  if (!descriptor) return {};
  const out: Record<string, VmValue> = {};
  for (const p of descriptor.properties) {
    if (typeof p.default === 'number' || typeof p.default === 'string' || typeof p.default === 'boolean') {
      out[p.name] = p.default;
    }
  }
  return out;
}

/** One cursor of a form's data environment, as the form document describes it. */
export class DataCursor extends DataObject {
  constructor(
    readonly name: string,
    cursor: FormCursor,
  ) {
    super('Cursor', {
      Name: name,
      Alias: cursor.alias,
      CursorSource: cursor.source,
      Database: cursor.database ?? '',
      Exclusive: cursor.exclusive === true,
      Order: cursor.order ?? '',
    });
  }
}

/** The data environment itself: its cursors by name, and its relations. */
export class DataEnvironment extends DataObject {
  readonly cursors: DataCursor[] = [];

  constructor(cursors: FormCursor[] | undefined) {
    super('DataEnvironment', { Name: 'Dataenvironment' });
    for (const [i, cursor] of (cursors ?? []).entries()) {
      this.cursors.push(new DataCursor(cursorName(i), cursor));
    }
    const first = this.cursors[0]?.get('Alias');
    this.set('InitialSelectedAlias', typeof first === 'string' ? first : '');
  }

  override member(name: string): 'prop' | 'method' | 'none' {
    const child = this.cursor(name);
    if (child) return 'prop';
    if (name.toUpperCase() === 'COUNT' || name.toUpperCase() === 'CONTROLCOUNT') return 'prop';
    return super.member(name);
  }

  override get(name: string): VmValue | HostObject | undefined {
    const child = this.cursor(name);
    if (child) return child;
    if (name.toUpperCase() === 'COUNT' || name.toUpperCase() === 'CONTROLCOUNT') return this.cursors.length;
    return super.get(name);
  }

  /** The cursor of that name, as `THISFORM.DataEnvironment.Cursor1` asks for it. */
  cursor(name: string): DataCursor | undefined {
    return this.cursors.find((c) => c.name.toUpperCase() === name.toUpperCase());
  }
}

/** The name a cursor answers to: the one VFP gives it, which is Cursor1, Cursor2 and so on. */
function cursorName(index: number): string {
  return `Cursor${index + 1}`;
}
