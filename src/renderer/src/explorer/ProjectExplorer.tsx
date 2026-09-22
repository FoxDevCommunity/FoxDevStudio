import { useMemo, useState, type ReactElement } from 'react';
import { Button, Menu, MenuItem, MenuList, MenuPopover, MenuTrigger, SearchBox, Text, Toolbar, Tooltip, Tree, TreeItem, TreeItemLayout } from '@fluentui/react-components';
import { AddRegular, DocumentRegular, FormRegular, NavigationRegular, TableRegular, DatabaseRegular, FolderRegular } from '@fluentui/react-icons';
import type { ProjectItem, ProjectItemKind } from '@shared/project/schema';
import { basename } from '@shared/paths';
import { fuzzyFilter, type FuzzyMatch } from '@shared/fuzzy';
import { getApi } from '../api/foxdev';
import { useProjectStore } from '../stores/projectStore';
import { FILTERS, openProjectItem } from '../stores/fileActions';
import { runCommand } from '../shell/commands/registry';
import './explorer.css';

const GROUPS: { kind: ProjectItemKind; label: string; icon: ReactElement }[] = [
  { kind: 'form', label: 'Forms', icon: <FormRegular /> },
  { kind: 'menu', label: 'Menus', icon: <NavigationRegular /> },
  { kind: 'program', label: 'Programs', icon: <DocumentRegular /> },
  { kind: 'class', label: 'Class Libraries', icon: <FolderRegular /> },
  { kind: 'database', label: 'Databases', icon: <DatabaseRegular /> },
  { kind: 'table', label: 'Free Tables', icon: <TableRegular /> },
  { kind: 'report', label: 'Reports', icon: <DocumentRegular /> },
  { kind: 'other', label: 'Other Files', icon: <DocumentRegular /> },
];

/** VFP Project Manager: items grouped by kind; double-click opens, right-click for actions. */
export function ProjectExplorer() {
  const doc = useProjectStore((s) => s.doc);
  const error = useProjectStore((s) => s.error);
  const [open, setOpen] = useState<string[]>(['form', 'menu', 'program']);
  const [query, setQuery] = useState('');

  // The path is matched, not just the file name, so `pgf/msg` finds controls/pgframe/msgbox.fxf.
  const matches = useMemo(() => fuzzyFilter(query.trim(), doc?.items ?? [], (i) => i.path), [query, doc]);
  const searching = query.trim() !== '';
  // a search opens every group that has a hit, so the answer is on screen without a click
  const matchedGroups = useMemo(() => [...new Set(matches.map((m) => m.item.kind))], [matches]);

  if (!doc) {
    return (
      <div className="fx-explorer" style={{ padding: 8 }} data-testid="project-explorer">
        <Text size={200}>No project open.</Text>
      </div>
    );
  }
  return (
    <div className="fx-explorer" style={{ display: 'flex', flexDirection: 'column', height: '100%', minHeight: 0 }} data-testid="project-explorer">
      <Toolbar size="small" aria-label="Project actions">
        <Tooltip content="New Form" relationship="label">
          <Button size="small" appearance="subtle" icon={<AddRegular />} aria-label="New Form" onClick={() => void runCommand('file.newForm')} />
        </Tooltip>
        <Tooltip content="Add existing file" relationship="label">
          <Button size="small" appearance="subtle" icon={<FolderRegular />} aria-label="Add File" onClick={() => void addExistingFile()} />
        </Tooltip>
        <Text size={100} weight="semibold" style={{ marginLeft: 6, textTransform: 'uppercase', letterSpacing: 0.4 }}>
          {doc.name}
        </Text>
      </Toolbar>
      {error && (
        <Text size={100} role="alert" style={{ padding: '0 8px', color: 'var(--colorPaletteRedForeground1)' }}>
          {error}
        </Text>
      )}
      <SearchBox
        size="small"
        placeholder="Search files"
        aria-label="Search files"
        value={query}
        onChange={(_e, d) => setQuery(d.value)}
        style={{ margin: '2px 4px 4px', width: 'auto' }}
      />
      <div style={{ overflow: 'auto', flex: 1 }}>
        <Tree aria-label="Project items" openItems={searching ? matchedGroups : open} onOpenChange={(_e, d) => setOpen([...d.openItems].map(String))}>
          {GROUPS.map((g) => {
            const items = matches.filter((m) => m.item.kind === g.kind);
            // while searching, a group with nothing in it is noise rather than a placeholder
            if (items.length === 0 && (searching || !['form', 'menu', 'program'].includes(g.kind))) return null;
            return (
              <TreeItem key={g.kind} itemType="branch" value={g.kind}>
                <TreeItemLayout iconBefore={g.icon}>
                  {g.label} ({items.length})
                </TreeItemLayout>
                <Tree>
                  {items.map(({ item, match }) => (
                    <ItemRow key={item.path} item={item} main={doc.main === item.path} match={match} />
                  ))}
                </Tree>
              </TreeItem>
            );
          })}
        </Tree>
      </div>
    </div>
  );
}

function ItemRow({ item, main, match }: { item: ProjectItem; main: boolean; match: FuzzyMatch }) {
  const setMain = useProjectStore((s) => s.setMain);
  const removeItem = useProjectStore((s) => s.removeItem);
  const setExcluded = useProjectStore((s) => s.setExcluded);
  const openIt = () => void openProjectItem(item.path).catch((e: Error) => useProjectStore.setState({ error: e.message }));
  const label = basename(item.path);
  // matches are positions in the whole path; only the ones inside the file name can be shown
  const from = item.path.length - label.length;
  const hits = match.positions.filter((p) => p >= from).map((p) => p - from);
  return (
    <Menu openOnContext>
      <MenuTrigger disableButtonEnhancement>
        <TreeItem itemType="leaf" value={item.path} data-item-path={item.path} onDoubleClick={openIt} onKeyDown={(e) => e.key === 'Enter' && openIt()}>
          <TreeItemLayout style={{ fontWeight: main ? 600 : undefined, opacity: item.excluded ? 0.6 : 1 }} aria-label={label} title={item.path}>
            <Highlighted text={label} at={hits} />
            {main ? '  (main)' : ''}
          </TreeItemLayout>
        </TreeItem>
      </MenuTrigger>
      <MenuPopover>
        <MenuList>
          <MenuItem onClick={openIt}>Open</MenuItem>
          <MenuItem onClick={() => setMain(main ? undefined : item.path)}>{main ? 'Clear Main' : 'Set Main'}</MenuItem>
          <MenuItem onClick={() => setExcluded(item.path, !item.excluded)}>{item.excluded ? 'Include' : 'Exclude'}</MenuItem>
          <MenuItem onClick={() => removeItem(item.path)}>Remove from Project</MenuItem>
        </MenuList>
      </MenuPopover>
    </Menu>
  );
}

/** The characters the search matched, marked so the reason a row is listed is visible. */
function Highlighted({ text, at }: { text: string; at: readonly number[] }) {
  if (at.length === 0) return <>{text}</>;
  const marked = new Set(at);
  return (
    <>
      {[...text].map((ch, i) =>
        marked.has(i) ? (
          <span key={i} style={{ fontWeight: 700, color: 'var(--colorBrandForeground1)' }}>
            {ch}
          </span>
        ) : (
          <span key={i}>{ch}</span>
        ),
      )}
    </>
  );
}

async function addExistingFile(): Promise<void> {
  const project = useProjectStore.getState();
  const path = await getApi().dialog.openFile({ title: 'Add File to Project', filters: FILTERS.any, defaultPath: project.dir() ?? undefined });
  if (path) project.addItem(path);
}
