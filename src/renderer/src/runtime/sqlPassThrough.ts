/**
 * SQL pass-through: `SQLCONNECT()`, `SQLEXEC()` and the rest, over ADO.
 *
 * Visual FoxPro reaches a data source through ODBC. What is reachable from here is COM, and the
 * thing on the other side of COM that speaks to every data source is ADO: `ADODB.Connection`,
 * which takes the same connection strings and the same DSN names. So a connection is an ADO
 * connection kept under a number, and a statement is `Execute` with its recordset read out.
 *
 * Nothing here parses SQL or decides what a column means: the columns come back as ADO
 * describes them and the VM makes the cursor.
 */

import type { OleValue } from '@shared/ipc/api';
import type { VmValue } from '@shared/runtime/values';
import { getApi } from '../api/foxdev';

/** One open connection: the ADO object, and the statement that is waiting to be run. */
interface Connection {
  object: number;
  /** The recordset the last statement left, when it had more results to come. */
  results: number | null;
}

const connections = new Map<number, Connection>();
let nextHandle = 1;

/** ADO's type numbers, and the Visual FoxPro field type each of them arrives as. */
const TYPES: Record<number, [kind: string, width: number, decimals: number]> = {
  2: ['I', 4, 0], // adSmallInt
  3: ['I', 4, 0], // adInteger
  4: ['N', 20, 6], // adSingle
  5: ['B', 8, 0], // adDouble
  6: ['Y', 8, 4], // adCurrency
  7: ['D', 8, 0], // adDate
  11: ['L', 1, 0], // adBoolean
  14: ['Y', 8, 4], // adDecimal
  17: ['I', 4, 0], // adTinyInt
  20: ['B', 8, 0], // adBigInt
  72: ['C', 36, 0], // adGUID
  131: ['N', 20, 6], // adNumeric
  133: ['D', 8, 0], // adDBDate
  135: ['T', 8, 0], // adDBTimeStamp
  200: ['C', 254, 0], // adVarChar
  201: ['M', 4, 0], // adLongVarChar
  202: ['C', 254, 0], // adVarWChar
  203: ['M', 4, 0], // adLongVarWChar
  204: ['M', 4, 0], // adBinary
  205: ['M', 4, 0], // adLongVarBinary
};

const ole = () => getApi().ole;
const str = (text: string): OleValue => ({ kind: 'string', text });
const num = (n: number): OleValue => ({ kind: 'number', num: n });

function plain(value: OleValue): VmValue {
  switch (value.kind) {
    case 'string':
      return value.text ?? '';
    case 'number':
      return value.num ?? 0;
    case 'bool':
      return value.flag ?? false;
    case 'date':
      // a date crosses COM as the milliseconds it stands for; the VM counts in seconds
      return { $dt: (value.num ?? 0) / 1000 };
    case 'object':
      return null;
    default:
      return null;
  }
}

function count(handle: number, name: string): number {
  const value = ole().get(handle, name, []);
  return value.kind === 'number' ? (value.num ?? 0) : 0;
}

/** Reads a whole recordset out: the columns first, then a row each. */
function readRows(recordset: number): VmValue[] {
  const fields = ole().get(recordset, 'Fields', []);
  if (fields.kind !== 'object') return [];
  const list = fields.handle ?? 0;
  const columns: VmValue[] = [];
  const shape: number[] = [];
  const total = count(list, 'Count');
  for (let i = 0; i < total; i += 1) {
    const field = ole().get(list, 'Item', [num(i)]);
    if (field.kind !== 'object') continue;
    const at = field.handle ?? 0;
    shape.push(at);
    const type = count(at, 'Type');
    const [kind, width, decimals] = TYPES[type] ?? ['C', 254, 0];
    const defined = count(at, 'DefinedSize');
    const name = ole().get(at, 'Name', []);
    columns.push({
      $arr: [
        name.kind === 'string' ? (name.text ?? '') : `COL${i + 1}`,
        kind,
        kind === 'C' ? Math.min(Math.max(defined, 1), 254) : width,
        decimals,
      ],
      $cols: 0,
    });
  }
  const out: VmValue[] = [{ $arr: columns, $cols: 0 }];
  // an empty recordset is at both ends at once, which is what stops the walk before it starts
  const done = (): boolean => {
    const eof = ole().get(recordset, 'EOF', []);
    return eof.kind === 'bool' ? (eof.flag ?? true) : true;
  };
  let guard = 0;
  while (!done() && guard < 1_000_000) {
    const row: VmValue[] = shape.map((field) => plain(ole().get(field, 'Value', [])));
    out.push({ $arr: row, $cols: 0 });
    ole().call(recordset, 'MoveNext', []);
    guard += 1;
  }
  return out;
}

