/**
 * The live object tree a running form is made of: the VFP-side counterpart of the design
 * document. React renders these, and compiled FoxPro code reads and writes them through the
 * host bridge, so this file is the single source of truth for runtime state.
 *
 * Deliberately React-free and synchronous. Each object is its own `useSyncExternalStore`
 * source: mutating a property bumps a version and notifies only that object's subscribers,
 * so `THISFORM.lblGreeting.Caption = x` re-renders one label rather than the whole form.
 */

import type { ControlNode, ControlType, FormCursor, FormNode, PropValue } from '../form/schema';
import { BASE_CLASS_MEMBERS, getObjectDescriptor, getPropertyMeta, resolveProps } from '../registry';
import type { EventMeta } from '../registry';
import { FOXPRO_METHODS, REPORT_METHODS } from '../language/foxproMethods';
import { Collection } from './collection';
import { registryObject } from './dataEnvironment';
import { HostError, type HostReads, type RuntimeError } from './host';
import { isDate, isDateTime, propToVm, vmToProp, type VmArray, type VmValue } from './values';
import { baseClassToControlType } from '../vfp/importForm';
import { isNonVisualBaseClass } from './classDef';
import { oleEmulation } from '../vfp/oleControl';
import { DataEnvironment } from './dataEnvironment';
import { createOleControl, type HostObject } from './oleObjects';
import type { EventOutcome } from './scheduler';
import type { LibraryHost, LibraryValue } from './libraryHost';

/**
 * What a library function is handed, and what it hands back.
 *
 * A library declares the type of every parameter it takes and the host converts to it, so all
 * that has to cross is which of FoxPro's own kinds the value is. A date or an object has no
 * place in a ParamBlk this host builds and arrives as nothing, which a library reads as an
 * empty parameter.
 */
function toLibraryValue(v: VmValue): LibraryValue {
  // `@cValue`: the library is handed the variable, and what it stores comes back to it
  if (typeof v === 'object' && v !== null && '$ref' in v) return { kind: 'ref', value: toLibraryValue(v.$val) };
  if (typeof v === 'string') return { kind: 'string', text: v };
  if (typeof v === 'number') return { kind: 'number', num: v };
  if (typeof v === 'boolean') return { kind: 'logical', flag: v };
  return { kind: 'none' };
}

/**
 * Measured in Visual FoxPro 9 with the API sample hello.fll, whose one function prints and never
 * sets a return value: the product answers `.T.`, the same thing a FoxPro procedure with no
 * RETURN answers. A date from `_RetDateStr` comes over as the text the library wrote and is left
 * as text, because no library here returns one and what the product makes of it has not been
 * measured.
 */
function fromLibraryValue(v: LibraryValue): VmValue {
  switch (v.kind) {
    case 'string':
      return v.text;
    case 'number':
      return v.num;
    case 'logical':
      return v.flag;
    case 'date':
    case 'datetime':
      return v.text;
    default:
      return true;
  }
}

/** Handle 0 is `_SCREEN`, the desktop that owns every form. */
export const SCREEN_HANDLE = 0;

/**
 * The application object: what `_VFP` and every object's `Application` property answer with. It
 * is the environment itself rather than anything on the screen, so it has a handle of its own
 * that no form can take.
 *
 * A large positive number rather than -1, because a handle crosses into the VM as an unsigned
 * one: -1 arrived there as 0, which is `_SCREEN`, and `_VFP.FullName` was answered by the
 * screen - which has no FullName - rather than by the application.
 */
export const APP_HANDLE = 0x7fff_ffff;

/**
 * What OS() reports. A FoxPro program that guards on the Windows version gets a truthful
 * answer instead of a zero that makes it refuse to run. Chromium's user agent is the only
 * platform fact available here, and it carries the version on Windows and macOS.
 */
export function detectOsInfo(userAgent = globalThis.navigator?.userAgent ?? ''): string {
  const windows = /Windows NT (\d+)(?:\.(\d+))?/.exec(userAgent);
  if (windows) return `Windows|${windows[1]}|${windows[2] ?? '0'}|0`;
  const mac = /Mac OS X (\d+)[._](\d+)/.exec(userAgent);
  if (mac) return `Darwin|${mac[1]}|${mac[2]}|0`;
  if (/Linux|X11/.test(userAgent)) return 'Linux|0|0|0';
  // no user agent at all: a test environment, or the CLI. Claim the version this app needs.
  return 'Windows|10|0|0';
}

/** What SetFocus() needs from a DOM node. Structural, so `src/shared` stays free of DOM types. */
/** One member of an object, as AMEMBERS() lists it and GETPEM() reads it. */
export interface MemberEntry {
  /** Upper-cased, which is how AMEMBERS() writes it. */
  name: string;
  kind: 'Property' | 'Event' | 'Method' | 'Object';
  /** Declared by the Visual FoxPro base class, rather than by a class definition above it. */
  native: boolean;
  /** Put there by ADDPROPERTY() while the program ran. */
  added: boolean;
  readOnly: boolean;
  /** It holds something other than what its class starts it at. */
  changed: boolean;
  /** What it holds now; nothing for a method or an event. */
  value?: VmValue;
}

export interface FocusableElement {
  focus(): void;
}

/** Where a property write came from; decides which change event fires. */
export type WriteSource = 'program' | 'interactive';

export type Dispatch = (obj: RuntimeObject, event: string, args?: VmValue[]) => EventOutcome | Promise<EventOutcome> | null;

/**
 * A value a host object answered with, where only one that is already in hand will do: a
 * property read runs inside wasm, so work that is still going on cannot be waited for there.
 */
function settled(value: VmValue | Promise<VmValue> | undefined): VmValue | undefined {
  return value instanceof Promise ? undefined : value;
}

const METHOD_NAMES = new Set([...FOXPRO_METHODS, ...REPORT_METHODS]);

/** One entry of a ComboBox or ListBox item list, as VFP's parallel indexed properties see it. */
export interface ListItem {
  /**
   * The row, column by column. A list has `ColumnCount` columns and `AddListItem` puts one
   * value in one of them - `lo.AddListItem(cName, i, 1)` then `lo.AddListItem(cKey, i, 2)` is
   * two halves of one row, not two rows - so the text of the row is column one and `Value`
   * comes from whichever column `BoundColumn` names.
   */
  columns: string[];
  text: string;
  data: VmValue;
  picture: string;
  selected: boolean;
  /** What the item is called by the ItemID properties: it stays with the item as the list moves. */
  id: number;
}

/**
 * A class name as the product writes it back: the first letter upper-cased, the rest lower.
 *
 * Measured in Visual FoxPro 9 with a library whose class is called `MyMixedCase`: `Class` comes
 * back "Mymixedcase" however the file spells it, and so do `BaseClass` and `ParentClass`. Every
 * BaseClass in the measured table of base classes is spelled the same way - "Commandbutton",
 * "Pageframe" - so this is the product's one rule for a class name and not a habit of `.vcx`
 * files.
 */
function spellClass(word: string): string {
  const name = word.trim();
  return name === '' ? name : name[0]!.toUpperCase() + name.slice(1).toLowerCase();
}

/** The handle in an object value, or nothing when the value is not an object. */
function handleOf(value: VmValue): number | undefined {
  return value !== null && typeof value === 'object' && '$obj' in value ? value.$obj : undefined;
}

/**
 * The shape in a property name that carries subscripts: `aPoly[1,1]`, `aRows(3)`.
 *
 * `ADDPROPERTY()` takes the name and the size in one string, and a form's `^aReports[1,0]` line
 * says the same thing, so both spellings are read here. `cols` is 0 for a list of one dimension,
 * which is the same convention the stored array uses.
 */
function arraySubscripts(name: string): { name: string; rows: number; cols: number; twoDimensional: boolean } | null {
  const match = /^([^[(]+)[[(]\s*([^,\])]+?)\s*(?:,\s*([^\])]+?)\s*)?[\])]\s*$/.exec(name.trim());
  if (!match) return null;
  const rows = Number(match[2]);
  const cols = match[3] === undefined ? 0 : Number(match[3]);
  if (!Number.isFinite(rows) || !Number.isFinite(cols)) return null;
  return { name: match[1]!.trim(), rows: Math.trunc(rows), cols: Math.trunc(cols), twoDimensional: match[3] !== undefined };
}

/** Indexed properties a list control answers from its item list rather than from a stored value. */
const LIST_ARRAYS = new Set(['LIST', 'LISTITEM', 'SELECTED', 'PICTURE', 'ITEMDATA']);

/**
 * The two that translate between a row's position and the id it keeps, which a list answers by
 * subscript as well as by call: the Foundation Classes' mover writes
 * `this.lstLeft.ListItemID = this.lstLeft.IndexToItemID[1]`, because brackets and parentheses
 * index alike in FoxPro. They are read-only, unlike the item list the rest of them stand for.
 */
const LIST_ID_ARRAYS = new Set(['INDEXTOITEMID', 'ITEMIDTOINDEX']);

/** Everything a list control answers beyond its descriptor, indexed or not. */
const LIST_PROPS = new Set([...LIST_ARRAYS, ...LIST_ID_ARRAYS, 'LISTCOUNT', 'LISTINDEX']);

/** Controls that keep an item list. */
const LIST_CONTROLS = new Set<string>(['ComboBox', 'ListBox']);

/**
 * The rows a Value row source stands for: `RowSource = "1st Quarter,2nd Quarter"` with
 * `RowSourceType = 1`.
 *
 * Measured in Visual FoxPro 9. The text between two commas is the row exactly as written -
 * `"a, b ,c"` gives `" b "` for row two, spaces and all - and only a trailing empty row is
 * dropped, one of them: `"a,b,,"` is three rows ending in an empty one, `","` is one empty row
 * and `""` is none at all.
 */
function valueListRows(source: string): string[] {
  const rows = source.split(',');
  if (rows[rows.length - 1] === '') rows.pop();
  return rows;
}

/**
 * The members a container answers by position: `Pages(2)`, `Columns(1)`, `Buttons(3)`.
 *
 * Each one is the container's own children of a kind, and `Controls` and `Objects` are every
 * child whatever it is. The key is which control type owns the member, from the measured table
 * of base classes: a page frame has `Pages` and no `Controls` at all - it has no `ControlCount`
 * either - and a grid has `Columns` the same way.
 */
const MEMBER_ARRAYS: Record<string, { owners?: ReadonlySet<string>; of?: ReadonlySet<string> }> = {
  CONTROLS: {},
  OBJECTS: {},
  PAGES: { owners: new Set(['PageFrame']), of: new Set(['page']) },
  COLUMNS: { owners: new Set(['Grid']), of: new Set(['column']) },
  BUTTONS: { owners: new Set(['CommandGroup', 'OptionGroup']), of: new Set(['commandbutton', 'optionbutton']) },
};

/** Intrinsic properties every object answers, whatever the descriptor says. */
const INTRINSICS = new Set(['NAME', 'PARENT', 'CLASS', 'BASECLASS', 'PARENTCLASS', 'CLASSLIBRARY', 'CONTROLCOUNT', 'CONTROLS', 'OBJECTS', 'HWND']);

export class RuntimeObject {
  readonly children: RuntimeObject[] = [];
  /** Bumped on every change; the React store snapshot. */
  version = 0;
  alive = true;
  parent: RuntimeObject | null = null;
  /**
   * The Name a program gave it while it ran - `oForm.Name = "one"` - which is what Name reads
   * from then on (measured). The node keeps the name the object was built with, because that is
   * how its methods are found.
   */
  renamed: string | null = null;
  /**
   * The compiled module holding this object's own method bodies, or -1 when whatever contains
   * it holds them. A form has one because it is a document; so does an object built from a
   * class library, because the class is a document of its own wherever the object ends up -
   * a control a Grid column made for itself still runs the library's code, not the form's.
   */
  module = -1;
  /**
   * The class the object is of, when that is not simply its base class: what `Class` answers.
   * Set for an object built from `DEFINE CLASS` and for one built from a class library; a
   * control on a designed form has none, because the importer flattens a subclass into the
   * base class it stands on.
   */
  className: string | null = null;
  /** The class its class was built from, for `ParentClass`; empty when it stands on a base class. */
  parentClass = '';
  /**
   * What the object answers for `BaseClass`, when the node it was built from does not say.
   *
   * A class definition names its base class in words - `custom`, `commandbutton` - and an object
   * of a class with nothing on screen is a form here for want of anywhere else to keep a module,
   * so without this a Custom class would answer "Form".
   */
  declaredBaseClass = '';
  /**
   * The document a form was built from, which is what `SYS(1271, oObject)` answers and how form
   * code finds the folder it is running out of. Only a form has one: measured, the product
   * answers .F. for everything else, an object out of a class library included.
   */
  file = '';
  /** The `.vcx` the object's class was read out of, for `ClassLibrary`. */
  classLibrary = '';

  private readonly listeners = new Set<() => void>();
  /** Runtime overrides on top of the design-time resolved props. */
  private readonly values = new Map<string, PropValue>();
  /** Lower-cased property name -> declared name, for VFP's case-insensitive access. */
  private readonly propNames = new Map<string, string>();
  private readonly childrenByName = new Map<string, RuntimeObject>();
  private readonly eventNames = new Map<string, EventMeta>();
  /** Lower-cased method names the class answers to, as the product lists them. */
  private readonly methodNames = new Set<string>();
  /** Lower-cased property name -> the error the product raises when a program writes it. */
  private readonly readOnlyNames = new Map<string, number>();
  private element: FocusableElement | null = null;
  /** Lower-cased method names a `DEFINE CLASS` gave this object, so they answer as members. */
  readonly classMethods = new Set<string>();
  /** Lower-cased names ADDPROPERTY put there, which AMEMBERS() reports apart from the declared. */
  private readonly addedProps = new Set<string>();
  /** SetFocus() called before the control rendered; applied when it binds. */
  private focusPending = false;
  /**
   * Array-valued properties: `DIMENSION aRGB[3]` in a class body, and the `^aIcon[5,2]` lines a
   * form writes for the arrays it adds for itself. `cols` is 0 for a list of one dimension.
   */
  private readonly arrays = new Map<string, { values: VmValue[]; cols: number }>();
  /** ComboBox/ListBox items added with AddItem, which VFP keeps apart from RowSource. */
  readonly items: ListItem[] = [];

  constructor(
    readonly handle: number,
    readonly node: ControlNode | FormNode,
    readonly desktop: Desktop,
  ) {
    const desc = getObjectDescriptor(node);
    for (const p of desc.properties) {
      this.propNames.set(p.name.toLowerCase(), p.name);
      if (p.readOnly) this.readOnlyNames.set(p.name.toLowerCase(), p.refusesWrite ?? 1743);
    }
    for (const e of desc.events) this.eventNames.set(e.name.toLowerCase(), e);
    for (const m of desc.methods ?? []) this.methodNames.add(m.toLowerCase());
    // properties the descriptor does not declare: a user property from DEFINE CLASS, or one a
    // Visual FoxPro form carried in. Without this they would be stored but unreadable.
    for (const name of Object.keys(node.props)) {
      if (!this.propNames.has(name.toLowerCase())) this.propNames.set(name.toLowerCase(), name);
    }
    // a list control designed with a Value row source has its rows before anything runs, which
    // is what a form's Init reads when it asks its combo box for `List[3]`
    this.rebuildValueList();
  }

