/**
 * A database container, shown as what it is.
 *
 * A `.dbc` is a table whose records describe a tree: a database owns tables, a table owns its
 * fields, indexes and relations, and every record points at its parent. Reading that as a grid is
 * accurate and useless - 26 rows of OBJECTID and PARENTID tell you nothing about the database.
 * This assembles the tree instead, and each table in it opens in the browser.
 *
 * The records stay editable, because they are the database: renaming an object here renames it
 * there. The grid is one click away for the rest.
 */

import { useCallback, useEffect, useRef, useState } from 'react';
import { Button, Spinner, Text, Tree, TreeItem, TreeItemLayout } from '@fluentui/react-components';
import { basename, dirname, join } from '@shared/paths';
import { getApi } from '../api/foxdev';
import { loadFoxVm } from '../../../wasm/foxvm/loader';
import { openFile } from '../stores/fileActions';
import { resolveIcon } from '../designer/icons';
import './browse.css';

const DatabaseIcon = resolveIcon('Database');
const TableIcon = resolveIcon('Table');
const FieldIcon = resolveIcon('TextField');
const IndexIcon = resolveIcon('Key');
const RelationIcon = resolveIcon('Link');
const OtherIcon = resolveIcon('Document');

/** One record of the container, as the reader hands it over. */
interface Entry {
  id: number;
  parent: number;
  type: string;
  name: string;
  /** Which record of the container it came from, so a renamed object is written back to it. */
  recno: number;
}

/** An entry with the entries that belong to it. */
interface Node extends Entry {
  children: Node[];
}

/** The colour a kind of object is drawn in, so the tree is scannable rather than uniform. */
const COLOURS: Record<string, string> = {
  database: 'var(--colorPaletteTealForeground2)',
  table: 'var(--colorPaletteBlueForeground2)',
  field: 'var(--colorPaletteGreenForeground2)',
  index: 'var(--colorPaletteMarigoldForeground2)',
  relation: 'var(--colorPalettePurpleForeground2)',
};

function iconFor(type: string) {
  switch (type.toLowerCase()) {
    case 'database':
      return { Icon: DatabaseIcon, colour: COLOURS.database };
    case 'table':
      return { Icon: TableIcon, colour: COLOURS.table };
    case 'field':
      return { Icon: FieldIcon, colour: COLOURS.field };
    case 'index':
      return { Icon: IndexIcon, colour: COLOURS.index };
    case 'relation':
      return { Icon: RelationIcon, colour: COLOURS.relation };
    default:
      return { Icon: OtherIcon, colour: 'var(--colorNeutralForeground3)' };
  }
}

