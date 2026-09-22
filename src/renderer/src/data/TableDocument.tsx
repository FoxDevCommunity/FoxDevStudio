/**
 * Which editor a DBF-based file gets.
 *
 * A `.dbf` is a table and gets a grid. A `.dbc` is also a table, but its records describe a tree
 * - a database owns tables, a table owns its fields, indexes and relations - so it gets the
 * database designer, with the grid one click away for when the records themselves are what you
 * came for.
 */

import { useState } from 'react';
import { extname } from '@shared/paths';
import { TableBrowserDocument } from './TableBrowser';
import { DatabaseDesignerDocument } from './DatabaseDesigner';

export function TableDocument({ path }: { path: string }) {
  const isContainer = extname(path).toLowerCase() === '.dbc';
  const [showRecords, setShowRecords] = useState(false);

  if (!isContainer || showRecords) {
    return <TableBrowserDocument path={path} onShowTree={isContainer ? () => setShowRecords(false) : undefined} />;
  }
  return <DatabaseDesignerDocument path={path} onShowRecords={() => setShowRecords(true)} />;
}