  /**
   * Fills the item list from a Value row source, the way the product does whenever either half
   * of that pair is written.
   *
   * Measured in Visual FoxPro 9: writing `RowSource` or `RowSourceType` builds the list again
   * from scratch - even a write of the value the property already holds, which throws away
   * whatever `AddItem` had put there - and any `RowSourceType` other than 1 leaves the list
   * empty however the `RowSource` reads. `AddItem` after that appends a row without touching
   * `RowSource`, and so do `RemoveItem` and `Clear`: the string is what the list was built
   * from, not a picture of what it holds.
   */
  private rebuildValueList(): void {
    if (!this.isListControl) return;
    this.items.length = 0;
    if (Number(this.get('RowSourceType') ?? 0) !== 1) return;
    const source = this.get('RowSource');
    if (typeof source !== 'string') return;
    for (const text of valueListRows(source)) this.addItem(text);
  }

  get name(): string {
    return this.node.name;
  }

  /**
   * `Form` for a form, else the control type: VFP's BaseClass.
   *
   * Spelled the way the product spells it rather than the way the node type is written. A node
   * type is a name this codebase chose - `CommandButton`, `PageFrame`, `OleControl` - and the
   * measured table in `tests/reference/vfp-base-classes.tsv` says Visual FoxPro answers
   * `Commandbutton`, `Pageframe`, `Olecontrol`: the same one rule as every other class name.
   * Until this went through `spellClass` a control designed onto a form answered one spelling
   * and one built from a class library - which already spelled its declared base class - the
   * other, so a form could hold both at once. `$` and `=` are case-sensitive in FoxPro, so the
   * difference is one a sample can see.
   */
  get baseClass(): string {
    if (this.declaredBaseClass !== '') return this.declaredBaseClass;
    return spellClass('type' in this.node ? this.node.type : 'Form');
  }

  get type(): ControlType | 'Form' {
    return 'type' in this.node ? this.node.type : 'Form';
  }

  // ---- subscription (useSyncExternalStore) ----

  subscribe = (listener: () => void): (() => void) => {
    this.listeners.add(listener);
    return () => void this.listeners.delete(listener);
  };

  getSnapshot = (): number => this.version;

  private notify(): void {
    this.version++;
    for (const l of [...this.listeners]) l();
  }

  // ---- properties ----

  /** Declared name for a case-insensitive lookup, or undefined when there is no such property. */
  private canonical(name: string): string | undefined {
    const lower = name.toLowerCase();
    if (this.propNames.has(lower)) return this.propNames.get(lower);
    return this.values.has(lower) || this.objectValues.has(lower) || this.momentValues.has(lower) ? name : undefined;
  }

  has(name: string): boolean {
    return this.canonical(name) !== undefined || INTRINSICS.has(name.toUpperCase());
  }

  /**
   * The object a property holds, by lower-cased name.
   *
   * A property is a scalar everywhere else in this runtime - the designer edits scalars and a
   * document holds scalars - but a running program keeps objects in them all the time:
   * `THIS.oToolbar = CREATEOBJECT("tbrbackcolor")` is how a form keeps the toolbar it put up.
   * They are held apart from the values rather than widening what a property may be, because
   * nothing outside the run has anywhere to put one.
   */
  private readonly objectValues = new Map<string, number>();

  /**
   * The date or datetime a property holds, by lower-cased name.
   *
   * The same problem as an object, and kept the same way: a document holds scalars, so a date
   * written into a property came back as the text it displays as. A date text box starts with
   * `Value = (DATE())` and its InteractiveChange writes `DTOC(THIS.Value)`, which refused the
   * text with "Function argument value, type, or count is invalid" - the sample's own way of
   * reporting that the property had lost its type.
   */
  private readonly momentValues = new Map<string, VmValue>();

  /** The handle a property holds, or nothing when it holds a value. */
  objectValue(name: string): number | undefined {
    return this.objectValues.get(name.toLowerCase());
  }

  /** The date or datetime a property holds, or nothing when it holds something else. */
  momentValue(name: string): VmValue | undefined {
    return this.momentValues.get(name.toLowerCase());
  }

  /** Puts a date or a datetime in a property, in place of whatever it held. */
  setMomentValue(name: string, value: VmValue, source: WriteSource = 'program'): void {
    const key = name.toLowerCase();
    this.objectValues.delete(key);
    this.values.delete(key);
    this.momentValues.set(key, value);
    if (!this.propNames.has(key)) this.propNames.set(key, name);
    this.notify();
    if (key === 'value') {
      void this.desktop.dispatch(this, source === 'interactive' ? 'InteractiveChange' : 'ProgrammaticChange');
    }
  }

  /** Puts an object in a property, in place of whatever it held. */
  setObjectValue(name: string, handle: number): void {
    const key = name.toLowerCase();
    this.objectValues.set(key, handle);
    this.momentValues.delete(key);
    this.values.delete(key);
    if (!this.propNames.has(key)) this.propNames.set(key, name);
    this.notify();
  }

  /** Current value: runtime override, else the design-time value, else the descriptor default. */
  get(name: string): PropValue | undefined {
    const key = name.toLowerCase();
    if (this.values.has(key)) return this.values.get(key);
    const declared = this.propNames.get(key);
    if (declared === undefined) return undefined;
    if (declared in this.node.props) return this.node.props[declared] ?? null;
    return getPropertyMeta(getObjectDescriptor(this.node), declared)?.default ?? null;
  }

  /**
   * Writes a property. A write from code fires ProgrammaticChange, one from the user fires
   * InteractiveChange, matching VFP. The property must already exist: unlike a memory
   * variable, VFP will not create one by assignment (that is what ADDPROPERTY is for).
   */
  set(name: string, value: PropValue, source: WriteSource = 'program'): void {
    const key = name.toLowerCase();
    const before = this.get(name);
    // a property that held an object or a date holds a plain value now
    this.objectValues.delete(key);
    this.momentValues.delete(key);
    this.values.set(key, value);
    if (!this.propNames.has(key)) this.propNames.set(key, name);
    // before the early return below: measured, the list is built again even when the write
    // leaves the property holding what it already held
    if (key === 'rowsource' || key === 'rowsourcetype') this.rebuildValueList();
    if (before === value) return;
    // a container that changes size takes the controls anchored to its edges with it
    if ((key === 'width' || key === 'height') && typeof before === 'number' && typeof value === 'number') {
      this.moveAnchored(key === 'width' ? value - before : 0, key === 'height' ? value - before : 0);
    }
    this.notify();
    if (key === 'value') {
      void this.desktop.dispatch(this, source === 'interactive' ? 'InteractiveChange' : 'ProgrammaticChange');
    }
  }

  /**
   * What the Anchor property means: the control keeps its distance from the edges it is anchored
   * to, so it moves when the far edge moves and stretches when both edges are named. The bits are
   * 1 top, 2 left, 4 bottom, 8 right, 16 centred vertically, 32 centred horizontally.
   */
  private moveAnchored(dx: number, dy: number): void {
    if (dx === 0 && dy === 0) return;
    for (const child of this.children) {
      const anchor = Number(child.get('Anchor') ?? 0);
      if (!anchor) continue;
      const left = Number(child.get('Left') ?? 0);
      const top = Number(child.get('Top') ?? 0);
      const width = Number(child.get('Width') ?? 0);
      const height = Number(child.get('Height') ?? 0);
      const [TOP, LEFT, BOTTOM, RIGHT, MIDDLE, CENTRE] = [1, 2, 4, 8, 16, 32];

      if (anchor & CENTRE) child.set('Left', Math.round((Number(this.get('Width') ?? 0) - width) / 2));
      else if (anchor & RIGHT) {
        // stretched when it is held to both sides, moved when only to the far one
        if (anchor & LEFT) child.set('Width', Math.max(0, width + dx));
        else child.set('Left', left + dx);
      }

      if (anchor & MIDDLE) child.set('Top', Math.round((Number(this.get('Height') ?? 0) - height) / 2));
      else if (anchor & BOTTOM) {
        if (anchor & TOP) child.set('Height', Math.max(0, height + dy));
        else child.set('Top', top + dy);
      }
    }
  }

  /** True when this control keeps an item list: AddItem, List(n), Selected(n) and friends. */
  /** An ActiveX control: everything it does is COM, which this runtime does not speak. */
  get isOleControl(): boolean {
    const base = this.baseClass.toLowerCase();
    return base === 'olecontrol' || base === 'oleboundcontrol';
  }

  /**
   * The control behind an ActiveX control, for the few this runtime provides itself. Its members
   * answer after the object's own, so `Visible` stays the form's and `Nodes` is the TreeView's.
   */
  ole: HostObject | null = null;

  /** Something the object shows has changed outside a property write; redraw it. */
  notifyChanged(): void {
    this.notify();
  }

  get isListControl(): boolean {
    return LIST_CONTROLS.has(this.type);
  }

  /**
   * `DIMENSION aRGB[3]` in a class body: an array property, every element starting at .F.
   *
   * `cols` above zero makes it two-dimensional, which is how a form declares `^aIcon[5,2]`. VFP
   * lays a table out row by row and lets either `a[r,c]` or a single running subscript reach an
   * element, so one flat list holds it and the column count says how to read a pair.
   */
  declareArray(name: string, rows: number, cols = 0, fill: VmValue = false): void {
    const length = Math.max(0, rows) * Math.max(1, cols);
    this.propNames.set(name.toLowerCase(), name);
    this.arrays.set(name.toLowerCase(), { values: new Array<VmValue>(length).fill(fill), cols: Math.max(0, cols) });
    this.notify();
  }

  /**
   * `DIMENSION oObject.aProp[2, 3]`: the property is given an array of that shape.
   *
   * Measured in Visual FoxPro 9: what the property already holds is regrown rather than
   * replaced, so an element that still fits stays where it is - `oObj.a[1,1]` and `oObj.a[2,2]`
   * both survive a two-by-two growing into a four-by-two - and a property the object has never
   * heard of raises rather than being created. A property that held something other than an
   * array is refused with 232 rather than becoming one: only a property that was declared with
   * subscripts - a form's `^aReports[1,0]` line, `ADDPROPERTY("aRows[1]")` - can be sized.
   */
  dimension(name: string, rows: number, cols: number): void {
    if (!this.has(name)) throw new HostError(1734, `Property ${name.toUpperCase()} is not found.`);
    const held = this.arrays.get(name.toLowerCase())?.values;
    if (!held) throw new HostError(232, `'${name.toUpperCase()}' is not an array.`);
    const length = Math.max(0, rows) * Math.max(1, cols);
    const values = new Array<VmValue>(length).fill(false);
    for (let i = 0; i < Math.min(length, held.length); i++) values[i] = held[i] ?? false;
    this.propNames.set(name.toLowerCase(), this.canonical(name) ?? name);
    this.arrays.set(name.toLowerCase(), { values, cols: Math.max(0, cols) });
    this.notify();
  }

  /**
   * The children `Pages(n)`, `Columns(n)`, `Controls(n)` and their kind reach, or nothing when
   * this object has no such member. A page frame answers `Pages` and a grid `Columns`; every
   * container answers `Controls` and `Objects` with everything it holds.
   */
  memberArray(name: string): RuntimeObject[] | undefined {
    const member = MEMBER_ARRAYS[name.toUpperCase()];
    if (!member) return undefined;
    if (member.owners && !member.owners.has(this.type)) return undefined;
    if (!member.of) return [...this.children];
    return this.children.filter((c) => member.of!.has(c.baseClass.toLowerCase()));
  }

  /**
   * An array property as a whole value. A list control answers its indexed properties from the
   * item list, so `List`, `Selected` and the rest read as arrays without being stored as one.
   */
  getArray(name: string): VmArray | undefined {
    const own = this.arrays.get(name.toLowerCase());
    if (own) return { $arr: [...own.values], $cols: own.cols };
    const items = this.listArray(name.toUpperCase());
    return items ? { $arr: items, $cols: 0 } : undefined;
  }

  /**
   * Where a subscript lands in an array property, or `undefined` when it lands outside it.
   *
   * One subscript runs through the whole array however many dimensions it has; two address a row
   * and a column, which only a two-dimensional array has.
   */
  private elementAt(own: { values: VmValue[]; cols: number }, index: readonly number[]): number | undefined {
    const at =
      index.length === 1
        ? (index[0] ?? 0)
        : index.length === 2 && own.cols > 0
          ? ((index[0] ?? 0) - 1) * own.cols + (index[1] ?? 0)
          : 0;
    const row = index.length === 2 ? (index[0] ?? 0) : 1;
    const col = index.length === 2 ? (index[1] ?? 0) : 1;
    if (at < 1 || at > own.values.length || row < 1 || col < 1 || (index.length === 2 && col > own.cols)) {
      return undefined;
    }
    return at - 1;
  }

  private listArray(upper: string): VmValue[] | undefined {
    if (!this.isListControl || !(LIST_ARRAYS.has(upper) || LIST_ID_ARRAYS.has(upper))) return undefined;
    switch (upper) {
      case 'INDEXTOITEMID':
        return this.items.map((i) => i.id);
      // laid out by id rather than by position, so `ItemIdToIndex[k]` lands on the row that
      // holds id k; a number no item is using answers 0, as the call form does
      case 'ITEMIDTOINDEX': {
        const highest = this.items.reduce((most, i) => Math.max(most, i.id), 0);
        return Array.from({ length: highest }, (_, k) => this.indexOfItemId(k + 1));
      }
      case 'SELECTED':
        return this.items.map((i) => i.selected);
      case 'PICTURE':
        return this.items.map((i) => i.picture);
      case 'ITEMDATA':
        return this.items.map((i) => i.data);
      default:
        return this.items.map((i) => i.text);
    }
  }

  /** `obj.Prop[2] = v`. VFP refuses a subscript past the end rather than growing the array. */
  setElement(name: string, index: readonly number[], value: VmValue): void {
    const at = index[0] ?? 0;
    const own = this.arrays.get(name.toLowerCase());
    if (own) {
      const cell = this.elementAt(own, index);
      if (cell === undefined) throw new HostError(31, 'Subscript is outside defined range');
      own.values[cell] = value;
      this.notify();
      return;
    }
    const upper = name.toUpperCase();
    if (this.isListControl && LIST_ARRAYS.has(upper)) {
      const item = this.items[at - 1];
      if (!item) throw new HostError(31, 'Subscript is outside defined range');
      if (upper === 'SELECTED') item.selected = value === true;
      else if (upper === 'PICTURE') item.picture = String(vmToProp(value) ?? '');
      else if (upper === 'ITEMDATA') item.data = value;
      else item.text = String(vmToProp(value) ?? '');
      this.notify();
      return;
    }
    if (!this.has(name)) throw new HostError(1734, `Property ${upper} is not found.`);
    throw new HostError(31, 'Subscript is outside defined range');
  }