export function DatabaseDesignerDocument({ path, onShowRecords }: { path: string; onShowRecords: () => void }) {
  const [tree, setTree] = useState<Node[] | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [table, setTable] = useState<{ handle: number; header: Uint8Array; headerLen: number; recordLen: number } | null>(null);
  const [editing, setEditing] = useState<{ recno: number; text: string } | null>(null);
  const [reload, setReload] = useState(0);
  /** Which field of the container holds the object name, for writing a rename back. */
  const columnIndex = useRef(-1);

  useEffect(() => {
    let live = true;
    let handle: number | null = null;
    void (async () => {
      try {
        const vm = await loadFoxVm();
        const opened = await getApi().data.open(path, true);
        handle = opened.handle;
        const header = fromLatin1(opened.header);
        const parsed = vm.read_dbf_header(header) as {
          ok: boolean;
          error?: string;
          fields: { name: string }[];
          record_count: number;
          header_len: number;
          record_len: number;
        };
        if (!parsed.ok) throw new Error(parsed.error ?? 'the header could not be read');

        // a container is small by nature - it describes a database, it does not hold one
        const bytes = await getApi().data.read(opened.handle, parsed.header_len, parsed.record_count * parsed.record_len);
        const page = vm.decode_dbf_page(header, fromLatin1(bytes)) as {
          ok: boolean;
          records: { deleted: boolean; values: (string | number | boolean | null)[] }[];
        };
        if (!live || !page.ok) return;
        setTable({ handle: opened.handle, header, headerLen: parsed.header_len, recordLen: parsed.record_len });

        const at = (name: string) => parsed.fields.findIndex((f) => f.name.toUpperCase() === name);
        const columns = { id: at('OBJECTID'), parent: at('PARENTID'), type: at('OBJECTTYPE'), name: at('OBJECTNAME') };
        if (Object.values(columns).some((i) => i < 0)) {
          throw new Error('this is not a database container');
        }
        const entries: Entry[] = page.records
          .map((r, i) => ({
            id: Number(r.values[columns.id] ?? 0),
            parent: Number(r.values[columns.parent] ?? 0),
            type: String(r.values[columns.type] ?? '').trim(),
            name: String(r.values[columns.name] ?? '').trim(),
            recno: i + 1,
            deleted: r.deleted,
          }))
          .filter((e) => !e.deleted && !isBookkeeping(e))
          .map(({ deleted: _deleted, ...e }) => e);
        setTree(assemble(entries));
        columnIndex.current = columns.name;
      } catch (e) {
        if (live) setError(e instanceof Error ? e.message : String(e));
      }
    })();
    return () => {
      live = false;
      if (handle !== null) void getApi().data.close(handle);
    };
  }, [path, reload]);

  /** Writes a renamed object back to the record it came from. */
  const rename = useCallback(
    async (recno: number, text: string) => {
      if (!table || columnIndex.current < 0) return;
      const vm = await loadFoxVm();
      const encoded = vm.encode_dbf_field(table.header, columnIndex.current, text) as
        | { ok: true; offset: number; bytes: string }
        | { ok: false; error: string };
      if (!encoded.ok) {
        setError(encoded.error);
        return;
      }
      const at = table.headerLen + (recno - 1) * table.recordLen + encoded.offset;
      try {
        await getApi().data.write(table.handle, at, encoded.bytes);
      } catch (e) {
        setError(e instanceof Error ? e.message : String(e));
        return;
      }
      setEditing(null);
      setReload((n) => n + 1);
    },
    [table],
  );

  /** A table of the database opens as a table: the file sits beside the container. */
  const openTable = useCallback(
    async (name: string) => {
      await openFile(join(dirname(path), `${name}.dbf`));
    },
    [path],
  );

  if (error) {
    return (
      <div style={{ padding: 16 }}>
        <Text>{`${basename(path)} could not be read: ${error}`}</Text>
      </div>
    );
  }
  if (!tree) {
    return (
      <div style={{ padding: 16 }}>
        <Spinner size="tiny" label={`Opening ${basename(path)}`} />
      </div>
    );
  }

  return (
    <div className="fx-browse" data-testid="database-designer">
      <div className="fx-browse__bar">
        <Text size={200}>{count(tree, 'Table')} tables</Text>
        <div style={{ flex: 1 }} />
        <Button size="small" appearance="subtle" onClick={onShowRecords}>
          Show records
        </Button>
      </div>
      <div className="fx-browse__scroll" style={{ padding: '4px 8px' }}>
        {/* a database is not deep, and its tables are what it is: everything starts open */}
        <Tree aria-label="Database" defaultOpenItems={branches(tree)}>
          {tree.map((node) => (
            <Branch
              key={node.id}
              node={node}
              onOpenTable={openTable}
              editing={editing}
              onBeginEdit={(n) => setEditing({ recno: n.recno, text: n.name })}
              onChange={(text) => setEditing((e) => (e ? { ...e, text } : e))}
              onCommit={rename}
              onCancel={() => setEditing(null)}
            />
          ))}
        </Tree>
      </div>
    </div>
  );
}

interface BranchProps {
  node: Node;
  onOpenTable: (name: string) => void;
  editing: { recno: number; text: string } | null;
  onBeginEdit: (node: Node) => void;
  onChange: (text: string) => void;
  onCommit: (recno: number, text: string) => void;
  onCancel: () => void;
}

