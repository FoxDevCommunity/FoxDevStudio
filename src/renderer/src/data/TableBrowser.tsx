/**
 * BROWSE: a table shown as a table, and edited as one.
 *
 * A `.dbf` or `.dbc` is not text, and reading one into the code editor gives a screen of
 * mojibake. This is what those open into instead. It reads and writes through the same data
 * engine a running program uses - the host hands out and takes back bytes at an offset, the VM
 * turns them into values and back - so a table far larger than memory browses like a small one:
 * only the rows on screen are ever read, and only the field edited is ever written.
 *
 * A record is a fixed run of bytes, so a write never moves anything: the VM says where the field
 * starts and what belongs there, and that run is put back.
 *
 * Memo fields show the word Memo, as Visual FoxPro's BROWSE does, and fetch their text when the
 * cell is opened. They are read-only: writing one means appending a block to the `.fpt`.
 */

import { useCallback, useEffect, useState } from 'react';
import { Button, Spinner, Text, Tooltip } from '@fluentui/react-components';
import { basename } from '@shared/paths';
import { getApi } from '../api/foxdev';
import { loadFoxVm } from '../../../wasm/foxvm/loader';
import './browse.css';

/** How many records one read asks for. Matches the VM's page so the two agree on what a page is. */
const PAGE = 64;

interface Field {
  name: string;
  kind: string;
  length: number;
  decimals: number;
}

interface Header {
  ok: boolean;
  error?: string;
  version: number;
  codepage: number | null;
  fields: Field[];
  record_count: number;
  header_len: number;
  record_len: number;
  has_memo: boolean;
}

/** A field value as the page decoder hands it over. */
type Cell = string | number | boolean | null | { $date: string } | { $dt: number } | { $bytes: string };

interface Row {
  deleted: boolean;
  values: Cell[];
}

/** The open table: its handle, its header, and the header bytes the encoder and decoder need. */
interface Table {
  handle: number;
  header: Header;
  bytes: Uint8Array;
  /** False when the file could only be opened to read, so every edit would fail.  */
  writable: boolean;
}

/** The cell being edited, and what has been typed into it. */
interface Editing {
  recno: number;
  field: number;
  text: string;
}