  /**
   * AddItem: appends, or inserts at a one-based position.
   *
   * `AddItem(cItem, nIndex, nColumn)` may name a column instead of adding a row: with a column
   * past the first, the text belongs to the row already at that position. `AddListItem` says
   * the same thing by item id rather than by position, and `byId` is which of the two this is.
   */
  addItem(text: string, at?: number, column = 1, byId = false): void {
    if (column > 1) {
      const found = at === undefined ? undefined : byId ? this.items.find((i) => i.id === at) : this.items[at - 1];
      // a column of a row that is not there yet is kept until the row arrives, which is what
      // happens when a program fills column two of a row it is about to add
      const row = found ?? this.items[this.items.length - 1];
      if (!row) return;
      row.columns[column - 1] = text;
      this.notify();
      return;
    }
    const item: ListItem = { columns: [text], text, data: null, picture: '', selected: false, id: byId && at !== undefined ? at : this.freeItemId() };
    if (!byId && at !== undefined && at >= 1 && at <= this.items.length) this.items.splice(at - 1, 0, item);
    else this.items.push(item);
    this.notify();
  }

  /**
   * The id the next item takes: the lowest positive number no item is using.
   *
   * Measured in Visual FoxPro 9 - three items added, two removed, three added again - the ids
   * come back 3, 1, 2, 4: an id freed by a removal is handed out again before a new one is
   * invented, so a program that keeps ids of its own has to look them up rather than assume.
   */
  private freeItemId(): number {
    const taken = new Set(this.items.map((i) => i.id));
    let id = 1;
    while (taken.has(id)) id++;
    return id;
  }

  /** One row's column, as `List(nRow, nColumn)` reads it; column one is the row's own text. */
  column(item: ListItem, which: number): string {
    return item.columns[Math.max(1, which) - 1] ?? (which <= 1 ? item.text : '');
  }

  /** Marks one item current, the way clicking a row does. */
  selectItem(text: string): void {
    for (const item of this.items) item.selected = item.text === text;
    this.notify();
  }

  removeItem(at: number): void {
    if (at >= 1 && at <= this.items.length) this.items.splice(at - 1, 1);
    this.notify();
  }

  clearItems(): void {
    this.items.length = 0;
    this.notify();
  }

  /** Adds a user-defined property (ADDPROPERTY). */
  addProperty(name: string, value: PropValue): void {
    this.propNames.set(name.toLowerCase(), name);
    this.values.set(name.toLowerCase(), value);
    this.addedProps.add(name.toLowerCase());
    this.notify();
  }

  /** Every property the object answers to, in the spelling it was declared with. */
  propertyNames(): string[] {
    return [...this.propNames.values()];
  }

  /** The events and the methods, as the class declares them. */
  eventNameList(): string[] {
    return [...this.eventNames.values()].map((e) => e.name);
  }

  methodNameList(): string[] {
    return [...this.methodNames];
  }

  /** ADDPROPERTY put it there, rather than a class declaring it: AMEMBERS()' "B". */
  wasAdded(name: string): boolean {
    return this.addedProps.has(name.toLowerCase());
  }

  /** It holds something other than what its class starts it at: AMEMBERS()' "C". */
  wasWritten(name: string): boolean {
    return this.values.has(name.toLowerCase());
  }

  /** REMOVEPROPERTY: only a property the descriptor does not declare can be removed. */
  removeProperty(name: string): boolean {
    const key = name.toLowerCase();
    const declared = getObjectDescriptor(this.node).properties.some((p) => p.name.toLowerCase() === key);
    if (declared || !this.propNames.has(key)) return false;
    this.propNames.delete(key);
    this.values.delete(key);
    this.notify();
    return true;
  }

  /** Every property with its current value: what the React renderers draw from. */
  resolved(): Record<string, PropValue> {
    const out = resolveProps(this.node);
    for (const [key, value] of this.values) {
      out[this.propNames.get(key) ?? key] = value;
    }
    return out;
  }

  hasEvent(name: string): boolean {
    return this.eventNames.has(name.toLowerCase());
  }

  /** The error the product raises when a program writes that property, if it refuses at all. */
  refusesWrite(name: string): number | undefined {
    return this.readOnlyNames.get(name.toLowerCase());
  }

  /**
   * True when the class itself has a method of that name.
   *
   * Which methods a class has is the product's answer, measured out of it: a Label has no
   * AddObject and a Timer has no SetFocus, and `PEMSTATUS(o, "AddObject", 5)` must say so. A
   * class the measurement does not cover - a report listener, a COM control - falls back to the
   * list of every method the language names, which is what everything used to get.
   */
  hasMethodName(name: string): boolean {
    if (this.methodNames.size === 0) return METHOD_NAMES.has(name.toUpperCase());
    return this.methodNames.has(name.toLowerCase());
  }

  /**
   * True when the object carries FoxPro source for a method of that name. A form or class may add
   * methods of its own - `CenterForm`, `GetDirectory` - and those are called like any other.
   */
  hasOwnMethod(name: string): boolean {
    this.ownMethods ??= new Set(Object.keys(this.node.methods).map((m) => m.toLowerCase()));
    return this.ownMethods.has(name.toLowerCase());
  }

  /** The node's method names, lower-cased: asked on every property read, for Access methods. */
  private ownMethods: Set<string> | undefined;

  /** True when the object's class defines this method in FoxPro source. */
  hasClassMethod(name: string): boolean {
    return this.classMethods.has(name.toLowerCase());
  }

  // ---- containment ----

  addChild(child: RuntimeObject): void {
    child.parent = this;
    this.children.push(child);
    this.childrenByName.set(child.name.toLowerCase(), child);
    // AddObject can add one while the form is on screen, and the container draws its children
    this.notify();
  }

  /** ZOrder: a child to the front of its container's drawing order, or to the back of it. */
  moveChild(child: RuntimeObject, where: 'front' | 'back'): void {
    const at = this.children.indexOf(child);
    if (at < 0) return;
    this.children.splice(at, 1);
    if (where === 'front') this.children.push(child);
    else this.children.unshift(child);
    this.notify();
  }

  /** The FoxPro source of a method the object carries, for ReadMethod. */
  methodSource(name: string): string {
    const key = Object.keys(this.node.methods).find((m) => m.toLowerCase() === name.toLowerCase());
    return key ? (this.node.methods[key] ?? '') : '';
  }

  /** WriteMethod: the source a method will run from here on. It is compiled when it is called. */
  setMethodSource(name: string, source: string): void {
    const key = Object.keys(this.node.methods).find((m) => m.toLowerCase() === name.toLowerCase()) ?? name;
    this.node.methods[key] = source;
    this.ownMethods?.add(key.toLowerCase());
  }

  /** MoveItem: an item of a list control, moved to another place in the list. */
  /**
   * MoveItem: an item moved to another place in the list.
   *
   * `source` says what moved it, as the OnMoveItem event is told: 0 the keyboard, 1 the left
   * mouse button, 8 a program calling the method.
   */
  moveItem(from: number, to: number, source = 8): void {
    const item = this.items[from - 1];
    if (!item || to < 1 || to > this.items.length) return;
    this.items.splice(from - 1, 1);
    this.items.splice(to - 1, 0, item);
    this.notify();
    void this.desktop.dispatch(this, 'OnMoveItem', [source, 0, to, to - from]);
  }

  /**
   * IndexToItemID: the id of the item at that position, or 0 when there is no item there.
   *
   * An item keeps the id it was added with wherever the item moves to, which is why a program
   * that means one particular row holds its id rather than its position.
   */
  itemIdAt(index: number): number {
    return index >= 1 && index <= this.items.length ? (this.items[index - 1]?.id ?? 0) : 0;
  }

  /** ItemIDToIndex: where the item with that id sits now. */
  indexOfItemId(id: number): number {
    return this.items.findIndex((i) => i.id === id) + 1;
  }

  /** RemoveObject: drops a child added at runtime. */
  removeChild(child: RuntimeObject): void {
    const at = this.children.indexOf(child);
    if (at >= 0) this.children.splice(at, 1);
    this.childrenByName.delete(child.name.toLowerCase());
    child.parent = null;
    this.notify();
  }

  child(name: string): RuntimeObject | undefined {
    return this.childrenByName.get(name.toLowerCase());
  }

  /** The form this object belongs to (THISFORM); `FormInstance` overrides it to return itself. */
  form(): FormInstance | null {
    return this.parent?.form() ?? null;
  }

  /**
   * The formset this object belongs to (THISFORMSET), or nothing when it is in none. A formset
   * stands above the forms, so this walks past the form rather than stopping at it.
   */
  formSet(): FormSetInstance | null {
    return this.parent?.formSet() ?? null;
  }

  /**
   * Whose compiled module holds this object's method bodies: its form, the formset itself for
   * the formset's own methods, or the object itself when it came from a document of its own -
   * a class out of a library. A formset is compiled apart from its forms because each of them
   * is a document of its own, and a library class for the same reason.
   */
  codeOwner(): RuntimeObject | null {
    if (this.module >= 0) return this;
    return this.parent?.codeOwner() ?? null;
  }

  /**
   * Path from whatever owns this object's code, e.g. `pgfMain.Page1.lblGreeting`; empty for the
   * owner itself. The methods of a class library's class are compiled from the class's own tree,
   * so an object built from one is where its members' paths start, wherever it has been put.
   */
  path(): string {
    if (this.module >= 0) return '';
    const parent = this.parent?.path() ?? '';
    return parent ? `${parent}.${this.name}` : this.name;
  }

  /** Depth-first list of this object and everything under it. */
  descendants(): RuntimeObject[] {
    const out: RuntimeObject[] = [];
    const walk = (o: RuntimeObject) => {
      for (const c of o.children) {
        out.push(c);
        walk(c);
      }
    };
    walk(this);
    return out;
  }

  // ---- DOM binding ----

  /** React renderers register their focusable element so SetFocus() can reach it. */
  bindElement = (el: FocusableElement | null): void => {
    this.element = el;
    if (el && this.focusPending) {
      this.focusPending = false;
      el.focus();
    }
  };

  /**
   * Init runs before the control is on screen, so a SetFocus() from there has nothing to focus
   * yet. VFP applies it when the form appears; here the request waits for the element to bind.
   */
  setFocus(): void {
    if (this.element) this.element.focus();
    else this.focusPending = true;
  }

  refresh(): void {
    this.notify();
  }
}

/** One thing a form was told to draw on itself, in the order it was told. */
export interface Drawing {
  shape: 'box' | 'line' | 'circle' | 'point' | 'text';
  /** Pixels, as the form's own coordinates. */
  x1: number;
  y1: number;
  x2: number;
  y2: number;
  /** What Print wrote, for a drawing that is text. */
  text?: string;
  /** A circle's radius and how much it is squashed; nothing for the rest. */
  radius?: number;
  aspect?: number;
  colour: number;
  width: number;
  /** DrawStyle, as the dash pattern it stands for. */
  style: number;
}

/**
 * What `DO FORM` opens and can be waited on: a form, or the formset a formset file makes.
 *
 * Both are asked for by name, both hand a value back when they close, and both may hold the
 * program that opened them until they do, so the waiting belongs to neither of them alone.
 */
export abstract class OpenedWindow extends RuntimeObject {
  modal = false;
  /** Value returned by Unload, for `DO FORM ... TO var`. */
  result: VmValue = null;
  /** Resolves when it is released; a modal DO FORM awaits it. */
  private closed: ((result: VmValue) => void)[] = [];

  whenClosed(): Promise<VmValue> {
    return new Promise((resolve) => this.closed.push(resolve));
  }

  settleClosed(): void {
    const waiting = this.closed;
    this.closed = [];
    for (const resolve of waiting) resolve(this.result);
  }
}

export class FormInstance extends OpenedWindow {
  /**
   * The tables the form opens when it loads, as objects: `THISFORM.DataEnvironment.Cursor1` is
   * how form code reads what it was given. Nothing here opens them; the session does that by
   * running the USE the data environment stands for.
   */
  dataEnvironment: DataEnvironment | null = null;
  /** A class with no visual base (Custom, Session) exists but is never shown. */
  nonVisual = false;
  /**
   * What Box, Line, Circle and PSet have drawn on the form, in order. A form draws over its
   * controls' background and under the controls themselves, which is where the window puts it.
   */
  readonly drawings: Drawing[] = [];

  /** Adds one, and asks the window to paint again. */
  draw(drawing: Drawing): void {
    this.drawings.push(drawing);
    this.notifyChanged();
  }

  /** `Cls`: the form forgets everything it drew. */
  clearDrawings(): void {
    this.drawings.length = 0;
    this.notifyChanged();
  }

  override form(): FormInstance {
    return this;
  }

  override codeOwner(): FormInstance {
    return this;
  }

  /** The form is the root of a path, so its own path is empty. */
  override path(): string {
    return '';
  }
}

/**
 * A formset: the container Visual FoxPro puts round several forms so they are built, shown and
 * released together, and what `THISFORMSET` answers from anywhere inside any of them.
 *
 * Its members are its children, so `oSet.frmLeft` and `oSet.frmLeft.Parent` both work without
 * anything special, and each member form stays a form in its own right - it has its own document,
 * its own compiled module and its own place in `_SCREEN.Forms`. What the formset adds is the
 * three things a form on its own has not: it holds them, it shows and hides them at once, and
 * releasing it releases all of them.
 */
export class FormSetInstance extends OpenedWindow {
  /** The member whose Activate ran last: what `ActiveForm` answers. */
  activeForm: FormInstance | null = null;

  /** `Formset`, which is how THISFORMSET recognises it walking up from a control. */
  override get baseClass(): string {
    return 'Formset';
  }

  override formSet(): FormSetInstance {
    return this;
  }

  override codeOwner(): FormSetInstance {
    return this;
  }

  /** Its methods are compiled at the root of its own module, as a form's are. */
  override path(): string {
    return '';
  }

  /** The forms it holds, in the order they were added. */
  get forms(): FormInstance[] {
    return this.children.filter((c): c is FormInstance => c instanceof FormInstance);
  }
}

export interface EventBinding {
  /** Object whose method runs. */
  handler: number;
  /** Method name on the handler. */
  delegate: string;
  flags: number;
}

export interface CreateFormOptions {
  modal?: boolean;
  noshow?: boolean;
  /** Arguments passed to the form's Init (`DO FORM x WITH 1, 2`). */
  args?: VmValue[];
  /** The `DEFINE CLASS` name this object was created from, if any. */
  className?: string;
  /** A class with no visual base: it is registered and runs its methods, but never shown. */
  nonVisual?: boolean;
  /** The tables the form's data environment names, which become its Cursor objects. */
  cursors?: FormCursor[];
  /**
   * Whether the form has a DataEnvironment object. Only a form read from a form file does: a
   * form made from a class - `CREATEOBJECT("Form")` or a `.vcx` form class - has none, and
   * `PEMSTATUS(o, "DataEnvironment", 5)` answers .F. for it (measured). Defaults to whether
   * cursors were given.
   */
  dataEnvironment?: boolean;
  /** Property values written as expressions, worked out as the form is built. */
  expressions?: Record<string, string>;
  /** Array properties the objects add for themselves, as `[rows, cols]`. */
  arrays?: Record<string, [number, number]>;
  /**
   * Reads an expression where the program that opened the form stands. Visual FoxPro works a
   * property expression out in that scope, so a form opened by a program holding cFile can say
   * Picture = (cFile); a program of our own would not see it.
   */
  inFrame?: (expression: string) => VmValue;
}

