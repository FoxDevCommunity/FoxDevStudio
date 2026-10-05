/**
 * Converts a Visual FoxPro form (`.scx`) or class library (`.vcx`) table into FoxDev documents.
 *
 * Both file types are DBF tables whose rows describe one object each: `OBJNAME` is the object's
 * VFP name, `PARENT` the name (or dotted path) of the object that contains it, `BASECLASS` the
 * VFP base class the object ultimately derives from, `CLASS`/`CLASSLOC` the class it was actually
 * stamped from, and the `PROPERTIES`/`METHODS` memos hold the designer's `Name = Value` lines and
 * the `PROCEDURE ... ENDPROC` blocks. Rows arrive already parsed: this module never sees bytes.
 *
 * Nothing is thrown for bad input. Anything that cannot be represented is reported as a warning
 * and anything that is merely unknown is preserved verbatim in `meta.vfp.reserved`.
 */

import { nanoid } from 'nanoid';
import { parseColor } from '../form/color';
import { dedupeName } from '../form/naming';
import { FORM_SCHEMA_ID, FORM_VERSION } from '../form/schema';
import type { ControlNode, ControlType, FormCursor, FormDocument, FormNode, FormRelation, PropValue, VfpFormset } from '../form/schema';
import { FORM_DESCRIPTOR, getDescriptor, getPropertyMeta } from '../registry';
import type { ObjectDescriptor } from '../registry';
import { binary, hasField, text } from './dbfTypes';
import { oleClassOf } from './oleControl';
import type { DbfTableData } from './dbfTypes';

/**
 * What a warning is about, so a caller importing hundreds of forms can group them.
 *
 * A real Visual FoxPro project produces the same handful of warnings over and over - every form
 * carries a data environment, and most carry a non-visual helper - and a list of them one per
 * line buries the few that are about this particular file.
 */
export type VfpWarningKind =
  /** The file holds no form at all - a .vcx, or a table that is not a designer's. */
  | 'noForm'
  | 'dataEnvironment'
  | 'unsupportedBaseClass'
  | 'classNotFound'
  | 'parentNotFound'
  | 'renamed'
  | 'formset'
  | 'other';

export interface VfpImportWarning {
  /** The VFP object the warning is about: an object path, a class name or the file stem. */
  object: string;
  message: string;
  kind: VfpWarningKind;
  /** The one thing that varies within a kind: a base class, or a class library. */
  detail?: string;
}

export interface ImportedForm {
  doc: FormDocument;
  warnings: VfpImportWarning[];
}

/** One `Name = Value` line of a PROPERTIES memo. `name` may be dotted (`Label1.Caption`). */
export interface VfpPropertyEntry {
  name: string;
  value: PropValue;
  /** Source text of the value, exactly as VFP wrote it (used for `meta.vfp.reserved`). */
  raw: string;
  /** The source is an expression to work out when the form loads, not a value written down. */
  expression?: boolean;
}

/** VFP base class -> FoxDev control type. Everything absent here has no designer equivalent. */
const BASE_CLASS_TYPES: Record<string, ControlType | 'Form'> = {
  form: 'Form',
  label: 'Label',
  textbox: 'TextBox',
  editbox: 'EditBox',
  commandbutton: 'CommandButton',
  commandgroup: 'CommandGroup',
  checkbox: 'CheckBox',
  optiongroup: 'OptionGroup',
  optionbutton: 'OptionButton',
  combobox: 'ComboBox',
  listbox: 'ListBox',
  spinner: 'Spinner',
  shape: 'Shape',
  line: 'Line',
  image: 'Image',
  container: 'Container',
  pageframe: 'PageFrame',
  page: 'Page',
  grid: 'Grid',
  column: 'Column',
  header: 'Header',
  timer: 'Timer',
  custom: 'Custom',
  session: 'Session',
  hyperlink: 'Hyperlink',
  collection: 'Collection',
  toolbar: 'Toolbar',
  separator: 'Separator',
  olecontrol: 'OleControl',
  oleboundcontrol: 'OleBoundControl',
  // VFP's `control` is the base every container derives from; nothing else distinguishes it
  control: 'Container',
};

/** Data-side rows: dropped without importing, with a single warning per form. */
const DATA_BASE_CLASSES = new Set(['dataenvironment', 'cursor', 'relation']);

/** Maps a VFP `BASECLASS` string to a control type. Case-insensitive; `null` when unsupported. */
export function baseClassToControlType(baseClass: string): ControlType | 'Form' | null {
  return BASE_CLASS_TYPES[baseClass.trim().toLowerCase()] ?? null;
}

// ---------------------------------------------------------------- literals

const TRUE_LITERALS = new Set(['.t.', '.y.', 'true']);
const FALSE_LITERALS = new Set(['.f.', '.n.', 'false']);

/**
 * Parses one VFP value literal. VFP has no string escapes, so a delimited string is simply its
 * contents; anything unrecognised (dates, expressions, object references) is kept as its text.
 */
/**
 * What a property line holds: a value written down, or an expression to work out.
 *
 * Everything in a `.scx` property memo is a Visual FoxPro expression - `Caption = "Find"` is the
 * expression `"Find"` - and the ones that are not simply literals are worked out when the form
 * loads. `Picture = (HOME() + "graphics\\edit.bmp")` is how a form finds a bitmap wherever
 * Visual FoxPro is installed, and `ClassLibrary = (IIF(VERSION(2)=0,"",HOME()+"FFC\\")+"_table.vcx")`
 * is how it finds a class library. Kept as text they are neither a caption nor a path.
 */
export function readVfpValue(source: string): { value: PropValue; expression: boolean } {
  const t = source.trim();
  const value = vfpLiteral(t);
  // the reader hands back the source itself when it is not a literal it knows
  return { value, expression: typeof value === 'string' && value === t && t !== '' && !isQuoted(t) };
}

/** True when the source is a quoted string, whose value is itself minus the quotes. */
function isQuoted(t: string): boolean {
  const closer = t.startsWith('[') ? ']' : t.startsWith('"') ? '"' : t.startsWith("'") ? "'" : '';
  return closer !== '' && t.length >= 2 && t.endsWith(closer);
}

