import { Menu, MenuButton, MenuDivider, MenuItem, MenuList, MenuPopover, MenuTrigger, Toolbar } from '@fluentui/react-components';
import { CheckmarkRegular } from '@fluentui/react-icons';
import { useState } from 'react';
import { docTitle, useDocumentsStore } from '../stores/documentsStore';
import { useProjectStore } from '../stores/projectStore';
import { useCommandRegistry, runCommand } from './commands/registry';

type Entry = string | '-' | { submenu: string; items: Entry[] };

const MENUS: { title: string; items: Entry[] }[] = [
  {
    title: 'File',
    items: ['file.newProject', 'file.openProject', 'file.closeProject', '-', 'file.newForm', 'file.newMenu', 'file.newProgram', 'file.open', '-', 'file.importVfp', '-', 'file.save', 'file.saveAs', 'file.saveAll', '-', 'file.close', 'file.closeAll', '-', { submenu: 'Recent Projects', items: [] }, '-', 'file.exit'],
  },
  { title: 'Edit', items: ['edit.undo', 'edit.redo', '-', 'edit.cut', 'edit.copy', 'edit.paste', 'edit.delete', '-', 'edit.selectAll'] },
  {
    title: 'Format',
    items: [
      { submenu: 'Align', items: ['format.align.left', 'format.align.right', 'format.align.top', 'format.align.bottom', 'format.align.centerH', 'format.align.centerV'] },
      { submenu: 'Size', items: ['format.size.width', 'format.size.height', 'format.size.both'] },
      { submenu: 'Spacing', items: ['format.distribute.horizontal', 'format.distribute.vertical'] },
      '-',
      'format.bringToFront',
      'format.sendToBack',
      '-',
      'format.snapToGrid',
      'format.showGrid',
      { submenu: 'Grid Size', items: ['format.grid.4', 'format.grid.8', 'format.grid.16'] },
    ],
  },
  { title: 'View', items: ['view.explorer', 'view.properties', 'view.output', 'view.debugger', '-', 'view.toggleTheme'] },
  { title: 'Program', items: ['program.run', 'program.doProgram', 'program.runMain', '-', 'program.cancel', '-', 'program.continue', 'program.stepInto', 'program.stepOver', 'program.stepOut', 'program.clearBreakpoints', '-', 'program.build', 'program.buildExe', '-', 'program.traceEvents'] },
  { title: 'Window', items: [] },
  { title: 'Help', items: ['help.about'] },
];

/** Fluent menu bar driven by the command registry. Enabled/checked states are evaluated when a menu opens. */
export function MenuBar() {
  return (
    <Toolbar size="small" aria-label="Main menu" style={{ padding: '0 4px', borderBottom: '1px solid var(--colorNeutralStroke2)' }}>
      {MENUS.map((m) => (
        <TopMenu key={m.title} title={m.title} items={m.items} />
      ))}
    </Toolbar>
  );
}

function TopMenu({ title, items }: { title: string; items: Entry[] }) {
  const [open, setOpen] = useState(false);
  return (
    <Menu open={open} onOpenChange={(_e, d) => setOpen(d.open)}>
      <MenuTrigger disableButtonEnhancement>
        <MenuButton appearance="transparent" size="small">
          {title}
        </MenuButton>
      </MenuTrigger>
      <MenuPopover>
        <MenuList>{title === 'Window' ? <WindowItems /> : items.map((it, i) => <Item key={i} entry={it} />)}</MenuList>
      </MenuPopover>
    </Menu>
  );
}

function Item({ entry }: { entry: Entry }) {
  useCommandRegistry((s) => s.version);
  if (entry === '-') return <MenuDivider />;
  if (typeof entry === 'object') {
    return (
      <Menu>
        <MenuTrigger disableButtonEnhancement>
          <MenuItem>{entry.submenu}</MenuItem>
        </MenuTrigger>
        <MenuPopover>
          <MenuList>{entry.submenu === 'Recent Projects' ? <RecentItems /> : entry.items.map((it, i) => <Item key={i} entry={it} />)}</MenuList>
        </MenuPopover>
      </Menu>
    );
  }
  const cmd = useCommandRegistry.getState().commands[entry];
  if (!cmd) return null;
  const enabled = cmd.isEnabled?.() ?? true;
  const checked = cmd.isChecked?.();
  return (
    <MenuItem
      disabled={!enabled}
      secondaryContent={cmd.shortcut}
      icon={cmd.isChecked ? <CheckmarkRegular style={{ visibility: checked ? 'visible' : 'hidden' }} /> : undefined}
      aria-checked={cmd.isChecked ? !!checked : undefined}
      onClick={() => void runCommand(cmd.id)}
      data-command={cmd.id}
    >
      {cmd.label}
    </MenuItem>
  );
}

function RecentItems() {
  const recent = useProjectStore((s) => s.recent);
  const openProject = useProjectStore((s) => s.openProject);
  if (recent.length === 0) return <MenuItem disabled>(none)</MenuItem>;
  return (
    <>
      {recent.map((p) => (
        <MenuItem key={p} onClick={() => void openProject(p)}>
          {p}
        </MenuItem>
      ))}
    </>
  );
}

function WindowItems() {
  const docs = useDocumentsStore((s) => s.docs);
  const order = useDocumentsStore((s) => s.order);
  const activate = useDocumentsStore((s) => s.activate);
  if (order.length === 0) return <MenuItem disabled>(no documents)</MenuItem>;
  return (
    <>
      {order.map((id) => (
        <MenuItem key={id} onClick={() => activate(id)}>
          {docTitle(docs[id]!, docs)}
        </MenuItem>
      ))}
    </>
  );
}