/**
 * `_SCREEN`: owns every running form, hands out handles and answers the VM's synchronous
 * reads. One per session.
 */
export class Desktop implements HostReads {
  readonly forms: FormInstance[] = [];
  /**
   * What a program has written to `_SCREEN` or given it with AddProperty, by upper-cased name.
   * A CodeMine application's first lines set the caption and add a property to carry its
   * start-up parameter across a CLEAR ALL, so the screen has to hold what it is told.
   */
  private readonly screenValues = new Map<string, VmValue>();
  /** The formsets running: containers of forms, which are not themselves windows on the screen. */
  readonly formSets: FormSetInstance[] = [];
  /** Handle -> object; index 0 is the desktop itself. */
  private readonly handles: (RuntimeObject | undefined)[] = [undefined];
  private nextHandle = 1;

  /**
   * What the application object says about itself, for `_VFP` and `Application`.
   *
   * The ones below are about this copy of the product and are worked out. Everything else the
   * application answers to falls through to what `_VFP` was measured holding, so a property this
   * switch has never heard of still answers rather than being missing.
   */
  /** What a program wrote on `_VFP`: `_VFP.Height = 500` reads back as 500. */
  private appValues = new Map<string, VmValue>();

  private appProp(name: string): VmValue | undefined {
    const measured = BASE_CLASS_MEMBERS['Application'];
    const written = this.appValues.get(name.toUpperCase());
    if (written !== undefined) return written;
    switch (name.toUpperCase()) {
      // `_VFP` in the product has no Class and no BaseClass at all - asking raises - but a great
      // deal of code asks any object what it is, and answering is kinder than raising
      case 'CLASS':
      case 'BASECLASS':
        return 'Application';
      case 'PARENT':
        throw new HostError(1924, 'PARENT is not an object.');
      case 'APPLICATION':
        return { $obj: APP_HANDLE };
      case 'SCREEN':
        return { $obj: SCREEN_HANDLE };
      case 'ACTIVEFORM':
        return this.forms.length ? { $obj: this.forms[this.forms.length - 1]!.handle } : null;
      // the project a builder reads and acts on, and the list it is the only member of
      case 'ACTIVEPROJECT': {
        const project = this.activeProject?.() ?? null;
        return project ? { $obj: this.hostHandle(project) } : null;
      }
      case 'PROJECTS': {
        const project = this.activeProject?.() ?? null;
        return { $obj: this.hostHandle(new Collection(project ? [project] : [])) };
      }
      case 'PROJECTCOUNT':
        return this.activeProject?.() ? 1 : 0;
      case 'FORMCOUNT':
        return this.visibleForms.length;
      // the window's title, which is this product's and not Visual FoxPro's. Name is left as the
      // measurement has it, because a program asking what it is running in is asking about the
      // language it was written for.
      case 'CAPTION':
        return 'FoxDev Studio';
      case 'VISIBLE':
        return true;
      // the application is the one running the program, not one started by another program
      case 'STARTMODE':
        return 0;
      case 'STATUSBAR':
        return this.statusText;
      case 'DEFAULTFILEPATH':
        return '';
      // where the product is installed. The measurement knows where Visual FoxPro was, which is
      // not where this is, and nothing here knows either, so a program is told nothing.
      case 'FULLNAME':
      case 'SERVERNAME':
        return '';
      // one process, one thread: the numbers are of this copy and not of the class
      case 'PROCESSID':
      case 'THREADID':
      case 'HWND':
        return 1;
      default: {
        const held = measured?.properties[Object.keys(measured.properties).find((p) => p.toUpperCase() === name.toUpperCase()) ?? ''];
        return held ?? undefined;
      }
    }
  }
  private readonly outputLines: string[] = [];
  private readonly listeners = new Set<() => void>();
  version = 0;

  /** Injected by the session so the object model can run FoxPro event handlers. */
  dispatch: Dispatch = () => null;
  /** Injected clock, overridable in tests. */
  clock: () => Date = () => new Date();
  /** Injected RNG, overridable in tests. */
  rng: () => number = Math.random;
  /** What OS() reports, as `name|major|minor|build`; overridable in tests. */
  osInfo: () => string = detectOsInfo;
  /**
   * Where the pointer is over the character screen, for MROW(), MCOL() and MDOWN().
   * Injected by the session, which is what the screen is drawn by.
   */
  mouse: () => { row: number; col: number; down: boolean } = () => ({ row: 0, col: 0, down: false });
  /** Injected by the session: what the application object's Quit method asks for. */
  quitRequested: (() => void) | null = null;
  /**
   * Where a `SET LIBRARY TO` goes: the process that hosts Visual FoxPro libraries, when there
   * is one. Injected by the session because only it knows how to reach the host - across an
   * IPC channel in the application, and the child process itself under test.
   */
  libraries: LibraryHost | null = null;

  /** Resolves `DO <program>` to an already loaded module id. */
  programs = new Map<string, number>();
  /** BINDEVENT: `sourceHandle:event` -> handlers to run after the object's own method. */
  private readonly bindings = new Map<string, EventBinding[]>();

  subscribe = (listener: () => void): (() => void) => {
    this.listeners.add(listener);
    return () => void this.listeners.delete(listener);
  };

  getSnapshot = (): number => this.version;

  private notify(): void {
    this.version++;
    for (const l of [...this.listeners]) l();
  }

  /** `?` output collected so far. Named `lines` because `output()` is the HostReads method. */
  get lines(): readonly string[] {
    return this.outputLines;
  }

  clearOutput(): void {
    this.outputLines.length = 0;
    this.notify();
  }

  object(handle: number): RuntimeObject | undefined {
    return this.handles[handle];
  }

  // ---- objects that are not controls: the emulated ActiveX controls and their parts ----

  /** Handle -> host object, for everything a FoxPro program holds that is not a control. */
  private readonly hosted = new Map<number, HostObject>();
  private readonly hostedHandles = new Map<HostObject, number>();

  /** The handle a host object answers to, assigning one the first time it is seen. */
  hostHandle(object: HostObject): number {
    const known = this.hostedHandles.get(object);
    if (known !== undefined) return known;
    const handle = this.nextHandle++;
    this.hosted.set(handle, object);
    this.hostedHandles.set(object, handle);
    return handle;
  }

  /** The host object a value refers to, for a member whose value is one. */
  hostObject(value: VmValue): HostObject | undefined {
    if (value === null || typeof value !== 'object' || !('$obj' in value)) return undefined;
    const handle = value.$obj;
    return this.hosted.get(handle) ?? this.handles[handle]?.ole ?? undefined;
  }

  /** Wraps a host object's answer for the VM: another object becomes a handle. */
  private fromHost(value: VmValue | HostObject | Promise<VmValue> | undefined): VmValue | Promise<VmValue> | undefined {
    if (value === null || value === undefined || typeof value !== 'object') return value;
    // a host object may answer a method with work that is still going on
    if (value instanceof Promise) return value;
    if ('$obj' in value || '$arr' in value || '$date' in value || '$dt' in value) return value;
    return { $obj: this.hostHandle(value as HostObject) };
  }

  /**
   * Gives an ActiveX control the object that stands in for it, when this runtime has one.
   *
   * The Common Controls are provided rather than emulated through COM: an OCX draws into a
   * window of its own, which nothing inside a browser engine can host, so a real one would be
   * invisible even where COM is available.
   */
  private attachOle(object: RuntimeObject): void {
    if (!object.isOleControl) return;
    const declared = String(object.get('OleClass') ?? '');
    const kind = oleEmulation(declared);
    if (kind) {
      object.ole = createOleControl(kind, {
        hosted: (value) => this.hostObject(value),
        changed: () => object.notifyChanged(),
      });
      return;
    }
    // a control this runtime does not draw can still be talked to, where COM is available. It
    // has no window here - an OCX draws into one of its own - but its properties and methods
    // answer, which is enough for the code around it to run.
    object.ole = this.createOleObject?.(declared) ?? null;
  }

  /**
   * Injected by the session: makes a COM object for a ProgID. Absent where there is no COM,
   * which is every platform but Windows and any build without the bridge.
   */
  createOleObject: ((progId: string) => HostObject | null) | null = null;

  /**
   * Injected by the session: the project open in the development environment, which
   * `_VFP.ActiveProject` hands over. Absent in the player, which has no project open.
   */
  activeProject: (() => HostObject | null) | null = null;

  /** Injected by the session: works out an expression, for the application's Eval method. */
  evaluate: ((expression: string) => Promise<VmValue>) | null = null;
  /**
   * The same, for a value the form is only trying to work out: a property expression that will
   * not run is not a fault in the program that opened the form, so it is not reported as one.
   */
  evaluateQuietly: ((expression: string) => Promise<VmValue>) | null = null;

  /** Injected by the session: runs a line, for what a method has to say in FoxPro. */
  runLine: ((text: string) => Promise<void>) | null = null;

  /**
   * Injected by the session: an object of a class that lives in a class library.
   *
   * Reading a `.vcx` and compiling what is in it belongs to the session, which is the only
   * thing here that can open a file; `into` is the container the new object is to be a member
   * of, under the name given. `undefined` when no library in reach has a class of that name,
   * and `null` when it has one whose Init refused to be created - which are different answers,
   * because only the first of them means the program named something that does not exist.
   */
  /**
   * An object of the class of that name, made as `CREATEOBJECT()` would make it: a base class,
   * a `DEFINE CLASS` of a program already loaded, or a class of a loaded class library. The
   * session provides it; `_SCREEN.AddObject` is what asks.
   */
  createNamedObject: ((className: string) => Promise<VmValue>) | null = null;
  libraryObject:
    | ((
        className: string,
        module: string,
        into?: { parent: RuntimeObject; name: string },
      ) => Promise<RuntimeObject | null | undefined>)
    | null = null;

  /** Injected by the session: sets a variable, for the application's SetVar method. */
  setVariable: ((name: string, value: VmValue) => void) | null = null;

  /** What the status bar says: the application's StatusBar property, and what Message answers. */
  statusText = '';

  /** Sets it, and lets whatever draws the status bar know. */
  setStatusText(text: string): void {
    this.statusText = text;
    this.notify();
  }

  /** The control a manual drag is carrying, from Drag(1) until Drag(0) or Drag(2). */
  dragging: RuntimeObject | null = null;

  /** What the pointer is over, so a drop knows what it landed on. */
  pointerOver: RuntimeObject | null = null;

  /** Builds the live object tree for a form document, without running any code yet. */
  instantiate(form: FormNode, module: number, options: CreateFormOptions = {}): FormInstance {
    const instance = new FormInstance(this.nextHandle++, form, this);
    instance.module = module;
    instance.className = options.className ?? null;
    instance.nonVisual = options.nonVisual ?? false;
    instance.modal = options.modal ?? instance.get('WindowType') === 1;
    if (options.dataEnvironment ?? options.cursors !== undefined) instance.dataEnvironment = new DataEnvironment(options.cursors);
    this.handles[instance.handle] = instance;

    this.buildChildren(form.children, instance);
    this.dimensionArrays(instance, options.arrays);

    this.forms.push(instance);
    this.notify();
    return instance;
  }

  /**
   * Builds the formset a file put round its forms. Nothing is in it yet and no code has run:
   * the forms are added as they are built, and the formset's own Init runs after all of them.
   */
  instantiateFormSet(node: FormNode, module: number): FormSetInstance {
    const set = new FormSetInstance(this.nextHandle++, node, this);
    set.module = module;
    this.handles[set.handle] = set;
    // a formset is visible from the moment it exists, whatever its forms are doing
    set.set('Visible', true);
    this.formSets.push(set);
    this.notify();
    return set;
  }

  /** Puts a form in a formset, which is what makes it a member rather than a window on its own. */
  addToFormSet(set: FormSetInstance, form: FormInstance): void {
    set.addChild(form);
  }

  /**
   * `Show` on a formset: every form it holds appears at once.
   *
   * Not quite every form - one the file said was invisible stays invisible, and shows only when
   * something asks that form itself. Measured in Visual FoxPro 9: a member declared
   * `Visible = .F.` is still `.F.` after the formset's Show, while one that said nothing is `.T.`.
   */
  showFormSet(set: FormSetInstance): void {
    for (const form of set.forms) {
      if (form.node.props['Visible'] === false) continue;
      form.set('Visible', true);
    }
    this.notify();
  }

  /** `Hide` on a formset: its forms go, and the formset itself carries on existing. */
  hideFormSet(set: FormSetInstance): void {
    for (const form of set.forms) form.set('Visible', false);
    this.notify();
  }

  /**
   * `Release` on a formset, and what a member form releasing the last of them comes to.
   *
   * Visual FoxPro runs the formset's own Destroy first and then takes the forms down in the
   * reverse of the order it built them, which is the order any container releases its members.
   */
  async releaseFormSet(set: FormSetInstance): Promise<void> {
    if (!set.alive) return;
    set.alive = false;
    await this.fire(set, 'Destroy');
    for (const form of [...set.forms].reverse()) await this.releaseForm(form, { queryUnload: false });
    this.handles[set.handle] = undefined;
    this.unbindEvent(set.handle);
    this.unbindEvent(undefined, undefined, set.handle);
    const at = this.formSets.indexOf(set);
    if (at >= 0) this.formSets.splice(at, 1);
    set.settleClosed();
    this.notify();
  }

  /** What a formset answers that a form does not. */
  private formSetProp(set: FormSetInstance, name: string): VmValue | undefined {
    switch (name.toUpperCase()) {
      case 'FORMCOUNT':
        return set.forms.length;
      case 'ACTIVEFORM': {
        const active = set.activeForm ?? set.forms[set.forms.length - 1];
        return active?.alive ? { $obj: active.handle } : null;
      }
      default:
        return undefined;
    }
  }

  /**
   * Creates the array properties the objects declared, before anything runs.
   *
   * An array property exists from the moment the object does - a form's Load can already read
   * `ALEN(THISFORM.aItems)` - so this happens as the tree is built rather than at Load.
   */
  private dimensionArrays(instance: RuntimeObject, arrays: Record<string, [number, number]> | undefined): void {
    for (const [where, [rows, cols]] of Object.entries(arrays ?? {})) {
      const cut = where.lastIndexOf('.');
      if (cut <= 0) continue;
      this.memberAt(instance, where.slice(0, cut))?.declareArray(where.slice(cut + 1), rows, cols);
    }
  }