export function TableBrowserDocument({ path, onShowTree }: { path: string; onShowTree?: () => void }) {
  const [table, setTable] = useState<Table | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [rows, setRows] = useState<Map<number, Row>>(new Map());
  const [first, setFirst] = useState(1);
  const [memo, setMemo] = useState<{ field: string; text: string } | null>(null);
  const [editing, setEditing] = useState<Editing | null>(null);
  const [cellError, setCellError] = useState<string | null>(null);
  const [reload, setReload] = useState(0);

  // open once per path, and let go of the file when the tab closes
  useEffect(() => {
    let live = true;
    let opened: number | null = null;
    void (async () => {
      try {
        const vm = await loadFoxVm();
        const answer = await getApi().data.open(path, true);
        opened = answer.handle;
        if (!live) return;
        const bytes = fromLatin1(answer.header);
        const parsed = vm.read_dbf_header(bytes) as Header;
        if (!parsed.ok) throw new Error(parsed.error ?? 'the header could not be read');
        setTable({ handle: answer.handle, header: parsed, bytes, writable: answer.writable });
      } catch (e) {
        if (live) setError(e instanceof Error ? e.message : String(e));
      }
    })();
    return () => {
      live = false;
      setTable(null);
      if (opened !== null) void getApi().data.close(opened);
    };
  }, [path]);

  // the page of records on screen, fetched when the table opens and whenever the window moves
  useEffect(() => {
    if (!table) return;
    let live = true;
    void (async () => {
      const vm = await loadFoxVm();
      const offset = table.header.header_len + (first - 1) * table.header.record_len;
      const text = await getApi().data.read(table.handle, offset, PAGE * table.header.record_len);
      const page = vm.decode_dbf_page(table.bytes, fromLatin1(text)) as { ok: boolean; records: Row[] };
      if (!live || !page.ok) return;
      setRows(new Map(page.records.map((r, i): [number, Row] => [first + i, r])));
    })();
    return () => {
      live = false;
    };
  }, [table, first, reload]);

  const openMemo = useCallback(
    async (field: Field, block: number) => {
      if (!table) return;
      const text = await getApi().data.readMemo(table.handle, block);
      // the block arrives with its 8-byte header, which is not part of the text
      setMemo({ field: field.name, text: text.slice(8) });
    },
    [table],
  );

  /** Writes one field of one record back, then re-reads the page so the grid shows what is there. */
  const commit = useCallback(
    async (edit: Editing) => {
      if (!table) return;
      const vm = await loadFoxVm();
      const encoded = vm.encode_dbf_field(table.bytes, edit.field, edit.text) as
        | { ok: true; offset: number; bytes: string }
        | { ok: false; error: string };
      if (!encoded.ok) {
        setCellError(encoded.error);
        return;
      }
      const at = table.header.header_len + (edit.recno - 1) * table.header.record_len + encoded.offset;
      try {
        await getApi().data.write(table.handle, at, encoded.bytes);
      } catch (e) {
        setCellError(e instanceof Error ? e.message : String(e));
        return;
      }
      setCellError(null);
      setEditing(null);
      setReload((n) => n + 1);
    },
    [table],
  );

  /** Marks a record deleted, or brings it back: one byte at the front of the record. */
  const setDeleted = useCallback(
    async (recno: number, deleted: boolean) => {
      if (!table) return;
      const at = table.header.header_len + (recno - 1) * table.header.record_len;
      try {
        await getApi().data.write(table.handle, at, deleted ? '*' : ' ');
      } catch (e) {
        setCellError(e instanceof Error ? e.message : String(e));
        return;
      }
      setReload((n) => n + 1);
    },
    [table],
  );

  if (error) {
    return (
      <div style={{ padding: 16 }}>
        <Text>{`${basename(path)} could not be read: ${error}`}</Text>
      </div>
    );
  }
  if (!table) {
    return (
      <div style={{ padding: 16 }}>
        <Spinner size="tiny" label={`Opening ${basename(path)}`} />
      </div>
    );
  }

  const header = table.header;
  const count = header.record_count;
  const last = Math.min(first + PAGE - 1, count);

  return (
    <div className="fx-browse" data-testid="table-browser">
      <div className="fx-browse__bar">
        <Text size={200}>{summary(header)}</Text>
        {!table.writable && (
          <Text size={200} style={{ color: 'var(--colorNeutralForeground3)' }}>
            read-only
          </Text>
        )}
        {cellError && (
          <Text size={200} className="fx-browse__error" role="alert">
            {cellError}
          </Text>
        )}
        <div style={{ flex: 1 }} />
        {onShowTree && (
          <Button size="small" appearance="subtle" onClick={onShowTree}>
            Show database
          </Button>
        )}
        <Button size="small" appearance="subtle" disabled={first <= 1} onClick={() => setFirst(1)}>
          Top
        </Button>
        <Button size="small" appearance="subtle" disabled={first <= 1} onClick={() => setFirst(Math.max(1, first - PAGE))}>
          Previous
        </Button>
        <Text size={200} aria-label="Record range">
          {count === 0 ? 'empty' : `${first}-${last}`}
        </Text>
        <Button size="small" appearance="subtle" disabled={last >= count} onClick={() => setFirst(first + PAGE)}>
          Next
        </Button>
        <Button size="small" appearance="subtle" disabled={last >= count} onClick={() => setFirst(Math.max(1, count - PAGE + 1))}>
          Bottom
        </Button>
      </div>

      <div className="fx-browse__scroll">
        <table className="fx-browse__grid">
          <thead>
            <tr>
              <th className="fx-browse__recno">Rec</th>
              {header.fields.map((f) => (
                <th key={f.name} title={fieldType(f)}>
                  {f.name}
                </th>
              ))}
              <th className="fx-browse__filler" />
            </tr>
          </thead>
          <tbody>
            {range(first, last).map((recno) => {
              const row = rows.get(recno);
              return (
                <tr key={recno} className={row?.deleted ? 'fx-browse__deleted' : undefined}>
                  <td className="fx-browse__recno">
                    <Tooltip content={`${row?.deleted ? 'Recall' : 'Delete'} record ${recno}`} relationship="label">
                      <button
                        type="button"
                        className="fx-browse__recno-button"
                        disabled={!table.writable}
                        onClick={() => void setDeleted(recno, !row?.deleted)}
                      >
                        {recno}
                      </button>
                    </Tooltip>
                  </td>
                  {header.fields.map((f, i) => (
                    <td
                      key={f.name}
                      className={isNumeric(f) ? 'fx-browse__num' : undefined}
                      onDoubleClick={() =>
                        table.writable
                          ? beginEdit(f, recno, i, row, setEditing, setCellError)
                          : setCellError('This file is read-only.')
                      }
                    >
                      {editing?.recno === recno && editing.field === i ? (
                        <input
                          className="fx-browse__input"
                          aria-label={`${f.name} of record ${recno}`}
                          autoFocus
                          value={editing.text}
                          onChange={(e) => setEditing({ ...editing, text: e.target.value })}
                          onBlur={() => void commit(editing)}
                          onKeyDown={(e) => {
                            if (e.key === 'Enter') void commit(editing);
                            if (e.key === 'Escape') {
                              setEditing(null);
                              setCellError(null);
                            }
                          }}
                        />
                      ) : row ? (
                        renderCell(f, row.values[i] ?? null, openMemo)
                      ) : (
                        ''
                      )}
                    </td>
                  ))}
                  <td className="fx-browse__filler" />
                </tr>
              );
            })}
          </tbody>
        </table>
      </div>

      {memo && (
        <div className="fx-browse__memo">
          <div className="fx-browse__bar">
            <Text size={200} weight="semibold">
              {memo.field}
            </Text>
            <div style={{ flex: 1 }} />
            <Button size="small" appearance="subtle" onClick={() => setMemo(null)}>
              Close
            </Button>
          </div>
          <pre>{memo.text}</pre>
        </div>
      )}
    </div>
  );
}

