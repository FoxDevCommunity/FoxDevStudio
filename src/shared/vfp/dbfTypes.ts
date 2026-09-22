/**
 * The shape the Rust DBF reader hands back over the wasm bridge, plus small accessors so
 * callers read fields by name rather than by position.
 */

export interface DbfFieldInfo {
  name: string;
  /** DBF type letter: C N F I B Y L D T M G P V Q. */
  kind: string;
  length: number;
  decimals: number;
}

export type DbfCellValue = string | number | boolean | null | { $date: string } | { $dt: number } | { $bytes: string };

export interface DbfRecordData {
  deleted: boolean;
  values: DbfCellValue[];
}

export interface DbfTableData {
  ok: true;
  version: number;
  codepage: number | null;
  fields: DbfFieldInfo[];
  records: DbfRecordData[];
}

export type DbfReadResult = DbfTableData | { ok: false; error: string };

/** Reads one field of a record by name, case-insensitively. */
export function cell(table: DbfTableData, record: DbfRecordData, field: string): DbfCellValue {
  const index = table.fields.findIndex((f) => f.name.toLowerCase() === field.toLowerCase());
  return index < 0 ? null : (record.values[index] ?? null);
}

/** Field value as trimmed text; memo and character fields both arrive as strings. */
export function text(table: DbfTableData, record: DbfRecordData, field: string): string {
  const value = cell(table, record, field);
  return typeof value === 'string' ? value.trim() : '';
}

/** Field value as a logical; VFP writes these as `L` fields. */
export function flag(table: DbfTableData, record: DbfRecordData, field: string): boolean {
  return cell(table, record, field) === true;
}

export function num(table: DbfTableData, record: DbfRecordData, field: string): number {
  const value = cell(table, record, field);
  return typeof value === 'number' ? value : 0;
}

/**
 * Field value as bytes. Binary memos - an OLE control's persisted state, compiled p-code - cross
 * the bridge base64-encoded, because a code page would not survive them.
 */
export function binary(table: DbfTableData, record: DbfRecordData, field: string): Uint8Array {
  const value = cell(table, record, field);
  if (value === null || typeof value !== 'object' || !('$bytes' in value)) return new Uint8Array(0);
  const text = atob(value.$bytes);
  const out = new Uint8Array(text.length);
  for (let i = 0; i < text.length; i++) out[i] = text.charCodeAt(i);
  return out;
}

export function hasField(table: DbfTableData, field: string): boolean {
  return table.fields.some((f) => f.name.toLowerCase() === field.toLowerCase());
}