  /**
   * VFP creation order: Load on the form, then Init on the controls (deepest first, so a
   * container's Init sees initialised children), then the form's Init, then Show/Activate.
   *
   * An Init returning .F. means "do not create this". For a control that is the control alone -
   * the form opens without it, and `ControlCount` is one lower - and only the form's own Init
   * refusing cancels the form. Measured in Visual FoxPro 9: a form whose member Init returns
   * .F. comes back as an object with one control instead of two, while a form whose own Init
   * returns .F. comes back as .F. itself. Taking the whole form down for one control is how a
   * form built on the Foundation Classes came up blank, because one of them declines when the
   * table it wants is not open.
   */
  async runFormLifecycle(instance: FormInstance, options: CreateFormOptions = {}): Promise<boolean> {
    await this.workOutProperties(instance, options.expressions, { inFrame: options.inFrame });
    await this.fire(instance, 'Load');

    const controls = instance.descendants().reverse();
    for (const control of controls) {
      // a control dropped by its own container's Init is gone before its turn comes
      if (!control.alive || !control.parent) continue;
      const outcome = await this.fire(control, 'Init');
      if (outcome?.value === false) this.dropControl(control);
    }

    const init = await this.fire(instance, 'Init', options.args ?? []);
    if (init?.value === false) {
      await this.releaseForm(instance, { queryUnload: false });
      return false;
    }

    if (!options.noshow && !instance.nonVisual) {
      instance.set('Visible', true);
      await this.fire(instance, 'Show', [1]);
      await this.fire(instance, 'Activate');
      // the formset remembers which of its forms went last, which is what ActiveForm answers
      const set = instance.formSet();
      if (set) set.activeForm = instance;
    }
    this.notify();
    return true;
  }

  /**
   * Works out the property values a form keeps as expressions rather than as values.
   *
   * Everything in a `.scx` property line is a Visual FoxPro expression, and the ones that are
   * not simply literals are worked out as the form is built - which is how
   * `Picture = (HOME() + "graphics\\edit.bmp")` finds a bitmap wherever Visual FoxPro is
   * installed, and how `Caption = (STR(RECNO()))` shows a record number. Kept as text they are
   * neither a path nor a caption, and a class library named that way reaches the file layer as
   * its own source.
   *
   * One that cannot be worked out is left alone rather than emptied: a property with the wrong
   * text in it is easier to see than one silently blank.
   */
  async workOutProperties(
    instance: RuntimeObject,
    expressions: Record<string, string> | undefined,
    options?: { inFrame?: (expression: string) => VmValue },
  ): Promise<void> {
    if (!expressions) return;
    const inFrame = options?.inFrame;
    const work = this.evaluateQuietly ?? this.evaluate;
    if (!inFrame && !work) return;
    for (const [where, source] of Object.entries(expressions)) {
      const cut = where.lastIndexOf('.');
      if (cut <= 0) continue;
      const target = this.memberAt(instance, where.slice(0, cut));
      if (!target) continue;
      try {
        const value = inFrame ? inFrame(source) : await work!(source);
        // a date is a value like any other here - `Value = (DATE())` starts a date text box -
        // even though it is not one a document could have written down
        if (isDate(value) || isDateTime(value)) target.setMomentValue(where.slice(cut + 1), value);
        else if (value !== null && typeof value !== 'object') target.set(where.slice(cut + 1), value);
      } catch {
        // the form still opens: a value it could not work out stays as it was
      }
    }
  }

  /**
   * Takes a control off the form because its own Init said not to create it.
   *
   * Everything inside it goes too: its children were built and initialised before it, and in
   * Visual FoxPro none of them exists once their container has declined. Nothing is fired on the
   * way out - the control was never created, so it has no Destroy to run.
   */
  dropControl(control: RuntimeObject): void {
    const going = [control, ...control.descendants()];
    control.parent?.removeChild(control);
    for (const o of going) {
      o.alive = false;
      delete this.handles[o.handle];
    }
    this.notify();
  }

  /**
   * VFP destruction order: QueryUnload (NODEFAULT cancels a user-initiated close), then
   * Destroy outermost-first, then Unload, whose return value is the `DO FORM ... TO` result.
   */
  async releaseForm(instance: FormInstance, { queryUnload = true } = {}): Promise<boolean> {
    if (!instance.alive) return true;
    if (queryUnload) {
      const outcome = await this.fire(instance, 'QueryUnload');
      if (outcome?.nodefault === true || outcome?.value === false) return false;
    }

    await this.fire(instance, 'Destroy');
    for (const control of instance.descendants()) await this.fire(control, 'Destroy');

    const unload = await this.fire(instance, 'Unload');
    instance.result = unload?.value ?? null;

    for (const object of [instance, ...instance.descendants()]) {
      object.alive = false;
      this.handles[object.handle] = undefined;
      this.gone.add(object.handle);
      this.unbindEvent(object.handle);
      this.unbindEvent(undefined, undefined, object.handle);
    }
    const at = this.forms.indexOf(instance);
    if (at >= 0) this.forms.splice(at, 1);

    // a member form leaving takes its place in the formset with it, and a formset with no forms
    // left has nothing to hold: AutoRelease is what says so, and it is on unless the file says not
    const set = instance.parent instanceof FormSetInstance ? instance.parent : null;
    if (set) {
      set.removeChild(instance);
      if (set.forms.length === 0 && set.get('AutoRelease') !== false) await this.releaseFormSet(set);
    }

    instance.settleClosed();
    this.notify();
    return true;
  }

  /** Awaits a dispatch that may or may not be asynchronous. */
  private async fire(obj: RuntimeObject, event: string, args: VmValue[] = []): Promise<EventOutcome | null> {
    const outcome = this.dispatch(obj, event, args);
    return outcome instanceof Promise ? await outcome : outcome;
  }

  // ---- BINDEVENT / UNBINDEVENT / RAISEEVENT ----

  private bindingKey(source: number, event: string): string {
    return `${source}:${event.toLowerCase()}`;
  }

  /**
   * Binds an event on one object to a method on another, as VFP's BINDEVENT does, and answers
   * with how many delegates that event now has - measured: a second delegate on the same event
   * answers 2, binding the same pair twice changes nothing and answers the same number again,
   * and a different event on the same object starts again at 1.
   */
  bindEvent(source: number, event: string, handler: number, delegate: string, flags = 0): number {
    if (!this.handles[source] || !this.handles[handler]) return 0;
    const key = this.bindingKey(source, event);
    const list = this.bindings.get(key) ?? [];
    // binding the same delegate twice is a no-op in VFP, not a second call
    if (!list.some((b) => b.handler === handler && b.delegate.toLowerCase() === delegate.toLowerCase())) {
      list.push({ handler, delegate, flags });
      this.bindings.set(key, list);
    }
    return list.length;
  }

  /**
   * `AEVENTS()`: what is bound, as rows of source, event, handler and delegate. An object
   * narrows it to what is bound on that object.
   */
  boundEvents(source?: number): { source: number; event: string; handler: number; delegate: string }[] {
    const out: { source: number; event: string; handler: number; delegate: string }[] = [];
    for (const [key, list] of this.bindings) {
      const [keySource, keyEvent] = key.split(':');
      if (source !== undefined && Number(keySource) !== source) continue;
      for (const binding of list) {
        out.push({ source: Number(keySource), event: keyEvent ?? '', handler: binding.handler, delegate: binding.delegate });
      }
    }
    return out;
  }

  /** `AINSTANCE()`: the objects alive under a class name, base class or otherwise. */
  instancesOf(className: string): RuntimeObject[] {
    const want = className.trim().toUpperCase();
    return this.handles.filter(
      (o): o is RuntimeObject =>
        o !== undefined && o.alive && (o.type.toUpperCase() === want || o.baseClass.toUpperCase() === want),
    );
  }

  /**
   * Removes bindings; every argument omitted means "all of them". Answers with how many went
   * away, which is what UNBINDEVENTS returns - 0 when there was nothing to remove.
   */
  unbindEvent(source?: number, event?: string, handler?: number, delegate?: string): number {
    let removed = 0;
    for (const [key, list] of [...this.bindings]) {
      const [keySource, keyEvent] = key.split(':');
      if (source !== undefined && Number(keySource) !== source) continue;
      if (event !== undefined && keyEvent !== event.toLowerCase()) continue;
      const kept = list.filter((b) => {
        const match = (handler === undefined || b.handler === handler) && (delegate === undefined || b.delegate.toLowerCase() === delegate.toLowerCase());
        if (match) removed += 1;
        return !match;
      });
      if (kept.length > 0) this.bindings.set(key, kept);
      else this.bindings.delete(key);
    }
    return removed;
  }

  /** Handlers bound to an event, in the order they were bound. */
  boundHandlers(source: number, event: string): EventBinding[] {
    return this.bindings.get(this.bindingKey(source, event)) ?? [];
  }

  findForm(name: string): FormInstance | undefined {
    return this.forms.find((f) => f.name.toLowerCase() === name.toLowerCase());
  }

  /** Forms with something to draw; a Custom or Session object is live but has no window. */
  get visibleForms(): FormInstance[] {
    return this.forms.filter((f) => !f.nonVisual);
  }

  /**
   * `_SCREEN.Forms`, topmost first: the form made last is `Forms(1)`. Measured: a Custom object
   * is not in it, a form nobody has shown is.
   */
  private screenForms(): FormInstance[] {
    return this.visibleForms.reverse();
  }

  /** `_SCREEN.Forms(n)`; past either end is 1924, as the product says it. */
  private screenForm(n: VmValue | undefined): VmValue {
    const form = this.screenForms()[Number(n ?? 0) - 1];
    if (!form) throw new HostError(1924, 'FORMS is not an object.');
    return { $obj: form.handle };
  }

  // ---- HostReads: called synchronously from inside wasm; no side effects here ----


  getProp(obj: number, name: string): VmValue | undefined {
    const value = this.readProp(obj, name);
    // a property holding a form that has been released holds .NULL. from then on, as a variable does
    return value !== null && typeof value === 'object' && '$obj' in value && this.gone.has(value.$obj) ? null : value;
  }

  private readProp(obj: number, name: string): VmValue | undefined {
    if (obj === APP_HANDLE) return this.appProp(name);
    if (obj === SCREEN_HANDLE) {
      const own = this.screenValues.get(name.toUpperCase());
      return own !== undefined ? own : this.screenProp(name);
    }
    const host = this.hosted.get(obj);
    // a property read happens inside wasm, so it can only be something that is already known
    if (host) return settled(this.fromHost(host.get(name)));
    const target = this.handles[obj];
    if (!target) return undefined;
    // an emulated ActiveX control answers what the form's own object does not
    if (target.ole && !target.has(name)) {
      const value = settled(this.fromHost(target.ole.get(name)));
      if (value !== undefined) return value;
    }
    const environment = this.dataEnvironmentOf(target, name);
    if (environment !== undefined) return { $obj: environment };
    if (target instanceof FormSetInstance) {
      const own = this.formSetProp(target, name);
      if (own !== undefined) return own;
    }
    const upper = name.toUpperCase();
    switch (upper) {
      case 'NAME':
        return target.renamed ?? target.name;
      // Parent is what contains the object, and a top-level form is contained by nothing: the
      // product answers TYPE("THISFORM.Parent") with "U" and raises 1924 on a read - measured -
      // where the screen would make it "O". A sample's Close button asks exactly that question
      // to tell a form in a form set from a form on its own, and answering _SCREEN sent it down
      // the THISFORMSET branch of every form.
      case 'PARENT':
        if (!target.parent) throw new HostError(1924, 'PARENT is not an object.');
        return { $obj: target.parent.handle };
      // BaseClass is what the object ultimately stands on; Class is what it was made from,
      // which is a different word only when it was made from a class of its own - one out of a
      // `DEFINE CLASS` or a class library. A control on a designed form has the two the same,
      // because the importer flattens a subclass into the base class underneath it.
      case 'CLASS':
        return target.className ?? target.baseClass;
      case 'BASECLASS':
        return target.baseClass;
      case 'CLASSLIBRARY':
        return target.classLibrary;
      case 'HWND':
        // a window handle is a number a program passes to the Windows API and compares for
        // identity; the object's own handle is unique in the same way, and is all there is
        return target.handle;
      // ParentClass is the class this object's class was made from, and not the object that
      // holds it: every base class the product ships answers it with nothing at all - measured.
      // A control on a designed form is always of a base class, because a subclass is flattened
      // into one when the form is imported; a class out of a library keeps its lineage.
      case 'PARENTCLASS':
        return target.parentClass;
      case 'CONTROLCOUNT':
        return target.children.length;
      // a page's place among its pageframe's pages, counting from 1, unless a program moved it;
      // CodeMine's ActivePage_Assign turns a page number into its PageOrder this way
      case 'PAGEORDER': {
        if (target.baseClass !== 'Page') break;
        const set = target.get('PageOrder');
        if (typeof set === 'number' && set > 0) return set;
        const pages = target.parent?.children.filter((c) => c.baseClass === 'Page') ?? [];
        return pages.indexOf(target) + 1;
      }
      case 'LISTCOUNT':
        return target.isListControl ? target.items.length : undefined;
      case 'LISTINDEX':
        return target.isListControl ? target.items.findIndex((i) => i.selected) + 1 : undefined;
      default: {
        // `THIS.Controls[i]` subscripts the member the same way `THIS.Controls(i)` calls it -
        // a sample's Init walks its own buttons that way - so the read hands back the members
        // for the subscript to land in. Where this and the product part company is a read with
        // no subscript at all, which the product refuses with 1924 and this answers with the
        // list, because by then the VM is holding the value rather than the member.
        const members = target.memberArray(name);
        if (members) return { $arr: members.map((m) => ({ $obj: m.handle })), $cols: 0 };
        const array = target.getArray(name);
        if (array) return array;
        // a property holding an object answers with the object, not with what it was before
        const held = target.objectValue(name);
        if (held !== undefined) return { $obj: held };
        // and one holding a date answers with the date rather than with how it displays
        const moment = target.momentValue(name);
        if (moment !== undefined) return moment;
        const value = target.get(name);
        return value === undefined ? undefined : propToVm(value);
      }
    }
  }

  private screenProp(name: string): VmValue | undefined {
    switch (name.toUpperCase()) {
      case 'NAME':
        return 'Screen';
      case 'CLASS':
      case 'BASECLASS':
        return 'Form';
      // the screen is contained by nothing either
      case 'PARENT':
        throw new HostError(1924, 'PARENT is not an object.');
      case 'FORMCOUNT':
        return this.visibleForms.length;
      case 'ACTIVEFORM':
        return this.forms.length ? { $obj: this.forms[this.forms.length - 1]!.handle } : null;
      case 'CAPTION':
        return 'FoxDev Studio';
      case 'VISIBLE':
        return true;
      // The main window's handle, which a program passes to the Windows API - CodeMine makes a
      // hidden marker window with it as the parent, so a second copy of the application can
      // find the first. The IDE's window handle cannot be had from here, and 0 is what Windows
      // reads as no parent at all, so the call still does what it is for.
      case 'HWND':
        return 0;
      case 'APPLICATION':
        return { $obj: APP_HANDLE };
      // `_SCREEN` is a Form - it answers "Form" for its own BaseClass - so everything a form
      // holds it holds too, and a program asking whether the desktop is wearing the Windows
      // theme is asking a form property. What the measurement found a fresh Form holding is
      // what it is told, which is a better answer than refusing the read.
      default: {
        const form = BASE_CLASS_MEMBERS['Form'];
        const declared = Object.keys(form?.properties ?? {}).find((p) => p.toUpperCase() === name.toUpperCase());
        return declared === undefined ? undefined : form?.properties[declared];
      }
    }
  }

  /** The data environment of a form, as a handle the VM can hold. */
  private dataEnvironmentOf(target: RuntimeObject | undefined, name: string): number | undefined {
    if (!(target instanceof FormInstance) || !target.dataEnvironment) return undefined;
    if (name.toUpperCase() !== 'DATAENVIRONMENT') return undefined;
    return this.hostHandle(target.dataEnvironment);
  }

