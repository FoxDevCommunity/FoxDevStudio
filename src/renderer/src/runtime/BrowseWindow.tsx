/**
 * The Browse window: the records of a work area in a grid, which is what `BROWSE` puts up.
 *
 * Visual FoxPro's Browse is the quickest way to see what is in a table, and a great many
 * programs end with one. The VM works the values out and hands the whole thing over, so nothing
 * here decodes a field; it draws what it was given, and says how much of the table it is.
 */

import { Button, Text } from '@fluentui/react-components';
import { DismissRegular } from '@fluentui/react-icons';
import type { BrowseTable } from '@shared/runtime/host';
import type { VmValue } from '@shared/runtime/values';
import { useSessionStore } from './session';

export function BrowseWindow({ browse }: { browse: BrowseTable }) {
  const close = (): void => useSessionStore.getState().closeBrowse(browse.alias);
  const shown = browse.rows.length;
  return (
    <div
      data-testid={`browse-${browse.alias}`}
      style={{
        border: '1px solid var(--colorNeutralStroke1)',
        borderRadius: 4,
        margin: 8,
        background: 'var(--colorNeutralBackground1)',
        boxShadow: 'var(--shadow8)',
        overflow: 'hidden',
      }}
    >
      <div
        style={{
          display: 'flex',
          alignItems: 'center',
          gap: 8,
          padding: '4px 8px',
          background: 'var(--colorNeutralBackground3)',
          fontWeight: 600,
        }}
      >
        <Text weight="semibold">{browse.title}</Text>
        <span style={{ flex: 1 }} />
        <Text size={200}>
          {shown === browse.count ? `${browse.count} record(s)` : `${shown} of ${browse.count} record(s)`}
        </Text>
        <Button appearance="subtle" size="small" icon={<DismissRegular />} aria-label="Close" onClick={close} />
      </div>
      <div style={{ maxHeight: 320, overflow: 'auto' }}>
        <table style={{ borderCollapse: 'collapse', width: '100%', fontSize: 12 }}>
          <thead>
            <tr>
              <th style={{ ...cell, position: 'sticky', top: 0, background: 'var(--colorNeutralBackground2)' }}>#</th>
              {browse.columns.map((column) => (
                <th
                  key={column.name}
                  style={{ ...cell, position: 'sticky', top: 0, background: 'var(--colorNeutralBackground2)', textAlign: 'left' }}
                >
                  {column.name}
                </th>
              ))}
            </tr>
          </thead>
          <tbody>
            {browse.rows.map((row) => (
              <tr key={row.recno} style={row.deleted ? { textDecoration: 'line-through', opacity: 0.6 } : undefined}>
                <td style={{ ...cell, color: 'var(--colorNeutralForeground3)' }}>{row.recno}</td>
                {row.values.map((value, i) => (
                  // a column is identified by its place, which is how the VM sent the values
                  <td key={browse.columns[i]?.name ?? i} style={{ ...cell, textAlign: numeric(browse.columns[i]?.kind) ? 'right' : 'left' }}>
                    {show(value)}
                  </td>
                ))}
              </tr>
            ))}
          </tbody>
        </table>
        {browse.rows.length === 0 && (
          <div style={{ padding: 12, color: 'var(--colorNeutralForeground3)' }}>
            <Text size={200}>No records.</Text>
          </div>
        )}
      </div>
    </div>
  );
}

const cell: React.CSSProperties = {
  border: '1px solid var(--colorNeutralStroke2)',
  padding: '2px 6px',
  whiteSpace: 'nowrap',
};

/** Whether a field's type is one that lines up on the right. */
function numeric(kind: string | undefined): boolean {
  return kind !== undefined && 'NFIBY'.includes(kind);
}

/** One value as the grid shows it; the VM already worked out what it is. */
function show(value: VmValue): string {
  if (value === null || value === undefined) return '.NULL.';
  if (typeof value === 'boolean') return value ? '.T.' : '.F.';
  if (typeof value === 'object') {
    if ('$date' in value) return value.$date;
    if ('$dt' in value) return new Date(value.$dt * 1000).toISOString().slice(0, 19).replace('T', ' ');
    return '';
  }
  return String(value);
}
