/**
 * Values as they cross the wasm bridge. Mirrors `host::JsonValue` in the Rust crate:
 * primitives travel as themselves, objects as handles, dates as ISO text, arrays by value.
 */

export type VmValue = null | boolean | number | string | VmObjectRef | VmFuncRef | VmDate | VmDateTime | VmArray | VmVarRef;

/**
 * A variable passed by reference to a method: the cell it lives in, and its value. Code of the
 * program's that the host runs is given the cell back, so it writes the caller's variable;
 * anything else only reads `$val`.
 */
export interface VmVarRef {
  $ref: number;
  $val: VmValue;
}

/** An argument as a method the host answers itself wants it: the value, not the variable. */
export function argValue(v: VmValue): VmValue {
  return typeof v === 'object' && v !== null && '$ref' in v ? v.$val : v;
}

export interface VmObjectRef {
  $obj: number;
}
/**
 * A lambda. It travels as its id in the VM's own table, the way an object travels as its
 * handle in the host's: the host keeps it, hands it back, and the VM finds the same function.
 */
export interface VmFuncRef {
  $fn: number;
}
export interface VmDate {
  /** `YYYY-MM-DD`, or `''` for the empty date `{}`. */
  $date: string;
}
export interface VmDateTime {
  /** Seconds since the epoch; `NaN` for the empty datetime. */
  $dt: number;
}
export interface VmArray {
  $arr: VmValue[];
  /** 0 for a one-dimensional array. */
  $cols: number;
}

export function isObjectRef(v: VmValue): v is VmObjectRef {
  return typeof v === 'object' && v !== null && '$obj' in v;
}
export function isFuncRef(v: VmValue): v is VmFuncRef {
  return typeof v === 'object' && v !== null && '$fn' in v;
}
export function isDate(v: VmValue): v is VmDate {
  return typeof v === 'object' && v !== null && '$date' in v;
}
export function isDateTime(v: VmValue): v is VmDateTime {
  return typeof v === 'object' && v !== null && '$dt' in v;
}
export function isArray(v: VmValue): v is VmArray {
  return typeof v === 'object' && v !== null && '$arr' in v;
}

export function objectRef(handle: number): VmObjectRef {
  return { $obj: handle };
}

/** VFP `VARTYPE()` letter. */
export function vartype(v: VmValue): string {
  if (v === null) return 'X';
  if (typeof v === 'boolean') return 'L';
  if (typeof v === 'number') return 'N';
  if (typeof v === 'string') return 'C';
  if (isObjectRef(v)) return 'O';
  // FoxScript's own letter for a lambda; see docs/foxscript.md
  if (isFuncRef(v)) return 'F';
  if (isDate(v)) return 'D';
  if (isDateTime(v)) return 'T';
  return 'A';
}

/** A JS `Date` as a VFP date value (local time). */
export function toVmDate(d: Date): VmDate {
  const pad = (n: number) => String(n).padStart(2, '0');
  return { $date: `${String(d.getFullYear()).padStart(4, '0')}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}` };
}

export function fromVmDate(v: VmDate): Date | null {
  if (!v.$date) return null;
  const [y, m, d] = v.$date.split('-').map(Number);
  return y === undefined || m === undefined || d === undefined ? null : new Date(y, m - 1, d);
}

/**
 * Text for a value the way `?` and `TRANSFORM()` show it. Kept in sync with `value::display`
 * in Rust; used by the JS side for the Output panel and control captions.
 */
export function displayValue(v: VmValue, decimals = 2): string {
  if (v === null) return '.NULL.';
  if (typeof v === 'boolean') return v ? '.T.' : '.F.';
  if (typeof v === 'number') return formatNumber(v, decimals);
  if (typeof v === 'string') return v;
  if (isObjectRef(v)) return `(Object ${v.$obj})`;
  if (isDate(v)) {
    const d = fromVmDate(v);
    if (!d) return '  /  /  ';
    const pad = (n: number) => String(n).padStart(2, '0');
    return `${pad(d.getMonth() + 1)}/${pad(d.getDate())}/${String(d.getFullYear()).slice(-2)}`;
  }
  if (isDateTime(v)) return Number.isNaN(v.$dt) ? '  /  /  ' : new Date(v.$dt * 1000).toLocaleString();
  return '(Array)';
}

function formatNumber(n: number, decimals: number): string {
  if (Number.isNaN(n)) return 'NaN';
  if (!Number.isFinite(n)) return n > 0 ? 'Infinity' : '-Infinity';
  if (Number.isInteger(n) && Math.abs(n) < 1e15) return String(n);
  const s = n.toFixed(decimals);
  return /^-0\.0*$/.test(s) ? s.slice(1) : s;
}

/**
 * Converts a document property value (`PropValue`: string | number | boolean | null) into a
 * VM value. `null` is a genuine VFP .NULL. here; absent properties never reach the VM.
 */
export function propToVm(v: string | number | boolean | null | undefined): VmValue {
  return v === undefined ? null : v;
}

/** Converts a VM value back into a document property value, dropping objects and arrays. */
export function vmToProp(v: VmValue): string | number | boolean | null {
  if (v === null || typeof v === 'string' || typeof v === 'number' || typeof v === 'boolean') return v;
  if (isDate(v) || isDateTime(v)) return displayValue(v);
  return null;
}