/** Starts editing a cell, seeding the box with the value as text. Memo fields are read-only. */
function beginEdit(
  field: Field,
  recno: number,
  index: number,
  row: Row | undefined,
  setEditing: (e: Editing) => void,
  setCellError: (e: string | null) => void,
) {
  if (!row) return;
  if ('MGP'.includes(field.kind)) {
    setCellError('Memo fields cannot be edited yet.');
    return;
  }
  setCellError(null);
  setEditing({ recno, field: index, text: editText(row.values[index] ?? null) });
}

/** What a value looks like in the edit box: what the encoder will read back. */
function editText(value: Cell): string {
  if (value === null || value === undefined) return '';
  if (typeof value === 'boolean') return value ? 'T' : 'F';
  if (typeof value === 'object') {
    if ('$date' in value) return value.$date;
    if ('$dt' in value) return new Date(value.$dt * 1000).toISOString().slice(0, 19);
    return '';
  }
  return String(value);
}

function summary(header: Header): string {
  const fields = header.fields.length;
  const records = header.record_count;
  return `${fields} ${fields === 1 ? 'field' : 'fields'}, ${records.toLocaleString()} ${records === 1 ? 'record' : 'records'}`;
}

function fieldType(f: Field): string {
  return `${f.kind}(${f.length}${f.decimals ? `,${f.decimals}` : ''})`;
}

function isNumeric(f: Field): boolean {
  return 'NFIBY+'.includes(f.kind);
}

/** One cell. Memo fields hold a block number here, not their text. */
function renderCell(field: Field, value: Cell, openMemo: (f: Field, block: number) => void) {
  if (value === null || value === undefined) return '';
  if ('MGP'.includes(field.kind)) {
    const block = typeof value === 'number' ? value : 0;
    if (!block) return '';
    return (
      <button type="button" className="fx-browse__link" onClick={() => void openMemo(field, block)}>
        Memo
      </button>
    );
  }
  if (typeof value === 'boolean') return value ? '.T.' : '.F.';
  if (typeof value === 'object') {
    if ('$date' in value) return value.$date;
    if ('$dt' in value) return new Date(value.$dt * 1000).toISOString().replace('T', ' ').slice(0, 19);
    return `${base64Length(value.$bytes)} bytes`;
  }
  return String(value);
}

function base64Length(b64: string): number {
  return Math.floor((b64.replace(/=+$/, '').length * 3) / 4);
}

function range(from: number, to: number): number[] {
  return Array.from({ length: Math.max(0, to - from + 1) }, (_, i) => from + i);
}

/** One character per byte, the way the data engine sends them. */
function fromLatin1(text: string): Uint8Array {
  const out = new Uint8Array(text.length);
  for (let i = 0; i < text.length; i++) out[i] = text.charCodeAt(i) & 0xff;
  return out;
}