function Branch({ node, onOpenTable, editing, onBeginEdit, onChange, onCommit, onCancel }: BranchProps) {
  const { Icon, colour } = iconFor(node.type);
  const isTable = node.type.toLowerCase() === 'table';
  const isEditing = editing?.recno === node.recno;
  const label = (
    <TreeItemLayout
      iconBefore={
        <span style={{ color: colour, display: 'inline-flex' }}>
          <Icon />
        </span>
      }
      aria-label={`${node.type} ${node.name}`}
      // a table opens; anything else is renamed in place, which is what a name in a container is
      onDoubleClick={isTable ? () => onOpenTable(node.name) : () => onBeginEdit(node)}
    >
      {isEditing ? (
        <input
          className="fx-browse__input"
          aria-label={`Name of ${node.type} ${node.name}`}
          autoFocus
          value={editing.text}
          onChange={(e) => onChange(e.target.value)}
          onBlur={() => onCommit(node.recno, editing.text)}
          // the tree navigates with the keyboard and answers to clicks; while a name is being
          // edited the box has both, or Enter would fold the branch instead of saving the name
          onClick={(e) => e.stopPropagation()}
          onKeyDown={(e) => {
            e.stopPropagation();
            if (e.key === 'Enter') onCommit(node.recno, editing.text);
            if (e.key === 'Escape') onCancel();
          }}
        />
      ) : (
        <>
          {node.name || '(unnamed)'}
          <span style={{ color: 'var(--colorNeutralForeground3)' }}> {node.type}</span>
        </>
      )}
    </TreeItemLayout>
  );
  if (node.children.length === 0) {
    return (
      <TreeItem itemType="leaf" value={String(node.id)}>
        {label}
      </TreeItem>
    );
  }
  return (
    <TreeItem itemType="branch" value={String(node.id)}>
      {label}
      <Tree>
        {node.children.map((child) => (
          <Branch
            key={child.id}
            node={child}
            onOpenTable={onOpenTable}
            editing={editing}
            onBeginEdit={onBeginEdit}
            onChange={onChange}
            onCommit={onCommit}
            onCancel={onCancel}
          />
        ))}
      </Tree>
    </TreeItem>
  );
}

/**
 * A container keeps records for its own bookkeeping - the transaction log, and the three that
 * hold stored procedure text - which are not objects of the database and which Visual FoxPro does
 * not show either.
 */
function isBookkeeping(e: { type: string; name: string; id: number; parent: number }): boolean {
  if (!e.type.toLowerCase().startsWith('database')) return false;
  return e.id !== e.parent;
}

/**
 * Builds the tree from the parent pointers. The database is its own parent, which is how a
 * container says "this is the root", so that is where the walk starts.
 */
function assemble(entries: Entry[]): Node[] {
  const nodes = new Map<number, Node>(entries.map((e) => [e.id, { ...e, children: [] }]));
  const roots: Node[] = [];
  for (const node of nodes.values()) {
    const parent = node.parent === node.id ? undefined : nodes.get(node.parent);
    if (parent) parent.children.push(node);
    else roots.push(node);
  }
  // fields before indexes before relations, and each group by name, which is how VFP shows them
  const order = ['field', 'index', 'relation'];
  const sort = (list: Node[]) => {
    list.sort((a, b) => {
      const rank = order.indexOf(a.type.toLowerCase()) - order.indexOf(b.type.toLowerCase());
      return rank !== 0 ? rank : a.name.localeCompare(b.name);
    });
    list.forEach((n) => sort(n.children));
  };
  sort(roots);
  return roots;
}

/** Every node that has children, so the tree opens showing the whole database. */
function branches(nodes: Node[]): string[] {
  return nodes.flatMap((n) => (n.children.length > 0 ? [String(n.id), ...branches(n.children)] : []));
}

function count(nodes: Node[], type: string): number {
  return nodes.reduce((n, node) => n + (node.type.toLowerCase() === type.toLowerCase() ? 1 : 0) + count(node.children, type), 0);
}

/** One character per byte, the way the data engine sends them. */
function fromLatin1(text: string): Uint8Array {
  const out = new Uint8Array(text.length);
  for (let i = 0; i < text.length; i++) out[i] = text.charCodeAt(i) & 0xff;
  return out;
}