  getMember(obj: number, name: string): number | 'prop' | 'method' | 'none' {
    if (obj === APP_HANDLE) {
      if (name.toUpperCase() === 'SCREEN') return SCREEN_HANDLE;
      if (name.toUpperCase() === 'APPLICATION') return APP_HANDLE;
      const form = this.findForm(name);
      if (form) return form.handle;
      if (name.toUpperCase() === 'FORMS') return 'method';
      return this.appProp(name) !== undefined ? 'prop' : METHOD_NAMES.has(name.toUpperCase()) ? 'method' : 'none';
    }
    if (obj === SCREEN_HANDLE) {
      if (name.toUpperCase() === 'APPLICATION') return APP_HANDLE;
      const form = this.findForm(name);
      if (form) return form.handle;
      const own = this.screenValues.get(name.toUpperCase());
      if (own !== undefined) return handleOf(own) ?? 'prop';
      if (name.toUpperCase() === 'FORMS') return 'method';
      return this.screenProp(name) !== undefined ? 'prop' : METHOD_NAMES.has(name.toUpperCase()) ? 'method' : 'none';
    }
    const host = this.hosted.get(obj);
    if (host) {
      // fetching a member to find out what it is would call it, on an object that may mind
      if (host.lazy) return host.member(name);
      const value = this.fromHost(host.get(name));
      if (value !== null && typeof value === 'object' && '$obj' in value) return value.$obj;
      return value === undefined ? host.member(name) : 'prop';
    }
    const target = this.handles[obj];
    if (!target) return 'none';
    const environment = this.dataEnvironmentOf(target, name);
    if (environment !== undefined) return environment;
    const child = target.child(name);
    if (child) return child.handle;
    // a property holding an object is reached through like a member, which is how
    // `THISFORM.oToolbar.Left` finds the toolbar the form keeps
    const held = target.objectValue(name);
    // one holding a form that has since been released holds .NULL., which only a read can give
    if (held !== undefined) return this.gone.has(held) ? 'prop' : held;
    // what a formset answers beyond a form: how many forms it holds, which of them is active,
    // and `Forms(n)`, which reads like a method call because it takes a subscript
    if (target instanceof FormSetInstance) {
      if (this.formSetProp(target, name) !== undefined) return 'prop';
      if (name.toUpperCase() === 'FORMS') return 'method';
    }
    if (target.ole && !target.has(name)) {
      // `oTree.Nodes` is a member with an object behind it: the VM needs its handle
      const value = this.fromHost(target.ole.get(name));
      if (value !== null && typeof value === 'object' && '$obj' in value) return value.$obj;
      if (value !== undefined) return 'prop';
      const member = target.ole.member(name);
      if (member !== 'none') return member;
    }
    if (target.hasClassMethod(name)) return 'method';
    if (target.has(name)) return 'prop';
    // a list control's own properties: ListCount and the indexed ones its item list answers
    if (target.isListControl && LIST_PROPS.has(name.toUpperCase())) return 'prop';
    if (target.hasMethodName(name) || target.hasEvent(name) || target.hasOwnMethod(name)) return 'method';
    // An ActiveX control answers through COM, which this runtime does not speak. Saying so is
    // worth doing, but not from here: this runs inside the VM, and an exception thrown across
    // that boundary leaves the VM borrowed and every call after it panics. The message is given
    // where a method call is performed instead, which is off the VM's stack.
    return 'none';
  }

  /**
   * Every member of an object: what AMEMBERS() lists and GETPEM() reads.
   *
   * `undefined` says the handle names nothing this can enumerate, and both functions then refuse
   * the call. `_SCREEN`, the application object and anything reached over COM answer that way:
   * their members are worked out one name at a time and there is no list to hand over.
   */
  members(obj: number): MemberEntry[] | undefined {
    // a hosted object that can say what it is made of says so; one that cannot is left to the
    // refusal below, as anything reached over COM is
    const hosted = this.hosted.get(obj);
    if (hosted) return hosted.list?.();
    const target = this.handles[obj];
    if (!target || !target.alive) return undefined;
    const declared = new Set(getObjectDescriptor(target.node).properties.map((p) => p.name.toUpperCase()));
    const out: MemberEntry[] = [];
    for (const name of target.propertyNames()) {
      out.push({
        name: name.toUpperCase(),
        kind: 'Property',
        native: declared.has(name.toUpperCase()),
        added: target.wasAdded(name),
        readOnly: target.refusesWrite(name) !== undefined,
        changed: target.wasWritten(name),
        value: this.getProp(obj, name),
      });
    }
    const plain = { native: true, added: false, readOnly: false, changed: false };
    for (const name of target.eventNameList()) out.push({ name: name.toUpperCase(), kind: 'Event', ...plain });
    for (const name of target.methodNameList()) out.push({ name: name.toUpperCase(), kind: 'Method', ...plain });
    // a method the class wrote for itself is the object's own, not the base class's
    for (const name of Object.keys(target.node.methods)) {
      if (!target.hasEvent(name) && !target.hasMethodName(name)) {
        out.push({ name: name.toUpperCase(), kind: 'Method', ...plain, native: false });
      }
    }
    for (const child of target.children) {
      out.push({
        name: child.name.toUpperCase(),
        kind: 'Object',
        native: false,
        added: false,
        readOnly: false,
        changed: false,
        value: { $obj: child.handle },
      });
    }
    return out;
  }

  /** Forms, and what was on them, that have been released; references to them read as .NULL. */
  private gone = new Set<number>();

  released(obj: number): boolean {
    return this.gone.has(obj);
  }

  hasCodeMethod(obj: number, name: string): boolean {
    const target = this.handles[obj];
    return target !== undefined && (target.hasClassMethod(name) || target.hasOwnMethod(name));
  }

  /** What `FOR EACH` walks: the forms of the screen, or of a formset. */
  enumerate(obj: number, name: string): VmValue | undefined {
    if (name.toUpperCase() !== 'FORMS') return undefined;
    const target = this.handles[obj];
    const forms = obj === SCREEN_HANDLE || obj === APP_HANDLE ? this.screenForms() : target instanceof FormSetInstance ? target.forms : undefined;
    return forms && { $arr: forms.map((f) => ({ $obj: f.handle })), $cols: 0 };
  }

  /** `DEFINE CLASS ... PROCEDURE Error`: what makes the VM route an error to the object. */
  hasClassMethod(obj: number, name: string): boolean {
    return this.handles[obj]?.hasClassMethod(name) ?? false;
  }

  objectClass(obj: number): string | null {
    if (obj === SCREEN_HANDLE) return 'Form';
    // the application is an object like any other as far as the VM is concerned: without this
    // `_VFP.FullName` is read against something the VM does not believe is an object at all
    if (obj === APP_HANDLE) return 'Application';
    const host = this.hosted.get(obj);
    if (host) return host.className;
    const target = this.handles[obj];
    return target && target.alive ? target.baseClass : null;
  }

  /**
   * `SYS(1271, oObject)`: the file the object was built from. A form answers with its document,
   * which is how form code finds the folder it is running out of; anything with no file of its
   * own answers with nothing, and the product then answers .F. rather than an empty string.
   */
  objectFile(obj: number): string | null {
    const target = this.handles[obj];
    return target?.alive && target.file !== '' ? target.file : null;
  }

  output(text: string, newline: boolean): void {
    if (newline || this.outputLines.length === 0) this.outputLines.push(text);
    else this.outputLines[this.outputLines.length - 1] += text;
    this.notify();
  }

  now(): { days: number; secs: number } {
    const d = this.clock();
    const midnight = new Date(d.getFullYear(), d.getMonth(), d.getDate());
    return { days: Math.floor(midnight.getTime() / 86400000 - midnight.getTimezoneOffset() / 1440), secs: (d.getTime() - midnight.getTime()) / 1000 };
  }

  random(): number {
    return this.rng();
  }

  resolveProgram(name: string): number {
    return this.programs.get(name.toLowerCase()) ?? -1;
  }

  // ---- SET LIBRARY TO: a Visual FoxPro library, hosted where a 32-bit image can be loaded ----

  loadLibrary(path: string): { id: number; path: string; functions: string[] } | { error: string } {
    if (!this.libraries?.available()) return { error: 'a Visual FoxPro library cannot be hosted here' };
    try {
      const loaded = this.libraries.load(path);
      this.say(loaded.output);
      // a function the library declared but does not want called keeps its place so the
      // numbering the host uses still lines up: internal, or run on load, or run on unload
      return {
        id: loaded.id,
        path: loaded.path,
        functions: loaded.functions.map((f) => (f.parmCount >= 0 ? f.name : '')),
      };
    } catch (error) {
      return { error: error instanceof Error ? error.message : String(error) };
    }
  }

  callLibrary(
    library: number,
    fn: number,
    args: VmValue[],
  ): { ok: true; value: VmValue; refs: { index: number; value: VmValue }[] } | { ok: false; code: number; message: string } {
    if (!this.libraries) return { ok: false, code: 1726, message: 'API library is not found.' };
    let answer;
    try {
      answer = this.libraries.call(library, fn, args.map(toLibraryValue));
    } catch (error) {
      return { ok: false, code: 1726, message: error instanceof Error ? error.message : String(error) };
    }
    this.say(answer.output);
    // _Error(n) raises the product's error n; _UserError(text) is 1098, in the library's words
    if (answer.error > 0) return { ok: false, code: answer.error, message: '' };
    if (answer.error < 0) return { ok: false, code: 1098, message: answer.errorText };
    return {
      ok: true,
      value: fromLibraryValue(answer.value),
      refs: (answer.refs ?? []).map((r) => ({ index: r.index, value: fromLibraryValue(r.value) })),
    };
  }

  unloadLibrary(library: number): void {
    this.libraries?.unload(library);
  }

  /**
   * What a library printed with _PutStr, which is a stream rather than a list of lines: what
   * comes before the first newline joins the line already there. Measured with the API sample
   * hello.fll, whose one function writes "\nHello, World!\n" - the product ends the line the
   * program was on, writes the greeting on the next, and leaves a blank one after it.
   */
  private say(text: string): void {
    if (text === '') return;
    const pieces = text.split('\n').map((piece) => piece.replace(/\r$/, ''));
    this.output(pieces[0]!, false);
    for (const rest of pieces.slice(1)) this.output(rest, true);
  }

  /**
   * Every error the VM raises, handled or not. A program that handles its own errors - the
   * Visual FoxPro samples all do - otherwise leaves nothing to go on but whatever its handler
   * chose to print, which is a message with no place.
   */
  errorRaised(error: RuntimeError, handled: boolean): void {
    this.onErrorRaised(error, handled);
  }

  /** Injected by the session; the IDE writes these to Output. */
  onErrorRaised: (error: RuntimeError, handled: boolean) => void = () => {};

  // ---- host request side: called with the VM off the stack, so events may run ----

  /**
   * `THISFORM.x.Caption = v`. Fires ProgrammaticChange for Value, like VFP, and raises the
   * same error a read would when the property does not exist.
   */
  setProp(obj: number, name: string, value: VmValue): void {
    // `_VFP` too - CodeMine puts the main window back where it was with `_VFP.Height = ...` -
    // and a name it does not have is the automation error the product raises (measured)
    if (obj === APP_HANDLE) {
      const upper = name.toUpperCase();
      if (!this.appValues.has(upper) && this.appProp(name) === undefined) {
        throw new HostError(1426, 'OLE error code 0x80020006: Unknown name.');
      }
      this.appValues.set(upper, value);
      return;
    }
    // the screen holds what it is told; a name it has never had is 1734, as on any form
    if (obj === SCREEN_HANDLE) {
      const upper = name.toUpperCase();
      if (!this.screenValues.has(upper) && this.screenProp(name) === undefined) {
        throw new HostError(1734, `Property ${upper} is not found.`);
      }
      this.screenValues.set(upper, value);
      return;
    }
    const host = this.hosted.get(obj);
    if (host) {
      if (host.member(name) === 'none' && host.get(name) === undefined) {
        throw new HostError(1734, `Property ${name.toUpperCase()} is not found.`);
      }
      host.set(name, value);
      return;
    }
    const target = this.handles[obj];
    if (!target) throw new HostError(1943, 'Member  does not evaluate to an object.');
    if (!target.has(name) && !(target.isListControl && LIST_PROPS.has(name.toUpperCase()))) {
      // an emulated ActiveX control owns every name the form's object does not
      if (target.ole && (target.ole.member(name) !== 'none' || target.ole.get(name) !== undefined)) {
        target.ole.set(name, value);
        return;
      }
      throw new HostError(1734, `Property ${name.toUpperCase()} is not found.`);
    }
    // a form's Class, a page frame's PageWidth, a text box's Text: the product works these out
    // and will not have them written, and says so rather than quietly taking the write
    const refused = target.refusesWrite(name);
    if (refused !== undefined) throw new HostError(refused, `${name.toUpperCase()} is a read-only property`);
    // a property may hold an object: a form keeps the toolbar it put up in one of its own
    if (name.toUpperCase() === 'NAME' && typeof value === 'string') target.renamed = value;
    const object = handleOf(value);
    if (object !== undefined) target.setObjectValue(name, object);
    else if (isDate(value) || isDateTime(value)) target.setMomentValue(name, value);
    else target.set(name, vmToProp(value), 'program');
  }

  /**
   * The Exception object `CATCH TO oErr` binds. It is not on the desktop and has no window:
   * it exists to be read, so it is registered as a handle and nothing else.
   */
  createException(info: { code: number; message: string; program: string; line: number; user_value: VmValue }): number {
    const node: FormNode = {
      name: 'Exception',
      props: {
        ErrorNo: info.code,
        Message: info.message,
        Procedure: info.program,
        LineNo: info.line,
        LineContents: '',
        Details: '',
        StackLevel: 1,
        UserValue: vmToProp(info.user_value),
      },
      methods: {},
      children: [],
    };
    const instance = new FormInstance(this.nextHandle++, node, this);
    instance.className = 'Exception';
    instance.nonVisual = true;
    this.handles[instance.handle] = instance;
    return instance.handle;
  }

  /** `DIMENSION THISFORM.aRows[2, 3]`: the array property is given that shape. */
  dimProp(obj: number, name: string, rows: number, cols: number): void {
    const target = this.handles[obj];
    if (!target) throw new HostError(1943, 'Member  does not evaluate to an object.');
    target.dimension(name, rows, cols);
  }

  /** `THISFORM.x.Picture[2] = v`: one element of an array property. */
  setPropIndex(obj: number, name: string, index: readonly number[], value: VmValue): void {
    const target = this.handles[obj];
    if (!target) throw new HostError(1943, 'Member  does not evaluate to an object.');
    target.setElement(name, index, value);
  }