/** What a failed call answers with: the number Visual FoxPro answers, and the message. */
function failed(error: unknown): number {
  lastError = error instanceof Error ? error.message : String(error);
  return -1;
}

let lastError = '';

/** The message the last pass-through call failed with, for the host to report. */
export function sqlError(): string {
  return lastError;
}

/** Lets go of every connection: the end of a run. */
export function closeSqlConnections(): void {
  for (const [, connection] of connections) {
    try {
      ole().call(connection.object, 'Close', []);
    } catch {
      // a connection the server already dropped needs no closing
    }
    ole().release(connection.object);
  }
  connections.clear();
}

/**
 * One pass-through call. `what` is the same number the VM sends: 0 connect, 1 disconnect,
 * 2 run a statement, 3 the tables, 4 the columns of one, 5 commit, 6 roll back, 7 cancel,
 * 8 the next result set.
 */
export function performSql(what: number, handle: number, text: string): VmValue {
  lastError = '';
  if (!ole().available()) {
    lastError = 'a data source is reached through ADO, which needs COM';
    return -1;
  }
  try {
    if (what === 0) {
      const object = ole().create('ADODB.Connection');
      ole().call(object, 'Open', [str(text)]);
      const number = nextHandle;
      nextHandle += 1;
      connections.set(number, { object, results: null });
      return number;
    }
    const connection = connections.get(handle);
    if (!connection && handle !== 0) {
      lastError = `connection ${handle} is not open`;
      return -1;
    }
    switch (what) {
      // SQLDISCONNECT: a handle of 0 lets every connection go
      case 1: {
        if (handle === 0) {
          closeSqlConnections();
          return 1;
        }
        ole().call(connection!.object, 'Close', []);
        ole().release(connection!.object);
        connections.delete(handle);
        return 1;
      }
      case 2: {
        const answer = ole().call(connection!.object, 'Execute', [str(text)]);
        if (answer.kind !== 'object') return 1;
        const recordset = answer.handle ?? 0;
        const rows = readRows(recordset);
        connection!.results = recordset;
        return { $arr: rows, $cols: 0 };
      }
      // SQLTABLES and SQLCOLUMNS: what the source says it holds, through ADO's schema rowsets
      case 3:
      case 4: {
        const schema = what === 3 ? 20 : 4; // adSchemaTables, adSchemaColumns
        const answer = ole().call(connection!.object, 'OpenSchema', [num(schema)]);
        if (answer.kind !== 'object') return 1;
        return { $arr: readRows(answer.handle ?? 0), $cols: 0 };
      }
      case 5:
        ole().call(connection!.object, 'CommitTrans', []);
        return 1;
      case 6:
        ole().call(connection!.object, 'RollbackTrans', []);
        return 1;
      case 7:
        ole().call(connection!.object, 'Cancel', []);
        return 1;
      // SQLMORERESULTS: what the statement left behind, if it left anything
      case 8: {
        const held = connection?.results;
        if (!held) return 2;
        const answer = ole().call(held, 'NextRecordset', []);
        if (answer.kind !== 'object') {
          connection!.results = null;
          return 2;
        }
        connection!.results = answer.handle ?? 0;
        return { $arr: readRows(answer.handle ?? 0), $cols: 0 };
      }
      default:
        return -1;
    }
  } catch (error) {
    return failed(error);
  }
}