export function vfpLiteral(source: string): PropValue {
  const t = source.trim();
  if (t === '') return '';
  const closer = t.startsWith('[') ? ']' : t.startsWith('"') ? '"' : t.startsWith("'") ? "'" : '';
  if (closer !== '' && t.length >= 2 && t.endsWith(closer)) return t.slice(1, -1);
  const lower = t.toLowerCase();
  if (lower === '.null.' || lower === 'null') return null;
  if (TRUE_LITERALS.has(lower)) return true;
  if (FALSE_LITERALS.has(lower)) return false;
  if (/^[-+]?(\d+\.?\d*|\.\d+)([eE][-+]?\d+)?$/.test(t)) return Number(t);
  if (/^rgb\s*\(/i.test(t)) {
    const color = parseColor(t);
    if (color !== null) return color;
  }
  return t;
}

/** True when `raw` opens a string delimiter that the line never closes. */
function isUnterminatedString(raw: string): boolean {
  const t = raw.trimStart();
  const closer = t.startsWith('[') ? ']' : t.startsWith('"') ? '"' : t.startsWith("'") ? "'" : '';
  if (closer === '') return false;
  return t.indexOf(closer, 1) < 0;
}

const PROPERTY_LINE = /^\s*([A-Za-z_][A-Za-z0-9_]*(?:\.[A-Za-z_][A-Za-z0-9_]*)*)\s*=\s*(.*)$/;

/**
 * Splits a PROPERTIES memo into entries, in source order. Entries are one per line; a line with no
 * `Name =` prefix, or any line while the previous value is an unterminated string, continues the
 * value above it.
 */
export function parseVfpProperties(memo: string): VfpPropertyEntry[] {
  const entries: { name: string; raw: string }[] = [];
  for (const line of splitLines(memo)) {
    const last = entries[entries.length - 1];
    if (last && isUnterminatedString(last.raw)) {
      last.raw += '\n' + line;
      continue;
    }
    if (line.trim() === '') continue;
    const m = PROPERTY_LINE.exec(line);
    if (m) entries.push({ name: m[1]!, raw: m[2]!.trim() });
    else if (last) last.raw += '\n' + line.trim();
  }
  return entries.map((e) => {
    const read = readVfpValue(e.raw);
    return { name: e.name, raw: e.raw, value: read.value, expression: read.expression };
  });
}

// ---------------------------------------------------------------- methods

const PROCEDURE_LINE = /^\s*PROCEDURE\s+([\w.]+)/i;
const ENDPROC_LINE = /^\s*ENDPROC\s*$/i;

/**
 * Splits a METHODS memo into `{ eventName: sourceText }`. Blank-bodied procedures (VFP writes those
 * for events that only exist in the designer) are dropped.
 */
export function parseVfpMethods(memo: string): Record<string, string> {
  // Every definition, in the order written. A class chain's memos arrive joined, the class the
  // chain starts from first, so one name written twice is an override: the last is the one the
  // object runs, and the ones before it are what DODEFAULT() reaches. Those are kept as
  // `Name#1` for the nearest ancestor, `Name#2` for the one above it, and so on.
  const defined: { name: string; source: string }[] = [];
  let name: string | null = null;
  let body: string[] = [];
  const flush = () => {
    if (name === null) return;
    const source = trimBlankEdges(body).join('\n');
    if (source.trim() !== '') defined.push({ name, source });
    name = null;
    body = [];
  };
  for (const line of splitLines(memo)) {
    const start = PROCEDURE_LINE.exec(line);
    if (start) {
      flush();
      name = start[1]!;
    } else if (ENDPROC_LINE.test(line)) {
      flush();
    } else if (name !== null) {
      body.push(line);
    }
  }
  flush();

  const levels = new Map<string, { name: string; source: string }[]>();
  for (const d of defined) {
    const key = d.name.toLowerCase();
    const list = levels.get(key);
    if (list) list.push(d);
    else levels.set(key, [d]);
  }
  const out: Record<string, string> = {};
  for (const list of levels.values()) {
    const latest = list[list.length - 1]!;
    out[latest.name] = latest.source;
    for (let depth = 1; depth < list.length; depth++) out[`${latest.name}#${depth}`] = list[list.length - 1 - depth]!.source;
  }
  return out;
}

/** An ancestor's copy of an overridden method, `Init#1`, as the event it is a copy of. */
export function ancestorEvent(name: string): { event: string; depth: number } {
  const hash = name.lastIndexOf('#');
  if (hash < 0) return { event: name, depth: 0 };
  const depth = Number(name.slice(hash + 1));
  return Number.isInteger(depth) && depth > 0 ? { event: name.slice(0, hash), depth } : { event: name, depth: 0 };
}

/**
 * Which of a memo's procedures are the object's own, and which belong to something inside it.
 *
 * Visual FoxPro writes a member's override in the *container's* METHODS memo, under a dotted
 * name: a list box's InteractiveChange on the form that holds it reads
 * `PROCEDURE lstSource.InteractiveChange`. Read as one name it became a method called
 * lstSource on the form, and the list box had none.
 */
export function splitMemberMethods(methods: Record<string, string>): {
  own: Record<string, string>;
  members: { path: string; event: string; source: string }[];
} {
  const own: Record<string, string> = {};
  const members: { path: string; event: string; source: string }[] = [];
  for (const [name, source] of Object.entries(methods)) {
    const cut = name.lastIndexOf('.');
    if (cut <= 0) {
      own[name] = source;
      continue;
    }
    members.push({ path: name.slice(0, cut), event: name.slice(cut + 1), source });
  }
  return { own, members };
}

function splitLines(memo: string): string[] {
  return memo.split(/\r\n|\r|\n/);
}

function trimBlankEdges(lines: string[]): string[] {
  let start = 0;
  let end = lines.length;
  while (start < end && lines[start]!.trim() === '') start++;
  while (end > start && lines[end - 1]!.trim() === '') end--;
  return lines.slice(start, end);
}

// ---------------------------------------------------------------- rows

interface Row {
  objName: string;
  /** The `PARENT` memo: the parent's name, or a dotted path for nested containers. */
  parentRef: string;
  className: string;
  classLoc: string;
  baseClass: string;
  properties: string;
  methods: string;
  /**
   * The `RESERVED3` memo: one line per member the object added for itself, as `name description`,
   * with a leading `*` marking a method. It is the only place a custom property with no value -
   * `lCalledBySolution` - is written down at all.
   */
  custom: string;
  /** The `OLE` memo: an ActiveX control's persisted state, which is where its class id is. */
  ole: Uint8Array;
  /** The `OLE2` memo: `OLEObject = <path to the OCX>`. */
  ole2: string;
  /** The `RESERVED7` memo: what the Class Info dialog calls the class's description. */
  description: string;
  /**
   * The header file whose `#DEFINE`s are in scope for this row's code, as the file names it.
   *
   * Visual FoxPro writes it in the `RESERVED8` memo: a `.scx` carries one for the whole file, on
   * the `COMMENT`/`Screen` record the designer writes first, and a `.vcx` carries one per class,
   * on the class's own definition row. Measured: the constants reach every method the file holds,
   * contained controls included, and reach nothing outside it - a method inherited from a class
   * library is compiled with that library's header and never with the form's, and a form's own
   * method never sees the header of a library it was built from.
   */
  include: string;
  /**
   * Where each method of `methods` came from, by upper-cased method name, when the memo was
   * merged out of several files. Only names that differ from `include` are listed.
   */
  methodIncludes?: Record<string, string>;
  /** Set when the row names a class that no library in scope defines. */
  classMissing?: boolean;
  /**
   * Set when the row is not in the file at all: it came from the class this object was stamped
   * from. A row of the file that lands on the same name is that member being redefined, not a
   * second member beside it - two members of one container cannot share a name.
   */
  fromClass?: boolean;
}

/** A class definition: a row with no parent, followed by every row that belongs to it. */
interface Group {
  root: Row;
  members: Row[];
}

function readRows(table: DbfTableData): Row[] {
  const out: Row[] = [];
  // A `.scx` names its header file once for the whole file, on the `COMMENT` record the designer
  // writes before anything else; a `.vcx` names one per class, on the class's own row. Both land
  // in `RESERVED8`, so the file's answer stands until a definition gives its own, and every row
  // inside a definition takes whatever that definition settled on.
  let file = '';
  let current = '';
  for (const record of table.records) {
    if (record.deleted) continue;
    const platform = text(table, record, 'PLATFORM');
    const reserved8 = text(table, record, 'RESERVED8').trim();
    if (platform !== '' && platform.toUpperCase() !== 'WINDOWS') {
      if (reserved8 !== '' && file === '') file = reserved8;
      continue;
    }
    const objName = text(table, record, 'OBJNAME');
    if (objName === '') continue;
    const parentRef = text(table, record, 'PARENT');
    if (parentRef === '') current = reserved8 === '' ? file : reserved8;
    out.push({
      include: current,
      objName,
      parentRef,
      className: text(table, record, 'CLASS'),
      classLoc: text(table, record, 'CLASSLOC'),
      baseClass: text(table, record, 'BASECLASS'),
      properties: text(table, record, 'PROPERTIES'),
      methods: text(table, record, 'METHODS'),
      custom: text(table, record, 'RESERVED3'),
      ole: binary(table, record, 'OLE'),
      ole2: text(table, record, 'OLE2'),
      description: text(table, record, 'RESERVED7'),
    });
  }
  return out;
}

/** Every row with an empty `PARENT` opens a definition; the rows after it are its members. */
function splitGroups(rows: Row[]): Group[] {
  const groups: Group[] = [];
  for (const row of rows) {
    if (row.parentRef === '') groups.push({ root: row, members: [] });
    else groups[groups.length - 1]?.members.push(row);
  }
  return groups;
}

function isFormTable(table: DbfTableData): boolean {
  return hasField(table, 'OBJNAME') && hasField(table, 'BASECLASS') && hasField(table, 'PROPERTIES');
}

function emptyForm(name: string): FormDocument {
  return { $schema: FORM_SCHEMA_ID, version: FORM_VERSION, form: { name, props: {}, methods: {}, children: [] } };
}

// ---------------------------------------------------------------- subclasses

/**
 * A `.vcx` whose classes a form may be built from. `name` is how a `CLASSLOC` refers to it
 * (normally the file name), and is matched without its directory or extension.
 */
/**
 * Bumped whenever the importer learns to keep something it used to lose.
 *
 * A converted document records the version that made it, so the IDE can convert it again when
 * this moves on. Without that, a fix to the importer only reaches projects imported afterwards,
 * and a project imported last week keeps failing in exactly the way that was just fixed.
 */
export const IMPORTER_VERSION = 8;

export interface VfpClassLibrary {
  name: string;
  table: DbfTableData;
  /**
   * Where the library was actually found, when the caller knows. A class names its header file
   * by a name of its own (`wincrypt.h`), and Visual FoxPro looks for that beside the file that
   * named it - so a method taken from this library carries its header as a path from here, and
   * one taken from the form carries the plain name the form wrote.
   */
  path?: string;
}

/** Every distinct `CLASSLOC` a table refers to, so the caller knows which libraries to load. */
export function classLibrariesReferenced(table: DbfTableData): string[] {
  if (!isFormTable(table)) return [];
  const seen = new Map<string, string>();
  for (const row of readRows(table)) {
    if (row.classLoc === '' || !isSubclassed(row)) continue;
    const key = row.classLoc.toLowerCase();
    if (!seen.has(key)) seen.set(key, row.classLoc);
  }
  return [...seen.values()];
}

/** True when the row was stamped from a class rather than straight from a VFP base class. */
function isSubclassed(row: Row): boolean {
  return row.className !== '' && row.className.toLowerCase() !== row.baseClass.toLowerCase();
}

/** Class definitions in scope, by lower-cased class name. */
type ClassIndex = Map<string, Group>;

function buildClassIndex(libraries: readonly VfpClassLibrary[]): ClassIndex {
  const index: ClassIndex = new Map();
  for (const library of libraries) {
    if (!isFormTable(library.table)) continue;
    const from = library.path ?? library.name;
    for (const group of splitGroups(readRows(library.table).map((row) => ({ ...row, include: headerFrom(from, row.include) })))) {
      const key = group.root.objName.toLowerCase();
      if (key !== '' && !index.has(key)) index.set(key, group);
    }
  }
  return index;
}

/**
 * A header file named inside `file`, as a reference the form's own folder can be asked for.
 *
 * Visual FoxPro looks for a header beside the file that named it, so a class library two folders
 * away brings its header from there - measured, and a relative folder on the name counts from
 * there as well. Only a full path stands on its own.
 */
function headerFrom(file: string, include: string): string {
  if (include === '' || /^[a-zA-Z]:|^[\\/]/.test(include)) return include;
  const dir = file.replace(/[^\\/]*$/, '');
  return dir === '' ? include : dir + include;
}

/** A class and everything it inherits, flattened into one root row plus its contained rows. */
interface ResolvedClass {
  /** Property and method memos from the furthest ancestor down to the class itself. */
  chain: Row[];
  /** Contained rows, inherited ones first, all parented under `rootName`. */
  members: Row[];
  rootName: string;
}

function resolveClass(className: string, index: ClassIndex, seen: Set<string>): ResolvedClass | undefined {
  const key = className.toLowerCase();
  if (seen.has(key)) return undefined; // a class that inherits from itself
  const group = index.get(key);
  if (!group) return undefined;
  seen.add(key);

  const rootName = group.root.objName;
  const parent = isSubclassed(group.root) ? resolveClass(group.root.className, index, seen) : undefined;
  const inherited = parent ? reparent(parent.members, parent.rootName, rootName) : [];
  return {
    chain: [...(parent?.chain ?? []), group.root],
    members: [...inherited, ...expandSubclasses(group.members, index)],
    rootName,
  };
}

/**
 * Replaces every subclassed row with the class it was stamped from: the class's property and
 * method memos underneath the instance's own, and the class's members re-parented under it.
 *
 * This is what VFP does when it instantiates the form. A `.scx` stores only one row for a control
 * dropped from a class library, with whatever the instance overrides written as dotted names in
 * that row's PROPERTIES memo, so without this the control arrives empty.
 */
function expandSubclasses(members: Row[], index: ClassIndex): Row[] {
  const out: Row[] = [];
  for (const row of members) {
    if (!isSubclassed(row)) {
      out.push(row);
      continue;
    }
    const resolved = index.size > 0 ? resolveClass(row.className, index, new Set()) : undefined;
    if (!resolved) {
      // reported where the row is placed, which is where its path in the form is known
      out.push({ ...row, classMissing: true });
      continue;
    }
    const [instance, classMembers] = applyClass(row, resolved);
    out.push(instance, ...classMembers.map((m) => ({ ...m, fromClass: true })));
  }
  return out;
}

/**
 * Merges a class into one instance row. Memos are concatenated with the class first, because a
 * later `Name = Value` line and a later `PROCEDURE` block both win. Dotted names in the
 * instance's memo (`Label1.Caption = "x"`) are moved onto the member they address, which is the
 * only place VFP records an override of something the class owns.
 */
function applyClass(row: Row, resolved: ResolvedClass): [Row, Row[]] {
  const members = reparent(resolved.members, resolved.rootName, row.objName).map((m) => ({ ...m }));
  const pathOf = (m: Row) => [...m.parentRef.split('.').slice(1), m.objName].join('.').toLowerCase();

  // `Label1.Caption` addresses a member, which has a row. `pf1.Page4.Name` addresses a page of
  // an inherited pageframe, and a page has no row of its own - the pageframe makes it. So the
  // longest member path that fits the front of the name takes the property, and what is left of
  // the name goes down with it, for that member to place when it builds what it contains.
  const addressed = (name: string): { member: Row; rest: string } | undefined => {
    const segments = name.split('.');
    for (let cut = segments.length - 1; cut >= 1; cut--) {
      const prefix = segments.slice(0, cut).join('.').toLowerCase();
      const member = members.find((m) => pathOf(m) === prefix);
      if (member) return { member, rest: segments.slice(cut).join('.') };
    }
    return undefined;
  };

  // An ancestor between this class and the one that made a member writes its overrides the same
  // way - `frmInstall` sets `pgfSteps.Page2.Name` on a pageframe `frmWizard` made - so every
  // memo in the chain is routed, oldest first, for a later line to win as it does in one memo.
  const routed = (properties: string): string => {
    const kept: string[] = [];
    for (const entry of parseVfpProperties(properties)) {
      const target = entry.name.includes('.') ? addressed(entry.name) : undefined;
      if (target) target.member.properties = `${target.member.properties}\n${target.rest} = ${entry.raw}`;
      else kept.push(`${entry.name} = ${entry.raw}`);
    }
    return kept.join('\n');
  };
  const inherited = resolved.chain.map((r) => routed(withoutName(r.properties)));
  const own = routed(row.properties);

  // The merged memo holds procedures out of several files, and each of them is compiled with the
  // header its own file named - so which file each one came from has to be written down before
  // the memos become one string.
  // An ancestor's copy of an overridden method - `INIT#1` - was written in its own class's
  // file, so it carries that file's header rather than the one the override came with.
  const writers = new Map<string, string[]>();
  for (const from of [...resolved.chain, row]) {
    for (const name of Object.keys(parseVfpMethods(from.methods))) {
      const key = name.toUpperCase();
      writers.set(key, [...(writers.get(key) ?? []), from.include]);
    }
  }
  const methodIncludes: Record<string, string> = {};
  for (const [name, includes] of writers) {
    methodIncludes[name] = includes[includes.length - 1]!;
    for (let depth = 1; depth < includes.length; depth++) methodIncludes[`${name}#${depth}`] = includes[includes.length - 1 - depth]!;
  }

  const instance: Row = {
    ...row,
    methodIncludes,
    // a class's own Name is the class's, never the instance's: that comes from OBJNAME
    properties: [...inherited, own].filter((p) => p.trim() !== '').join('\n'),
    methods: [...resolved.chain.map((r) => r.methods), row.methods].filter((m) => m.trim() !== '').join('\n'),
    custom: [...resolved.chain.map((r) => r.custom), row.custom].filter((c) => c.trim() !== '').join('\n'),
  };
  return [instance, members];
}

function withoutName(properties: string): string {
  return parseVfpProperties(properties)
    .filter((e) => e.name.toLowerCase() !== 'name')
    .map((e) => `${e.name} = ${e.raw}`)
    .join('\n');
}

/** Rewrites the leading segment of each `PARENT` so the rows hang off `to` instead of `from`. */
function reparent(members: Row[], from: string, to: string): Row[] {
  if (from.toLowerCase() === to.toLowerCase()) return members;
  return members.map((row) => {
    const segments = row.parentRef.split('.');
    if ((segments[0] ?? '').toLowerCase() === from.toLowerCase()) segments[0] = to;
    return { ...row, parentRef: segments.join('.') };
  });
}

function warnMissingClass(row: Row, object: string, warnings: VfpImportWarning[]): void {
  if (!row.classMissing) return;
  const from = row.classLoc === '' ? '' : ` from "${row.classLoc}"`;
  warnings.push({
    object,
    kind: 'classNotFound',
    detail: row.classLoc === '' ? row.className : row.classLoc,
    message: `"${row.objName}" is an instance of class "${row.className}"${from}, which was not found; it was imported as a plain ${row.baseClass} and members defined in the class are missing.`,
  });
}

// ---------------------------------------------------------------- public entry points

/**
 * Converts a parsed `.scx` table into a form document. `name` is the file stem, used when the form
 * row carries no `Name` property and no `OBJNAME`.
 */
export function importFormTable(table: DbfTableData, name: string, libraries: readonly VfpClassLibrary[] = []): ImportedForm {
  const forms = importFormFile(table, name, libraries);
  return forms[0]?.imported ?? { doc: emptyForm(name), warnings: [{ object: name, kind: 'other', message: 'No form definition was found in this file.' }] };
}

/**
 * Every form a `.scx` defines. Usually one; a formset defines several, and each becomes its own
 * document, because a formset has no designer equivalent and its forms are separate windows.
 */
export function importFormFile(
  table: DbfTableData,
  name: string,
  libraries: readonly VfpClassLibrary[] = [],
): { formName: string; imported: ImportedForm }[] {
  if (!isFormTable(table)) {
    const warning: VfpImportWarning = { object: name, kind: 'noForm', message: 'This table is not a Visual FoxPro form or class library.' };
    return [{ formName: name, imported: { doc: emptyForm(name), warnings: [warning] } }];
  }
  const groups = splitGroups(readRows(table));
  const formGroup = groups.find((g) => {
    const base = g.root.baseClass.toLowerCase();
    return base === 'form' || base === 'formset';
  });
  if (!formGroup) {
    // a .vcx is exactly this: a file full of classes and no form at all
    const warning: VfpImportWarning = { object: name, kind: 'noForm', message: 'No form definition was found in this file.' };
    return [{ formName: name, imported: { doc: emptyForm(name), warnings: [warning] } }];
  }

  // whatever else is in the file belongs to every form it defines: the data environment is one
  const shared: VfpImportWarning[] = [];
  const cursors: FormCursor[] = [];
  const relations: FormRelation[] = [];
  for (const other of groups) {
    if (other === formGroup) continue;
    if (DATA_BASE_CLASSES.has(other.root.baseClass.toLowerCase())) {
      cursors.push(...readCursors(other));
      relations.push(...readRelations(other));
    } else
      shared.push({
        object: other.root.objName,
        kind: 'unsupportedBaseClass',
        detail: other.root.baseClass || 'unknown',
        message: `Top-level "${other.root.objName}" (${other.root.baseClass || 'unknown'}) is not part of the form and was not imported.`,
      });
  }

  const index = buildClassIndex(libraries);
  const split: VfpImportWarning[] = [];
  const { parts, formset } = formsIn(formGroup, split);
  // the warnings about the file as a whole belong on the first form, not on each of them
  const definitions = parts.map((part, i) => buildDefinition(part, name, index, i === 0 ? [...split, ...shared] : []));
  const memberNames = definitions.map((b) => b.doc.form.name);
  return definitions.map((built) => {
    // every member form carries the whole formset, so any one of them can build it again
    if (formset) built.formset = readFormset(formset, memberNames, built.expressions, built.reserved);
    const imported = finish(built);
    // every form of a formset shares the file's data environment, as it does in VFP
    if (cursors.length > 0) imported.doc.data = cursors.map((c) => ({ ...c }));
    if (relations.length > 0) imported.doc.relations = relations.map((r) => ({ ...r }));
    return { formName: imported.doc.form.name, imported };
  });
}

/**
 * The tables a form's data environment opens.
 *
 * Each is a `cursor` row carrying an `Alias` and a `CursorSource`, which is the file. That is
 * the whole of what a form needs to have its tables open before Load runs, and it is what a great
 * deal of form code assumes without ever saying so.
 */
function readCursors(group: Group): FormCursor[] {
  const out: FormCursor[] = [];
  for (const row of [group.root, ...group.members]) {
    if (row.baseClass.toLowerCase() !== 'cursor') continue;

    const entries = parseVfpProperties(row.properties);
    const value = (name: string) => entries.find((e) => e.name.toLowerCase() === name)?.value;
    const source = String(value('cursorsource') ?? '').trim();
    if (source === '') continue;
    const alias = String(value('alias') ?? '').trim() || source.replace(/^.*[\\/]/, '').replace(/\.[^.]+$/, '');
    const order = String(value('order') ?? '').trim();
    const database = String(value('database') ?? '').trim();
    const cursor: FormCursor = { alias, source };
    if (database !== '') cursor.database = database;
    if (order !== '') cursor.order = order;
    if (value('exclusive') === true) cursor.exclusive = true;
    out.push(cursor);
  }
  return out;
}

/**
 * The links a form's data environment sets up between its tables.
 *
 * A `relation` row names the two aliases, the expression matched against the child's index and
 * the tag that index is. This is what makes a grid of order lines follow the order the user is
 * on, and a form built on it shows nothing without them.
 */
function readRelations(group: Group): FormRelation[] {
  const out: FormRelation[] = [];
  for (const row of [group.root, ...group.members]) {
    if (row.baseClass.toLowerCase() !== 'relation') continue;
    const entries = parseVfpProperties(row.properties);
    const value = (name: string) => String(entries.find((e) => e.name.toLowerCase() === name)?.value ?? '').trim();
    const parent = value('parentalias');
    const child = value('childalias');
    const expression = value('relationalexpr');
    // a relation missing any of its three parts links nothing, and VFP's designer cannot make one
    if (parent === '' || child === '' || expression === '') continue;
    const relation: FormRelation = { parent, child, expression };
    const order = value('childorder');
    if (order !== '') relation.childOrder = order;
    if (entries.find((e) => e.name.toLowerCase() === 'onetomany')?.value === true) relation.oneToMany = true;
    out.push(relation);
  }
  return out;
}

/** The members one object of a class declares in the library's own records. */
export interface VfpClassMembers {
  props: string[];
  methods: string[];
}

/**
 * What makes a class definition a class rather than a form: its lineage, what it is for, and
 * which of the values it carries it writes itself.
 *
 * The document a class library is written as keeps every value an instance would have, inherited
 * ones included, so it can be built without reading the parent library. `own` is what tells the
 * two apart afterwards, and it can only be worked out here, from the rows before the parent class
 * was folded into them.
 */
export interface VfpClassFacts {
  baseClass: string;
  parentClass?: string;
  parentLibrary?: string;
  description?: string;
  /** Dotted path of the object in the document -> what this class's own rows write on it. */
  own: Record<string, VfpClassMembers>;
}

/** A `.vcx` holds several class definitions; each becomes its own form-shaped document. */
export function importClassLibrary(
  table: DbfTableData,
  name: string,
  libraries: readonly VfpClassLibrary[] = [],
): { className: string; imported: ImportedForm; facts: VfpClassFacts }[] {
  if (!isFormTable(table)) return [];
  // a class in this library may inherit from another one in the same file
  const index = buildClassIndex([{ name, table }, ...libraries]);
  return splitGroups(readRows(table))
    .filter((g) => !DATA_BASE_CLASSES.has(g.root.baseClass.toLowerCase()))
    .map((group) => {
      // a formset class is the container itself, so its forms are kept beside its control tree
      const own = withoutMemberForms(group);
      const built = buildDefinition(own, name, index);
      built.memberForms = memberFormsOf(group).map((root) => buildDefinition({ root, members: descendantsOf(root, group.members) }, name, index).doc.form);
      return { className: group.root.objName, facts: classFacts(own, built, name), imported: finish(built) };
    });
}

/** The class's own rows, with everything that belongs to a member form of it taken out. */
function withoutMemberForms(group: Group): Group {
  const forms = memberFormsOf(group);
  if (forms.length === 0) return group;
  const theirs = new Set(forms.flatMap((f) => [f, ...descendantsOf(f, group.members)]));
  return { root: group.root, members: group.members.filter((r) => !theirs.has(r)) };
}

/**
 * The lineage of one class, and the members its own rows write.
 *
 * The rows are read before the parent class was folded into them, which is the only moment the
 * two are still apart: a class writes a line for what it sets and nothing at all for what it
 * takes, and an override of something further down arrives as a dotted name in the class's own
 * memo (`lstSource.Height = 132`), which is the only place VFP records one.
 */
function classFacts(group: Group, built: Built, library: string): VfpClassFacts {
  const own: Record<string, VfpClassMembers> = {};
  const at = (vfpPath: string): VfpClassMembers => {
    const key = docPathOf(built, vfpPath);
    return (own[key] ??= { props: [], methods: [] });
  };

  for (const row of [group.root, ...group.members]) {
    const path = row === group.root ? row.objName : `${row.parentRef}.${row.objName}`;
    for (const entry of parseVfpProperties(row.properties)) {
      const dot = entry.name.lastIndexOf('.');
      const property = dot < 0 ? entry.name : entry.name.slice(dot + 1);
      if (!isEditableProperty(property)) continue;
      at(dot < 0 ? path : `${path}.${entry.name.slice(0, dot)}`).props.push(property);
    }
    const methods = at(path).methods;
    for (const method of Object.keys(parseVfpMethods(row.methods))) methods.push(method);
  }
  for (const [path, members] of Object.entries(own)) {
    if (members.props.length === 0 && members.methods.length === 0) delete own[path];
  }

  const root = group.root;
  const parent = isSubclassed(root) ? root.className : '';
  const parentLibrary = parent === '' || sameLibrary(root.classLoc, library) ? '' : root.classLoc;
  return {
    baseClass: root.baseClass,
    ...(parent === '' ? {} : { parentClass: parent }),
    ...(parentLibrary === '' ? {} : { parentLibrary }),
    ...(root.description === '' ? {} : { description: root.description }),
    own,
  };
}

/** Resolves a dotted path in the file to the path the object ended up at, as `PARENT` is resolved. */
function docPathOf(built: Built, vfpPath: string): string {
  const key = vfpPath.toLowerCase();
  const full = built.paths.get(key);
  if (full) return full;
  const segments = key.split('.');
  return built.names.get(segments[segments.length - 1] ?? '') ?? vfpPath;
}

/** A property line that is not a value the designer shows: the object's identity, or a note. */
function isEditableProperty(name: string): boolean {
  return name.toLowerCase() !== 'name' && !HOUSEKEEPING.has(name.toLowerCase()) && !name.startsWith('_');
}

/** Whether a `CLASSLOC` names the library being read: a class library refers to itself by file. */
function sameLibrary(classLoc: string, library: string): boolean {
  const stem = (path: string) =>
    path
      .replace(/^.*[\\/]/, '')
      .replace(/\.[^.]+$/, '')
      .toLowerCase();
  return classLoc === '' || stem(classLoc) === stem(library);
}

// ---------------------------------------------------------------- building

interface Built {
  doc: FormDocument;
  warnings: VfpImportWarning[];
  reserved: Record<string, string>;
  /** Values the form works out when it loads, by `objectPath.PropertyName`. */
  expressions: Record<string, string>;
  /** Array properties an object adds for itself, by `objectPath.PropertyName`. */
  arrays: Record<string, [number, number]>;
  dataDropped: boolean;
  /** The formset this definition is a member form of, when the file put one round it. */
  formset?: VfpFormset;
  /** The forms this definition holds, when the definition is itself a formset. */
  memberForms?: FormNode[];
  /** `CLASSLOC` of the definition's own row: the library its class was stamped from. */
  classLib: string;
  /** `BASECLASS` of the definition's own row, kept when it is not a plain form. */
  baseClass: string;
  /** The header file the definition's own file named, which most of its methods are compiled with. */
  include: string;
  /** The header file of every method that is compiled with a different one, by `path.Event`. */
  includes: Record<string, string>;
  /**
   * Where every row ended up: its dotted path in the file, lower-cased, against the dotted path
   * of the object it became. A row's document name is its `Name` property rather than its
   * `OBJNAME`, and a clashing one is renamed, so nothing outside can work these out for itself.
   */
  paths: Map<string, string>;
  /** The same by bare name, because a `PARENT` memo is often one where a path was meant. */
  names: Map<string, string>;
}

/** A row that has been placed (or deliberately dropped) plus the identity its children need. */
interface Placed {
  /** Dotted VFP path of the row, as written in the file (used to resolve `PARENT`). */
  path: string;
  /** Path built from the final, de-duplicated document names (used for `meta.vfp.reserved` keys). */
  docPath: string;
  node: ControlNode | null;
  /** The form itself has no ControlNode; children still attach to it. */
  container: FormNode | ControlNode | null;
  descriptor: ObjectDescriptor | null;
  /** True for a member the container made itself: a Page, a Column, a Header. */
  auto?: boolean;
  /** True for a member that came from the class the object was stamped from. */
  inherited?: boolean;
}

function buildDefinition(part: { root: Row; members: Row[] }, fileStem: string, index: ClassIndex, carried: VfpImportWarning[] = []): Built {
  const warnings: VfpImportWarning[] = [...carried];
  const reserved: Record<string, string> = {};
  const expressions: Record<string, string> = {};
  const arrays: Record<string, [number, number]> = {};
  let dataDropped = false;

  // the definition itself may be stamped from a class, and so may every row inside it
  const { root, members } = expandRoot(part.root, part.members, index);

  const rootType = baseClassToControlType(root.baseClass);
  const rootDescriptor: ObjectDescriptor = rootType && rootType !== 'Form' ? getDescriptor(rootType) : FORM_DESCRIPTOR;
  const rootEntries = parseVfpProperties(root.properties);
  const rootName = nameFromEntries(rootEntries) || root.objName || fileStem;

  const form: FormNode = { name: rootName, props: {}, methods: {}, children: [] };
  const rootDotted = rootEntries.filter((e) => e.name.includes('.'));
  applyProperties(
    rootEntries.filter((e) => !e.name.includes('.')),
    rootDescriptor,
    rootName,
    form.props,
    reserved,
    expressions,
  );
  // Which header file each method is compiled with, by the path `formMethodSources` gives it -
  // the object's dotted path from the form and the event, with the form's own methods under the
  // bare event name. Only the ones that differ from the file's own header are written down.
  const includes: Record<string, string> = {};
  const methodKey = (docPath: string, event: string): string =>
    (docPath === rootName ? event : `${docPath.slice(rootName.length + 1)}.${event}`).toLowerCase();
  const noteIncludes = (row: Row, docPath: string, methods: Record<string, string>, memoName?: (event: string) => string): void => {
    for (const event of Object.keys(methods)) {
      const from = row.methodIncludes?.[(memoName ? memoName(event) : event).toUpperCase()] ?? row.include;
      if (from !== root.include) includes[methodKey(docPath, event)] = from;
    }
  };

  const deferredMethods: { from: string; path: string; event: string; source: string; row: Row }[] = [];
  const rootMethods = splitMemberMethods(parseVfpMethods(root.methods));
  form.methods = normaliseMethods(rootMethods.own, rootDescriptor);
  noteIncludes(root, rootName, form.methods);
  for (const m of rootMethods.members) deferredMethods.push({ from: root.objName, ...m, row: root });
  applyCustomMembers(root.custom, form, rootName, arrays);
  warnMissingClass(root, root.objName || rootName, warnings);

  const byPath = new Map<string, Placed>();
  const byName = new Map<string, Placed>();
  const rootPlaced: Placed = { path: root.objName, docPath: rootName, node: null, container: form, descriptor: rootDescriptor };
  register(byPath, byName, rootPlaced, root.objName);

  // The definition itself makes members when its base class does: a class rooted at a Grid has
  // its Columns, each with a Header, and the rows that follow are parented to those by name.
  // Only rows did this before, so a .vcx class that *is* a grid came out with its headers and
  // text boxes lying loose in the grid - and renamed, because they then shared a container.
  const rootClaimed = new Set<VfpPropertyEntry>();
  if (rootType && rootType !== 'Form') {
    buildAutoChildren(
      { node: form, type: rootType, dotted: rootDotted, path: root.objName, docPath: rootName, reserved, expressions, claimed: rootClaimed },
      (child, childPath, childDoc) => {
        const placed: Placed = { path: childPath, docPath: childDoc, node: child, container: child, descriptor: getDescriptor(child.type), auto: true };
        register(byPath, byName, placed, child.name);
      },
    );
  }
  for (const entry of rootDotted) {
    if (!rootClaimed.has(entry)) reserved[`${rootName}.${entry.name}`] = entry.raw;
  }

  // VFP normally writes parents before children, but resolve to a fixed point so any order works.
  let pending = members;
  for (;;) {
    const deferred: Row[] = [];
    for (const row of pending) {
      const parent = lookupParent(byPath, byName, row.parentRef);
      if (!parent) {
        deferred.push(row);
        continue;
      }
      placeRow(row, parent);
    }
    if (deferred.length === pending.length) {
      for (const row of deferred) {
        warnings.push({
          object: row.objName,
          kind: 'parentNotFound',
          detail: row.parentRef,
          message: `Parent "${row.parentRef}" was not found; imported at the top level of the form.`,
        });
        placeRow(row, rootPlaced);
      }
      break;
    }
    pending = deferred;
    if (pending.length === 0) break;
  }

  // now that every row is placed, a method written under a dotted name can find what it is on
  for (const { from, path, event, source, row } of deferredMethods) {
    const target = lookupParent(byPath, byName, `${from}.${path}`) ?? lookupParent(byPath, byName, path);
    if (!target?.node || !target.descriptor) {
      warnings.push({
        object: `${from}.${path}`,
        kind: 'other',
        message: `"${path}.${event}" is written on "${from}", which has no "${path}" in it; the code was not imported.`,
      });
      continue;
    }
    const written = normaliseMethods({ [event]: source }, target.descriptor);
    Object.assign(target.node.methods, written);
    noteIncludes(row, target.docPath, written, () => `${path}.${event}`);
  }

  return {
    doc: { $schema: FORM_SCHEMA_ID, version: FORM_VERSION, form },
    warnings,
    reserved,
    expressions,
    arrays,
    dataDropped,
    classLib: root.classLoc,
    baseClass: root.baseClass,
    include: root.include,
    includes,
    paths: new Map([...byPath].map(([key, placed]) => [key, placed.docPath])),
    names: new Map([...byName].map(([key, placed]) => [key, placed.docPath])),
  };

  function placeRow(row: Row, parent: Placed): void {
    const path = `${parent.path}.${row.objName}`;
    const drop = (docPath: string) => register(byPath, byName, { path, docPath, node: null, container: null, descriptor: null }, row.objName);

    // Anything under a dropped object goes with it, silently: it was already reported once.
    if (!parent.container) return drop(`${parent.docPath}.${row.objName}`);
    if (DATA_BASE_CLASSES.has(row.baseClass.toLowerCase())) {
      dataDropped = true;
      return drop(`${parent.docPath}.${row.objName}`);
    }

    const type = baseClassToControlType(row.baseClass);
    if (type === null || type === 'Form') {
      warnings.push({
        object: path,
        kind: 'unsupportedBaseClass',
        detail: row.baseClass || 'unknown',
        message: `"${row.objName}" is a ${row.baseClass || 'unknown'}, which the form designer does not support; it was not imported.`,
      });
      return drop(`${parent.docPath}.${row.objName}`);
    }

    const descriptor = getDescriptor(type);
    const entries = parseVfpProperties(row.properties);

    // The container may already have made this member itself - a Page, a Column, a Header. VFP
    // does not normally write a row for one, but when it does the row describes what is there
    // rather than a second copy of it.
    const existing = alreadyBuilt(byPath, path);
    if (existing?.node && existing.descriptor) {
      applyProperties(entries, existing.descriptor, existing.docPath, existing.node.props, reserved, expressions);
      const split = splitMemberMethods(parseVfpMethods(row.methods));
      const written = normaliseMethods(split.own, existing.descriptor);
      Object.assign(existing.node.methods, written);
      noteIncludes(row, existing.docPath, written);
      for (const m of split.members) deferredMethods.push({ from: row.objName, ...m, row });
      return;
    }

    // Names only have to be unique among siblings: VFP is quite happy with a Shape2 on each
    // page of a pageframe, and so is the object model, which addresses controls by path.
    const siblings = new Set((parent.container.children ?? []).map((c) => c.name.toLowerCase()));
    const wanted = nameFromEntries(entries) || row.objName;
    const name = dedupeName(wanted, siblings);
    if (name !== wanted) {
      warnings.push({ object: path, kind: 'renamed', message: `Name "${wanted}" is already used in "${parent.docPath}"; renamed to "${name}".` });
    }

    const docPath = `${parent.docPath}.${name}`;
    const node: ControlNode = { id: nanoid(10), type, name, props: {}, methods: {} };
    const dotted = entries.filter((e) => e.name.includes('.'));
    applyProperties(
      entries.filter((e) => !e.name.includes('.')),
      descriptor,
      docPath,
      node.props,
      reserved,
      expressions,
    );
    node.methods = normaliseMethods(parseVfpMethods(row.methods), descriptor);
    noteIncludes(row, docPath, node.methods);
    applyCustomMembers(row.custom, node, docPath, arrays);
    // an ActiveX control says which control it is only in its persisted state
    if (row.ole.length > 0 && (node.props['OleClass'] ?? '') === '') {
      const oleClass = oleClassOf(row.ole, row.ole2);
      if (oleClass !== '') node.props['OleClass'] = oleClass;
    }
    if (descriptor.container) node.children = [];
    warnMissingClass(row, path, warnings);

    // A PageFrame's pages and a Grid's columns have no rows of their own: the container says how
    // many there are and names them with dotted properties, so they are built here and
    // registered, because the rows that follow are parented to them by their new names.
    const claimed = new Set<VfpPropertyEntry>();
    buildAutoChildren({ node, type, dotted, path, docPath, reserved, expressions, claimed }, (child, childPath, childDoc) => {
      const placed: Placed = { path: childPath, docPath: childDoc, node: child, container: child, descriptor: getDescriptor(child.type), auto: true };
      register(byPath, byName, placed, child.name);
    });
    for (const entry of dotted) {
      if (!claimed.has(entry)) reserved[`${docPath}.${entry.name}`] = entry.raw;
    }

    const host = parent.container;
    if (host.children) host.children.push(node);
    else {
      warnings.push({ object: path, kind: 'other', message: `"${host.name}" cannot contain other controls; "${name}" was imported at the top level of the form.` });
      form.children.push(node);
    }
    register(byPath, byName, { path, docPath, node, container: node, descriptor, inherited: row.fromClass }, row.objName);
  }
}

/**
 * The definitions one group holds, and the formset round them when there is one.
 *
 * A formset is a container of whole forms, and a form is what the designer draws, so each member
 * form becomes a document of its own. Nothing is lost by that: the formset's own name, values,
 * code and the order of its forms go on every one of them, and the runtime builds the formset
 * back from whichever member it is asked for.
 */
function formsIn(group: Group, warnings: VfpImportWarning[]): { parts: { root: Row; members: Row[] }[]; formset?: FormsetRows } {
  if (group.root.baseClass.toLowerCase() !== 'formset') return { parts: [{ root: group.root, members: group.members }] };

  const forms = memberFormsOf(group);
  if (forms.length === 0) {
    warnings.push({ object: group.root.objName, kind: 'formset', message: `Formset "${group.root.objName}" contains no form.` });
    return { parts: [{ root: group.root, members: group.members }] };
  }
  // Each form takes the rows that hang off it. Sharing the whole member list would put one
  // form's controls on another, which is what happened while a formset imported as one form.
  return {
    parts: forms.map((root) => ({ root, members: descendantsOf(root, group.members) })),
    formset: { row: group.root, forms },
  };
}

/** The formset's own row and the rows of the forms directly inside it, in file order. */
interface FormsetRows {
  row: Row;
  forms: Row[];
}

/** The forms a formset holds: the rows whose parent is the formset itself. */
function memberFormsOf(group: Group): Row[] {
  const owner = group.root.objName.toLowerCase();
  return group.members.filter((r) => {
    if (r.baseClass.toLowerCase() !== 'form') return false;
    const segments = r.parentRef.split('.');
    return (segments[segments.length - 1] ?? '').toLowerCase() === owner;
  });
}

/**
 * The formset itself, written down: its name, the values and code on it, and which forms belong
 * to it in the order the file defines them.
 *
 * Its properties are read the way a form's are - a value written down is a value, one written as
 * an expression is worked out when the formset is built - except that nothing prunes them
 * against a descriptor, because a formset has none here.
 */
function readFormset(rows: FormsetRows, forms: readonly string[], expressions: Record<string, string>, reserved: Record<string, string>): VfpFormset {
  const entries = parseVfpProperties(rows.row.properties);
  const name = nameFromEntries(entries) || rows.row.objName;
  const props: Record<string, PropValue> = {};
  for (const entry of entries) {
    const housekeeping = HOUSEKEEPING.has(entry.name.toLowerCase()) || entry.name.startsWith('_');
    if (entry.name.includes('.') || housekeeping) {
      reserved[`${name}.${entry.name}`] = entry.raw;
      continue;
    }
    if (entry.name.toLowerCase() === 'name') continue;
    if (entry.expression) expressions[`${name}.${entry.name}`] = entry.raw;
    else props[entry.name] = entry.value;
  }
  return { name, props, methods: parseVfpMethods(rows.row.methods), forms: [...forms] };
}

/** Rows under `root`, directly or through another row, to a fixed point. */
function descendantsOf(root: Row, members: readonly Row[]): Row[] {
  const owned = new Set<string>([root.objName.toLowerCase()]);
  const out = new Set<Row>();
  for (let changed = true; changed; ) {
    changed = false;
    for (const row of members) {
      if (row === root || out.has(row)) continue;
      const segments = row.parentRef.split('.');
      const parent = (segments[segments.length - 1] ?? '').toLowerCase();
      if (!owned.has(parent)) continue;
      out.add(row);
      owned.add(row.objName.toLowerCase());
      changed = true;
    }
  }
  return [...out];
}

function register(byPath: Map<string, Placed>, byName: Map<string, Placed>, placed: Placed, objName: string): void {
  byPath.set(placed.path.toLowerCase(), placed);
  const key = objName.toLowerCase();
  if (!byName.has(key)) byName.set(key, placed);
}

/** `PARENT` is a full dotted path in nested containers, and a bare name everywhere else. */
function lookupParent(byPath: Map<string, Placed>, byName: Map<string, Placed>, parentRef: string): Placed | undefined {
  const full = byPath.get(parentRef.toLowerCase());
  if (full) return full;
  const segments = parentRef.split('.');
  const last = segments[segments.length - 1] ?? '';
  return byName.get(last.toLowerCase());
}

/** The definition's own row may be stamped from a class too: a form built from a form class. */
function expandRoot(root: Row, members: Row[], index: ClassIndex): { root: Row; members: Row[] } {
  const [newRoot, ...inherited] = expandSubclasses([root], index);
  return { root: newRoot ?? root, members: [...inherited, ...expandSubclasses(members, index)] };
}

/** The last `Name` wins, as it does for every other property once memos have been merged. */
/** What `buildAutoChildren` is working on: one container and the dotted properties aimed at it. */
interface AutoChildContext {
  /** The container being filled: a row's node, or the definition's own root. */
  node: FormNode | ControlNode;
  type: ControlType;
  dotted: VfpPropertyEntry[];
  /** VFP path of the container, which the rows that follow use to name their parent. */
  path: string;
  docPath: string;
  reserved: Record<string, string>;
  /** Values that are worked out when the form loads rather than written down. */
  expressions: Record<string, string>;
  claimed: Set<VfpPropertyEntry>;
}

/**
 * Creates the members a container makes for itself, as VFP does when it reads the file: a
 * PageFrame its Pages, an OptionGroup its buttons, a Grid its Columns and each Column a Header.
 * `Page1.Name = "pagCustomers"` in the container's memo both renames a page and is how every
 * later row refers to it, so each one is announced through `onChild` to be registered.
 */
function buildAutoChildren(ctx: AutoChildContext, onChild: (child: ControlNode, path: string, docPath: string) => void): void {
  const auto = getDescriptor(ctx.type).container?.autoChildren;
  if (!auto) return;

  const descriptor = getDescriptor(ctx.type);
  const declared = auto.countProp ? ctx.node.props[auto.countProp] : undefined;
  const fallback = auto.countProp ? getPropertyMeta(descriptor, auto.countProp)?.default : undefined;
  const count = Math.max(0, Math.trunc(Number(declared ?? fallback ?? auto.count) || 0));

  ctx.node.children ??= [];
  const desc = getDescriptor(auto.type);

  // A member a container makes for itself answers to two names. `Page4.Name = "pg4"` uses the
  // name it is born with, which is the only name it has when it is being made; a class that
  // inherits the pageframe and changes a page it did not create writes `pg1.Enabled = .F.`,
  // using the name that page already has. Both find it, so the names it was born with are
  // matched over the whole list first and the names it now has over what is left, which keeps
  // a page called "Page2" from taking what belongs to the second page.
  const firstSegment = (e: VfpPropertyEntry) => e.name.slice(0, e.name.indexOf('.')).toLowerCase();
  const strip = (e: VfpPropertyEntry) => ({ ...e, name: e.name.slice(e.name.indexOf('.') + 1) });
  const take = (wanted: string) => {
    const mine = ctx.dotted.filter((e) => !ctx.claimed.has(e) && firstSegment(e) === wanted.toLowerCase());
    for (const entry of mine) ctx.claimed.add(entry);
    return mine.map(strip);
  };

  const slots: { vfpName: string; name: string; own: VfpPropertyEntry[] }[] = [];
  for (let i = 0; i < count; i++) {
    const vfpName = `${desc.namePrefix}${i + 1}`;
    const own = take(vfpName);
    slots.push({ vfpName, name: nameFromEntries(own) || vfpName, own });
  }
  for (const slot of slots) {
    if (slot.name.toLowerCase() === slot.vfpName.toLowerCase()) continue;
    slot.own.push(...take(slot.name));
    slot.name = nameFromEntries(slot.own) || slot.name;
  }

  for (const { vfpName, name, own } of slots) {
    const child: ControlNode = { id: nanoid(10), type: auto.type, name: vfpName, props: {}, methods: {} };
    if (desc.properties.some((p) => p.name === 'Caption')) child.props['Caption'] = vfpName;
    if (desc.container) child.children = [];
    child.name = name;

    const childPath = `${ctx.path}.${child.name}`;
    const childDoc = `${ctx.docPath}.${child.name}`;
    applyProperties(
      own.filter((e) => !e.name.includes('.')),
      desc,
      childDoc,
      child.props,
      ctx.reserved,
      ctx.expressions,
    );

    ctx.node.children.push(child);
    onChild(child, childPath, childDoc);
    // a Column makes its own Header, and anything addressed deeper goes with it
    const deeper = { ...ctx, node: child, type: auto.type, dotted: own.filter((e) => e.name.includes('.')), path: childPath, docPath: childDoc };
    buildAutoChildren(deeper, onChild);
  }
}

/**
 * The member a container made for itself at this path, when it made one. Only those are merged
 * into: two ordinary rows that happen to share a name are two controls, and get renamed.
 */
function alreadyBuilt(byPath: Map<string, Placed>, path: string): Placed | undefined {
  const placed = byPath.get(path.toLowerCase());
  // A container's own member - a Page, a Column - and one that came from the class this object
  // was stamped from are both already there, so a row naming either describes it rather than a
  // second one beside it. Two rows of the file with one name are a different thing: those really
  // are two members, and one of them has to be renamed.
  const already = placed?.auto === true || placed?.inherited === true;
  return already && placed?.node && placed.descriptor ? placed : undefined;
}

function nameFromEntries(entries: VfpPropertyEntry[]): string {
  let name = '';
  for (const entry of entries) {
    if (entry.name.toLowerCase() !== 'name') continue;
    if (typeof entry.value === 'string' && entry.value.trim() !== '') name = entry.value.trim();
  }
  return name;
}

/** A custom member line: `*` marks a method, `^` an array, and the brackets hold its shape. */
const CUSTOM_MEMBER_LINE = /^([*^]?)([A-Za-z_][A-Za-z0-9_]*)(?:\[\s*(\d+)\s*(?:,\s*(\d+)\s*)?\])?/;

/**
 * Declares the members an object added for itself, from the `RESERVED3` memo.
 *
 * Visual FoxPro writes one line per custom member as `name description`, with a leading `*` on a
 * method and `^` on an array, whose shape follows in brackets: `^aIcon[5,2]`, `^oWindows[1,0]`.
 * It is the only record of a custom property that was never given a value -
 * `lCalledBySolution` has no line in the Properties memo at all - and without it every method
 * that reads one fails with "Property ... is not found".
 *
 * An array is not a property with a value, so it is recorded apart and dimensioned when the form
 * is built. Without that, `ALEN(thisform.oWindows)` is "Unknown member OWINDOWS".
 *
 * A property already carrying a value keeps it; a method already carrying source keeps that.
 */
function applyCustomMembers(
  custom: string,
  node: FormNode | ControlNode,
  objectPath: string,
  arrays: Record<string, [number, number]>,
): void {
  for (const line of custom.split(/\r?\n/)) {
    const match = CUSTOM_MEMBER_LINE.exec(line.trim());
    if (!match) continue;
    const [, mark, name, rows, cols] = match;
    if (name === undefined) continue;
    const lower = name.toLowerCase();
    if (mark === '*') {
      if (!Object.keys(node.methods).some((m) => m.toLowerCase() === lower)) node.methods[name] = '';
    } else if (rows !== undefined) {
      // a second dimension of zero is how VFP writes a list rather than a table
      arrays[`${objectPath}.${name}`] = [Number(rows), Number(cols ?? 0)];
    } else if (!Object.keys(node.props).some((k) => k.toLowerCase() === lower)) {
      // a custom property with no value is .F. in VFP, whatever it is later used to hold
      node.props[name] = false;
    }
  }
}

/**
 * Fills `props` with the entries the object carries: the ones the registry knows about sparsely,
 * only where they differ from the descriptor default, and the ones it does not know at all in
 * full. A property a form adds for itself - `lCalledBySolution`, `cOldPath` - is as real as
 * `Caption` to the code that reads it, and dropping it makes every method that touches one fail.
 *
 * What is left is what cannot be a property of this object: a value for a member further down
 * (`Page1.Caption`), which is kept verbatim in `reserved` so a re-export loses nothing.
 */
function applyProperties(
  entries: VfpPropertyEntry[],
  descriptor: ObjectDescriptor,
  objectPath: string,
  props: Record<string, PropValue>,
  reserved: Record<string, string>,
  expressions: Record<string, string> = {},
): void {
  for (const entry of entries) {
    if (!entry.name.includes('.')) {
      if (entry.name.toLowerCase() === 'name') continue;
      const declared = declaredPropertyName(descriptor, entry.name);
      if (declared) {
        const meta = descriptor.properties.find((p) => p.name === declared)!;
        // a value that is worked out is remembered as the expression it is, and the property is
        // left at its default until the form loads and it can be worked out
        if (entry.expression) {
          expressions[`${objectPath}.${declared}`] = entry.raw;
          continue;
        }
        // A property at its default is left out, which is what keeps an imported form from
        // carrying every property VFP has. But once a line has set one, a later line saying
        // "back to the default" has to be written down, or a class that inherits `PageCount = 1`
        // and puts it back to 2 keeps the 1: the memos are merged, so the last line wins.
        if (entry.value !== meta.default || declared in props) props[declared] = entry.value;
        continue;
      }
      // a property of this object the registry has never heard of: the form added it, unless it
      // is one of the designer's own notes, which describe the file rather than the object
      if (!HOUSEKEEPING.has(entry.name.toLowerCase()) && !entry.name.startsWith('_')) {
        props[entry.name] = entry.value;
        // A property of the class's own keeps its text unquoted - `cAppName = ShutterDesign` is a
        // string - so only one written in parentheses is an expression, worked out when the object
        // is made: CodeMine's `nHkeyMachineRoot = ((2^31) + 2)` is a number to the registry calls
        // that are handed it, or they read nothing. The text stays until then.
        if (isParenthesised(entry.raw)) expressions[`${objectPath}.${entry.name}`] = entry.raw.trim();
      }
      reserved[`${objectPath}.${entry.name}`] = entry.raw;
      continue;
    }
    reserved[`${objectPath}.${entry.name}`] = entry.raw;
  }
}

/**
 * Entries a Visual FoxPro designer writes into the Properties memo that are not properties of the
 * object: the name it is filed under, and the flag saying whether to build it at load time.
 * Everything beginning with an underscore is a designer's private note as well.
 */
const HOUSEKEEPING = new Set(['docreate']);

/** `(expr)`, the whole of it inside one pair of parentheses. */
function isParenthesised(raw: string): boolean {
  const t = raw.trim();
  if (!t.startsWith('(') || !t.endsWith(')')) return false;
  let depth = 0;
  for (let i = 0; i < t.length; i++) {
    if (t[i] === '(') depth++;
    else if (t[i] === ')' && --depth === 0 && i < t.length - 1) return false;
  }
  return depth === 0;
}

/** VFP writes property names in any case; the registry declares one exact spelling. */
function declaredPropertyName(descriptor: ObjectDescriptor, name: string): string | undefined {
  const lower = name.toLowerCase();
  return descriptor.properties.find((p) => p.name.toLowerCase() === lower)?.name;
}

/** Keeps VFP's spelling for user-defined procedures, but snaps known events to the declared case. */
function normaliseMethods(methods: Record<string, string>, descriptor: ObjectDescriptor): Record<string, string> {
  const out: Record<string, string> = {};
  for (const [name, source] of Object.entries(methods)) {
    // an ancestor's copy is spelled as the event it is a copy of, and keeps its depth
    const { event, depth } = ancestorEvent(name);
    const lower = event.toLowerCase();
    const spelled = descriptor.events.find((e) => e.name.toLowerCase() === lower)?.name ?? event;
    out[depth > 0 ? `${spelled}#${depth}` : spelled] = source;
  }
  return out;
}

function finish(built: Built): ImportedForm {
  const { doc, reserved, expressions, arrays } = built;
  if (built.dataDropped) {
    built.warnings.push({
      object: doc.form.name,
      kind: 'dataEnvironment',
      message: 'A data environment written inside the form itself was not imported; its tables are not opened.',
    });
  }
  const vfp: NonNullable<NonNullable<FormDocument['meta']>['vfp']> = {};
  if (built.classLib !== '') vfp.classLib = built.classLib;
  if (built.baseClass !== '' && built.baseClass.toLowerCase() !== 'form') vfp.baseClass = built.baseClass;
  if (built.include !== '') vfp.include = built.include;
  if (Object.keys(built.includes).length > 0) vfp.includes = built.includes;
  if (Object.keys(reserved).length > 0) vfp.reserved = reserved;
  if (Object.keys(expressions).length > 0) vfp.expressions = expressions;
  if (Object.keys(arrays).length > 0) vfp.arrays = arrays;
  if (built.formset) vfp.formset = built.formset;
  if (built.memberForms && built.memberForms.length > 0) vfp.memberForms = built.memberForms;
  // every converted document says which importer made it, so a later one can redo the work
  vfp.importer = IMPORTER_VERSION;
  doc.meta = { vfp };
  return { doc, warnings: built.warnings };
}