  /** Built-in object methods. Returns `undefined` when the name is not one of them. */
  /** Puts what an array holds on the clipboard, a row a line and a tab between the columns. */
  private toClipboard(rows: VmValue): VmValue {
    const items = rows && typeof rows === 'object' && '$arr' in rows ? rows.$arr : [];
    const columns = rows && typeof rows === 'object' && '$cols' in rows ? Number(rows.$cols ?? 0) : 0;
    const text: string[] = [];
    for (let at = 0; at < items.length; at += Math.max(columns, 1)) {
      const row = items.slice(at, at + Math.max(columns, 1));
      text.push(row.map((v) => (v === null || v === undefined ? '' : String(v))).join('\t'));
    }
    const written = text.join('\r\n');
    // the clipboard is the browser's, and a test environment may not have one
    const board = (globalThis as { navigator?: { clipboard?: { writeText?(text: string): Promise<void> } } }).navigator;
    void board?.clipboard?.writeText?.(written)?.catch(() => undefined);
    return text.length;
  }

  callMethod(obj: number, name: string, args: VmValue[]): VmValue | Promise<VmValue> | undefined {
    const host = this.hosted.get(obj);
    if (host) {
      const result = this.fromHost(host.call(name, args));
      if (result === undefined) throw new HostError(1925, `Unknown member ${name.toUpperCase()}.`);
      return result;
    }
    if ((obj === SCREEN_HANDLE || obj === APP_HANDLE) && name.toUpperCase() === 'FORMS') return this.screenForm(args[0]);
    const target = obj === SCREEN_HANDLE ? undefined : this.handles[obj];
    // a formset shows, hides and releases all of its forms at once, and hands them out by number
    if (target instanceof FormSetInstance) {
      switch (name.toUpperCase()) {
        case 'FORMS': {
          const form = target.forms[Number(args[0] ?? 0) - 1];
          return form ? { $obj: form.handle } : null;
        }
        case 'SHOW':
          this.showFormSet(target);
          return null;
        case 'HIDE':
          this.hideFormSet(target);
          return null;
        case 'RELEASE':
          return this.releaseFormSet(target).then(() => null);
        default:
          break;
      }
    }
    switch (name.toUpperCase()) {
      // the report listener: the engine calls these as it works through a report, and a
      // listener a program has not overridden answers what the base class answers
      case 'CANCELREPORT':
        target?.set('OutputPageCount', 0);
        return null;
      case 'SUPPORTSLISTENERTYPE': {
        // 0 page preview, 1 printer, 2 nothing at all, 3 XML: the base class takes them all
        const kind = Number(args[0] ?? -1);
        return kind >= 0 && kind <= 3;
      }
      case 'INCLUDEPAGEINOUTPUT':
        return true;
      case 'OUTPUTPAGE':
      case 'RENDER':
      case 'ONPREVIEWCLOSE':
        return null;
      case 'SETFOCUS':
        target?.setFocus();
        return null;
      case 'REFRESH':
        target?.refresh();
        return null;
      case 'RELEASE': {
        const form = target instanceof FormInstance ? target : target?.form();
        // measured: Release() answers .T.
        return form ? this.releaseForm(form, { queryUnload: false }).then(() => true) : null;
      }
      case 'SHOW':
        target?.set('Visible', true);
        return null;
      case 'HIDE':
        target?.set('Visible', false);
        return null;
      case 'MOVE': {
        const [left, top, width, height] = args;
        if (typeof left === 'number') target?.set('Left', left);
        if (typeof top === 'number') target?.set('Top', top);
        if (typeof width === 'number') target?.set('Width', width);
        if (typeof height === 'number') target?.set('Height', height);
        return null;
      }
      case 'CLS': {
        const form = target instanceof FormInstance ? target : target?.form();
        form?.clearDrawings();
        return null;
      }
      case 'DRAW':
        // everything drawn is kept, so drawing again is asking for a repaint
        target?.notifyChanged();
        return null;
      case 'DOVERB':
        throw new HostError(
          1429,
          'DoVerb(): a verb belongs to an embedded OLE document, and what this runtime reaches through COM is an automation object; call the object\'s own methods instead',
        );
      // Visual FoxPro refuses this one too, in these words and with this number: CloneObject
      // is the designer copying what it is designing, and a running program is not that.
      case 'CLONEOBJECT':
        throw new HostError(1953, 'Feature is only available if the object is in design mode.');
      // Print(cText, x, y): text drawn on the form where CurrentX and CurrentY say, and they
      // move on to where it ended
      case 'PRINT': {
        const form = target instanceof FormInstance ? target : target?.form();
        if (!form) return null;
        const [text, x, y] = args;
        const at = {
          x: typeof x === 'number' ? x : Number(form.get('CurrentX') ?? 0),
          y: typeof y === 'number' ? y : Number(form.get('CurrentY') ?? 0),
        };
        const said = typeof text === 'string' ? text : text === undefined || text === null ? '' : String(text);
        form.draw({
          shape: 'text',
          x1: at.x,
          y1: at.y,
          x2: at.x,
          y2: at.y,
          text: said,
          colour: Number(form.get('ForeColor') ?? 0),
          width: Number(form.get('DrawWidth') ?? 1),
          style: Number(form.get('DrawStyle') ?? 0),
        });
        form.set('CurrentX', at.x + said.length * Number(form.get('FontSize') ?? 9) * 0.6);
        form.set('CurrentY', at.y);
        return null;
      }
      // Drag(nAction): 1 begin, 2 drop, 0 cancel. What a drop lands on is what the pointer is
      // over, which the host tells the desktop as the pointer moves.
      case 'DRAG': {
        const action = args.length > 0 ? Number(args[0]) : 1;
        if (!target) return null;
        if (action === 1) {
          this.dragging = target;
          return null;
        }
        const carried = this.dragging;
        this.dragging = null;
        if (action === 2 && carried) {
          const over = this.pointerOver ?? target;
          const outcome = this.dispatch(over, 'DragDrop', [{ $obj: carried.handle }, 0, 0]);
          if (outcome instanceof Promise) return outcome.then(() => null);
        }
        return null;
      }
      // OLEDrag(lDetectDrag): the object is asked what it is putting on the DataObject, which
      // is what OLEStartDrag is for
      case 'OLEDRAG': {
        if (!target) return null;
        const carrier = registryObject('DataObject');
        const payload = carrier ? { $obj: this.hostHandle(carrier) } : null;
        const outcome = this.dispatch(target, 'OLEStartDrag', [payload]);
        return outcome instanceof Promise ? outcome.then(() => true) : true;
      }
      case 'RESET':
        // a timer's reset starts its interval again, which the timer service does by rebinding
        target?.notifyChanged();
        return null;
      case 'BOX':
      case 'LINE': {
        const form = target instanceof FormInstance ? target : target?.form();
        if (!form) return null;
        const [x1, y1, x2, y2] = args.map((a) => (typeof a === 'number' ? a : 0));
        form.draw({
          shape: name.toUpperCase() === 'BOX' ? 'box' : 'line',
          x1: x1 ?? 0,
          y1: y1 ?? 0,
          x2: x2 ?? 0,
          y2: y2 ?? 0,
          colour: Number(target?.get('ForeColor') ?? 0),
          width: Number(target?.get('DrawWidth') ?? 1),
          style: Number(target?.get('DrawStyle') ?? 0),
        });
        return null;
      }
      case 'CIRCLE': {
        const form = target instanceof FormInstance ? target : target?.form();
        if (!form) return null;
        const [radius, x, y, aspect] = args.map((a) => (typeof a === 'number' ? a : 0));
        form.draw({
          shape: 'circle',
          x1: x ?? 0,
          y1: y ?? 0,
          x2: x ?? 0,
          y2: y ?? 0,
          radius: radius ?? 0,
          aspect: aspect || 1,
          colour: Number(target?.get('ForeColor') ?? 0),
          width: Number(target?.get('DrawWidth') ?? 1),
          style: Number(target?.get('DrawStyle') ?? 0),
        });
        return null;
      }
      case 'PSET': {
        const form = target instanceof FormInstance ? target : target?.form();
        if (!form) return null;
        const [x, y] = args.map((a) => (typeof a === 'number' ? a : 0));
        form.draw({
          shape: 'point',
          x1: x ?? 0,
          y1: y ?? 0,
          x2: x ?? 0,
          y2: y ?? 0,
          colour: Number(target?.get('ForeColor') ?? 0),
          width: Number(target?.get('DrawWidth') ?? 1),
          style: 0,
        });
        return null;
      }
      case 'POINT':
        // the colour of a pixel, which needs the drawn image back: nothing here keeps one, and
        // -1 is what VFP answers for a point it cannot read
        return -1;
      case 'TEXTWIDTH':
      case 'TEXTHEIGHT': {
        // measured from the font, as a window with no fonts loaded can: the average character
        // of a proportional face is about half its size, and a line is about a fifth taller
        const text = String(vmToProp(args[0] ?? '') ?? '');
        const size = Number(target?.get('FontSize') ?? 9);
        const bold = target?.get('FontBold') === true ? 1.05 : 1;
        if (name.toUpperCase() === 'TEXTHEIGHT') return Math.round(size * 1.6);
        return Math.round(text.length * size * 0.55 * bold);
      }
      case 'ZORDER': {
        // 0 brings the control to the front of its parent, 1 sends it to the back
        const [where] = args;
        target?.parent?.moveChild(target, where === 1 ? 'back' : 'front');
        return null;
      }
      case 'ADDPROPERTY': {
        const [property, value] = args;
        if (typeof property !== 'string') return false;
        if (obj === SCREEN_HANDLE) return this.addProperty(SCREEN_HANDLE, property, value ?? null);
        if (!target) return false;
        return this.addProperty(target.handle, property, value ?? null);
      }
      case 'RESETTODEFAULT': {
        const [property] = args;
        if (typeof property !== 'string' || !target) return null;
        const meta = getPropertyMeta(getObjectDescriptor(target.node), property);
        if (meta) target.set(property, meta.default, 'program');
        return null;
      }
      case 'READMETHOD': {
        const [method] = args;
        return typeof method === 'string' ? (target?.methodSource(method) ?? '') : '';
      }
      case 'WRITEMETHOD': {
        const [method, code] = args;
        if (typeof method === 'string' && typeof code === 'string') target?.setMethodSource(method, code);
        return null;
      }
      case 'READEXPRESSION':
        // a property bound to an expression is a design-time thing, and a running form has none
        return '';
      case 'WRITEEXPRESSION':
      case 'SAVEASCLASS':
      case 'SHOWWHATSTHIS':
      case 'WHATSTHISMODE':
      case 'HELP':
      case 'DOMESSAGE':
      case 'DOSTATUS':
      case 'CLEARSTATUS':
      case 'UPDATESTATUS':
        // what these ask for is a development environment or a status bar, which a running form
        // does not have; VFP does nothing visible with them either when there is none
        return null;
      case 'DOSCROLL':
      case 'SETVIEWPORT': {
        const [first, second] = args;
        if (name.toUpperCase() === 'SETVIEWPORT') {
          if (typeof first === 'number') target?.set('ViewPortLeft', first);
          if (typeof second === 'number') target?.set('ViewPortTop', second);
        }
        void this.dispatch(target!, 'Scrolled', [typeof first === 'number' ? first : 0]);
        return null;
      }
      // a page frame answers its own size; a report listener answers the page it is printing
      case 'GETPAGEHEIGHT':
        return Number(target?.has('PageHeight') ? target.get('PageHeight') : (target?.get('Height') ?? 0));
      case 'GETPAGEWIDTH':
        return Number(target?.has('PageWidth') ? target.get('PageWidth') : (target?.get('Width') ?? 0));
      case 'DOCK': {
        const [where] = args;
        if (typeof where === 'number') {
          target?.set('DockPosition', where);
          target?.set('Docked', where >= 0);
          void this.dispatch(target!, where >= 0 ? 'AfterDock' : 'UnDock', []);
        }
        return null;
      }
      case 'GETDOCKSTATE':
        return Number(target?.get('DockPosition') ?? -1);
      case 'MOVEITEM': {
        // `MoveItem(nIndex, nNewIndex)`: an item of a list, moved to another place in it
        const [from, to] = args;
        if (typeof from === 'number' && typeof to === 'number') target?.moveItem(from, to);
        return null;
      }
      // The two that translate between a row's position and the id it keeps. Measured: a
      // position or an id that is not there answers 0 rather than raising, and a call with no
      // argument at all is refused - which is the difference between asking about a row that
      // has gone and not asking about a row.
      case 'INDEXTOITEMID':
      case 'ITEMIDTOINDEX': {
        const [which] = args;
        if (typeof which !== 'number') throw new HostError(11, 'Function argument value, type, or count is invalid.');
        return name.toUpperCase() === 'INDEXTOITEMID' ? (target?.itemIdAt(which) ?? 0) : (target?.indexOfItemId(which) ?? 0);
      }
      case 'ACTIVATECELL': {
        const [row, column] = args;
        if (typeof row === 'number') target?.set('ActiveRow', row);
        if (typeof column === 'number') target?.set('ActiveColumn', column);
        return null;
      }
      case 'ADDCOLUMN': {
        const [at] = args;
        if (!target) return null;
        const columns = target.children.length + 1;
        target.set('ColumnCount', columns);
        void at;
        return null;
      }
      case 'DELETECOLUMN': {
        const [at] = args;
        if (!target) return null;
        const index = typeof at === 'number' ? at : target.children.length;
        const column = target.children[index - 1];
        if (column) this.removeObject(target, column.name);
        return null;
      }
      case 'AUTOFIT':
        // every column as wide as what is in it: nothing here measures text on a canvas, so the
        // columns keep the widths they were given
        target?.notifyChanged();
        return null;
      case 'QUIT':
        // the application object's Quit is the QUIT command, which the session performs
        this.quitRequested?.();
        return null;
      // the application object runs and works out what a program hands it, which is the VM's
      // to do: the session hands these three in
      case 'DOCMD': {
        const [text] = args;
        if (typeof text !== 'string' || !this.runLine) return null;
        return this.runLine(text).then(() => null);
      }
      case 'EVAL': {
        const [expression] = args;
        if (typeof expression !== 'string' || !this.evaluate) return null;
        return this.evaluate(expression);
      }
      case 'SETVAR': {
        const [name_, value] = args;
        if (typeof name_ !== 'string' || !this.setVariable) return false;
        this.setVariable(name_, value ?? null);
        return true;
      }
      // RequestData([area][, nRecords]): the records of a table as an array, which is what
      // COPY TO ARRAY makes; DataToClip puts the same thing on the clipboard as text
      case 'REQUESTDATA':
      case 'DATATOCLIP': {
        if (!this.runLine || !this.evaluate) return null;
        const [where, count] = args;
        const select = typeof where === 'string' ? `SELECT ${where}` : typeof where === 'number' ? `SELECT ${where}` : '';
        const scope = typeof count === 'number' && count > 0 ? ` NEXT ${Math.trunc(count)}` : '';
        const lines = ['PUBLIC __takedata', 'DIMENSION __takedata(1)'];
        if (select) lines.push(select);
        lines.push(`COPY TO ARRAY __takedata${scope}`);
        const wanted = name.toUpperCase();
        return this.runLine(lines.join('\n'))
          .then(() => this.evaluate!('__takedata'))
          .then((rows) => (wanted === 'REQUESTDATA' ? rows : this.toClipboard(rows)));
      }
      case 'SETALL': {
        // `SetAll(cProperty, uValue [, cClass])`: every control inside, optionally of one class
        const [property, value, className] = args;
        if (typeof property !== 'string' || !target) return null;
        const wanted = typeof className === 'string' ? className.toLowerCase() : undefined;
        for (const child of target.descendants()) {
          if (wanted && child.baseClass.toLowerCase() !== wanted && child.node.name.toLowerCase() !== wanted) continue;
          if (child.has(property)) child.set(property, value as PropValue, 'program');
        }
        return null;
      }
      case 'ADDITEM': {
        const [text, at, column] = args;
        target?.addItem(String(vmToProp(text ?? '') ?? ''), typeof at === 'number' ? at : undefined, typeof column === 'number' ? column : 1);
        return null;
      }
      // AddListItem names the row by the id it gives it, where AddItem names it by position
      case 'ADDLISTITEM': {
        const [text, itemId, column] = args;
        target?.addItem(String(vmToProp(text ?? '') ?? ''), typeof itemId === 'number' ? itemId : undefined, typeof column === 'number' ? column : 1, true);
        return null;
      }
      case 'REMOVEITEM':
      case 'REMOVELISTITEM': {
        const [at] = args;
        if (typeof at === 'number') target?.removeItem(at);
        return null;
      }
      case 'CLEAR':
        target?.clearItems();
        return null;
      case 'REQUERY':
        return null;
      case 'ADDOBJECT': {
        // `AddObject(cName, cClass [, cOLEClass])`: the third argument says which ActiveX
        // control an OleControl stands for, which is the only way a control made while the
        // program runs can name one. `AddObject("ole1", "olecontrol", "WMPlayer.OCX")` is how
        // the samples reach the Media Player.
        const [name, klass, ole] = args;
        if (obj === SCREEN_HANDLE) return this.screenAddObject(String(vmToProp(name ?? '') ?? ''), String(vmToProp(klass ?? '') ?? ''));
        if (!target) throw new HostError(1943, 'Member  does not evaluate to an object.');
        return this.addObject(
          target,
          String(vmToProp(name ?? '') ?? ''),
          String(vmToProp(klass ?? '') ?? ''),
          String(vmToProp(ole ?? '') ?? ''),
        );
      }
      // `NewObject(cName, cClass [, cModule])` is AddObject with a file to read the class out
      // of. Measured: it answers .T., the control arrives hidden as AddObject's does, and a
      // base class with no file named is exactly AddObject.
      case 'NEWOBJECT': {
        const [name, klass, module] = args;
        if (!target) throw new HostError(1943, 'Member  does not evaluate to an object.');
        return this.newObject(
          target,
          String(vmToProp(name ?? '') ?? ''),
          String(vmToProp(klass ?? '') ?? ''),
          String(vmToProp(module ?? '') ?? ''),
        );
      }
      case 'REMOVEOBJECT': {
        const [name] = args;
        if (target) this.removeObject(target, String(vmToProp(name ?? '') ?? ''));
        return null;
      }
      default: {
        // `oCombo.Selected(1)` reads an indexed property with call syntax, which VFP allows
        const element = this.readIndexed(target, name, args);
        if (element !== undefined) return element;
        // a method the object carries source for: a form's own CenterForm, GetDirectory and the
        // rest are called exactly like the ones this switch handles
        if (target?.hasOwnMethod(name)) {
          const outcome = this.dispatch(target, name, args);
          if (outcome === null) return null;
          return outcome instanceof Promise ? outcome.then((o) => o.value) : outcome.value;
        }
        if (target?.ole) {
          const answered = this.fromHost(target.ole.call(name, args));
          if (answered !== undefined) return answered;
        }
        if (target?.isOleControl) {
          throw new HostError(1429, `${target.name} is an ActiveX control; this runtime cannot call into COM`);
        }
        // an event the object has, called with no code written for it, answers .T. - measured on
        // Init, Click, Destroy, Valid and the rest, which CodeMine leans on to fire a page's Init
        if (target && (target.hasEvent(name) || target.hasMethodName(name))) return true;
        return undefined;
      }
    }
  }

  /** `obj.Prop(2)` as an expression: an array property read through call syntax. */
  private readIndexed(target: RuntimeObject | undefined, name: string, args: VmValue[]): VmValue | undefined {
    const subscripts = args.every((a) => typeof a === 'number') ? (args as number[]) : undefined;
    if (!target) return undefined;
    // `THISFORM.pgf.Pages(2)`: the member is the container's children of a kind, reached by
    // position. Measured: a subscript outside the list - and no subscript at all - leaves
    // nothing that is an object, which the product reports against the member's own name.
    const members = target.memberArray(name);
    if (members) {
      const found = subscripts?.length === 1 ? members[(subscripts[0] ?? 0) - 1] : undefined;
      if (!found) throw new HostError(1924, `${name.toUpperCase()} is not an object.`);
      return { $obj: found.handle };
    }
    if (!subscripts || subscripts.length < 1 || subscripts.length > 2) return undefined;
    const array = target.getArray(name);
    if (!array) return undefined;
    // a row and a column on a table, one running subscript on anything
    const at =
      subscripts.length === 2 && array.$cols > 0
        ? ((subscripts[0] ?? 0) - 1) * array.$cols + (subscripts[1] ?? 0)
        : subscripts.length === 2
          ? 0
          : (subscripts[0] ?? 0);
    if (at < 1 || at > array.$arr.length) throw new HostError(31, 'Subscript is outside defined range');
    return array.$arr[at - 1] ?? null;
  }

  /**
   * `AddObject(cName, cClass)`: a control created while the form runs. VFP adds it hidden, so
   * the code that follows can position it before it appears.
   */
  /**
   * `_SCREEN.AddObject(cName, cClass)`: an object of the class, made as CREATEOBJECT makes one,
   * held by the screen under that name. A CodeMine application keeps its global object manager
   * there - `_SCREEN.AddObject('cmGlobalObjectManager', 'cmGlobalObjectManager')`, a class its
   * procedure file defines - and reaches it as `_SCREEN.cmGlobalObjectManager` from then on.
   */
  private async screenAddObject(name: string, className: string): Promise<VmValue> {
    if (!this.createNamedObject) throw new HostError(1733, `Class definition ${className.toUpperCase()} is not found.`);
    const made = await this.createNamedObject(className);
    // an Init that refused leaves the screen without the member, as it leaves a form without one
    if (made === null || made === false) return false;
    this.screenValues.set(name.toUpperCase(), made);
    const handle = handleOf(made);
    if (handle !== undefined) this.handles[handle]?.set('Name', name.toUpperCase(), 'program');
    return true;
  }

  private addObject(parent: RuntimeObject, name: string, className: string, oleClass = ''): Promise<VmValue> | VmValue {
    const type = baseClassToControlType(className);
    if (type === null || type === 'Form') throw new HostError(1733, `Class definition ${className.toUpperCase()} is not found.`);

    // measured: the product upper-cases the name a control is added under, so that is what the
    // new control answers for its own Name afterwards
    const props: Record<string, PropValue> = { Visible: false };
    if (oleClass.trim() !== '') props['OleClass'] = oleClass.trim();
    const node: ControlNode = { id: `rt-${this.nextHandle}`, type, name: name.toUpperCase(), props, methods: {}, children: [] };
    const child = new RuntimeObject(this.nextHandle++, node, this);
    this.handles[child.handle] = child;
    // a control added while the program runs reaches its ActiveX control the same way one the
    // form was designed with does
    this.attachOle(child);
    parent.addChild(child);
    this.notify();
    return this.fire(child, 'Init').then(() => null);
  }

  /**
   * `NewObject(cName, cClass [, cModule])`: the same as AddObject for a base class, and for
   * anything else the class is looked for in the file named, or in the libraries `SET CLASSLIB`
   * has loaded when the call named no file.
   *
   * Measured in Visual FoxPro 9: the call answers .T., the new member is hidden as AddObject's
   * is, and its Name is upper-cased.
   */
  private async newObject(parent: RuntimeObject, name: string, className: string, module: string): Promise<VmValue> {
    if (module === '' && baseClassToControlType(className) !== null) {
      await this.addObject(parent, name, className);
      return true;
    }
    const made = await this.libraryObject?.(className, module, { parent, name: name.toUpperCase() });
    // a class nobody has is the program naming something that is not there; a class whose own
    // Init refused is not - the container simply has no member, as a form has no control when
    // one of its controls declines
    if (made === undefined) throw new HostError(1733, `Class definition ${className.toUpperCase()} is not found.`);
    return true;
  }

  /**
   * An object of a class read out of a class library: the tree the class describes, brought to
   * life with the module its own methods were compiled into.
   *
   * A class whose base class is a form, or one with nothing on screen at all, is a form here
   * for the same reason a `DEFINE CLASS` of one is: the form is what carries a module and a
   * data environment. Anything else is a control, which is what lets a Grid column make one for
   * itself - `into` is that container, and a member added this way arrives hidden, as one added
   * with AddObject does.
   *
   * Nothing runs before the whole tree exists, and then the members' Inits run deepest first,
   * which is the order a container is built in everywhere in Visual FoxPro.
   */
  async createClassObject(
    node: FormNode,
    module: number,
    info: {
      className: string;
      baseClass: string;
      library: string;
      parentClass: string;
      arrays?: Record<string, [number, number]>;
      expressions?: Record<string, string>;
    },
    into?: { parent: RuntimeObject; name: string },
  ): Promise<RuntimeObject | null> {
    const type = baseClassToControlType(info.baseClass);
    const nonVisual = isNonVisualBaseClass(info.baseClass) || type === null;

    // A non-visual class standing on its own is an object of its own; one added to a container -
    // `THIS.NewObject('oContextMenu', 'cmContextMenuManager')`, a Custom - is a member like any
    // control, when its base class is one a form can hold
    if (type === 'Form' || (nonVisual && !(into && type !== null))) {
      // a form is a window: it is not a member of anything, which is what the product says too
      if (into) throw new HostError(1733, `Class definition ${info.className.toUpperCase()} is not found.`);
      const instance = this.instantiate(node, module, { className: spellClass(info.className), nonVisual, noshow: true, arrays: info.arrays });
      instance.parentClass = spellClass(info.parentClass);
      instance.classLibrary = info.library;
      instance.declaredBaseClass = spellClass(info.baseClass);
      const created = await this.runFormLifecycle(instance, { noshow: true, expressions: info.expressions });
      return created ? instance : null;
    }

    const root: ControlNode = { id: `rt-${this.nextHandle}`, type, name: into?.name ?? node.name, props: { ...node.props }, methods: node.methods, children: node.children as ControlNode[] };
    // measured: a member added while the form runs arrives hidden, so the code that follows can
    // put it where it belongs before it appears
    if (into) root.props['Visible'] = false;
    const object = new RuntimeObject(this.nextHandle++, root, this);
    object.module = module;
    object.className = spellClass(info.className);
    object.parentClass = spellClass(info.parentClass);
    object.classLibrary = info.library;
    object.declaredBaseClass = spellClass(info.baseClass);
    this.handles[object.handle] = object;
    this.attachOle(object);
    this.buildChildren(root.children ?? [], object);
    this.dimensionArrays(object, info.arrays);
    into?.parent.addChild(object);
    this.notify();

    await this.workOutProperties(object, info.expressions);
    for (const member of object.descendants().reverse()) {
      if (!member.alive || !member.parent) continue;
      const outcome = await this.fire(member, 'Init');
      if (outcome?.value === false) this.dropControl(member);
    }
    const init = await this.fire(object, 'Init');
    if (init?.value === false) {
      this.dropControl(object);
      return null;
    }
    return object;
  }

  /** Builds the live objects for a tree of nodes under `parent`. */
  private buildChildren(nodes: readonly ControlNode[], parent: RuntimeObject): void {
    for (const node of nodes) {
      const child = new RuntimeObject(this.nextHandle++, node, this);
      this.handles[child.handle] = child;
      this.attachOle(child);
      parent.addChild(child);
      if (node.children?.length) this.buildChildren(node.children, child);
    }
  }

  /** The object a `fdvpanel.cmdgo` path names, counted from the class's own root. */
  private memberAt(root: RuntimeObject, path: string): RuntimeObject | undefined {
    let at: RuntimeObject | undefined = root;
    for (const part of path.split('.').slice(1)) {
      at = at?.child(part);
      if (!at) return undefined;
    }
    return at;
  }

  /**
   * `CREATEOBJECT("Form")`, `CREATEOBJECT("Label")`: an object of a Visual FoxPro base class,
   * standing on its own rather than inside a form.
   *
   * Every base class is a class a program may ask for by name - the SDI Form sample builds its
   * child windows with `CreateObject('form')` - and a name we do not answer to falls through to
   * COM, where it comes back as "Invalid class string". It arrives hidden, as VFP's does: a form
   * made this way is on screen only once the program calls Show.
   */
  createBaseObject(className: string): RuntimeObject | undefined {
    const type = baseClassToControlType(className);
    if (type === null) return undefined;
    if (type === 'Form') {
      const node: FormNode = { name: className, props: { Visible: false }, methods: {}, children: [] };
      const instance = this.instantiate(node, -1, { noshow: true });
      instance.className = className;
      return instance;
    }
    const node: ControlNode = { id: `rt-${this.nextHandle}`, type, name: className, props: { Visible: false }, methods: {}, children: [] };
    const object = new RuntimeObject(this.nextHandle++, node, this);
    this.handles[object.handle] = object;
    this.notify();
    return object;
  }

  private removeObject(parent: RuntimeObject, name: string): void {
    const child = parent.child(name);
    if (!child) return;
    parent.removeChild(child);
    for (const o of [child, ...child.descendants()]) {
      o.alive = false;
      delete this.handles[o.handle];
    }
    this.notify();
  }

  removeProperty(obj: number, name: string): boolean {
    const target = this.handles[obj];
    if (!target) return false;
    return target.removeProperty(name);
  }

  /**
   * `ADDPROPERTY(o, cName, uValue)` and `o.AddProperty(cName, uValue)`, which are the same thing.
   *
   * The name may carry subscripts - `AddProperty("aPoly[1,1]")` is how a sample gives a Line the
   * array its PolyPoints points at - and then the property is an array of that shape with the
   * value in every element, `.F.` when none was given. Measured: a zero in a subscript raises
   * 31, and the answer is .T. whether or not the object already had the property.
   */
  addProperty(obj: number, name: string, value: VmValue): boolean {
    if (obj === SCREEN_HANDLE) {
      // an array property on the screen is kept as its first value, which is all anything
      // reads of one there
      const bare = (arraySubscripts(name)?.name ?? name).toUpperCase();
      this.screenValues.set(bare, value ?? false);
      return true;
    }
    const target = this.handles[obj];
    if (!target) return false;
    const sized = arraySubscripts(name);
    if (sized) {
      if (sized.rows < 1 || (sized.twoDimensional && sized.cols < 1)) throw new HostError(31, 'Invalid subscript reference.');
      target.declareArray(sized.name, sized.rows, sized.cols, value);
      return true;
    }
    const object = handleOf(value);
    target.addProperty(name, object === undefined ? vmToProp(value) : null);
    if (object !== undefined) target.setObjectValue(name, object);
    else if (isDate(value) || isDateTime(value)) target.setMomentValue(name, value);
    return true;
  }
}
