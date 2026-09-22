import { useState, type ReactElement } from 'react';
import { Button, Field, Input, Select, Text, Toolbar, ToolbarDivider, Tooltip } from '@fluentui/react-components';
import { AddRegular, ArrowDownRegular, ArrowLeftRegular, ArrowRightRegular, ArrowUpRegular, DeleteRegular, LineHorizontal1Regular, SubtractRegular } from '@fluentui/react-icons';
import { MENU_LOCATIONS, type MenuLocation } from '@shared/menu/schema';
import type { MenuDesignerStore } from './store/createMenuDesignerStore';
import { MenuDesignerProvider, useMenuDesigner } from './store/MenuDesignerContext';
import { MenuItemTree } from './MenuItemTree';
import { MenuItemEditor } from './MenuItemEditor';
import { MenuPreview } from './MenuPreview';
import { TextEditor } from '../designer/properties/editors';

/** VFP Menu Designer: item tree, item editor, general options and a live preview. */
export function MenuDesignerDocument({ store }: { docId: string; store: MenuDesignerStore }) {
  return (
    <MenuDesignerProvider store={store}>
      <MenuDesignerBody />
    </MenuDesignerProvider>
  );
}

function MenuDesignerBody() {
  const doc = useMenuDesigner((s) => s.doc);
  const selectedId = useMenuDesigner((s) => s.selectedId);
  const st = useMenuDesigner((s) => s);
  const [last, setLast] = useState('');
  const sel = selectedId;
  const btn = (label: string, icon: ReactElement, onClick: () => void, disabled = false) => (
    <Tooltip content={label} relationship="label">
      <Button size="small" appearance="subtle" icon={icon} aria-label={label} onClick={onClick} disabled={disabled} />
    </Tooltip>
  );

  return (
    <div style={{ display: 'flex', flexDirection: 'column', flex: 1, minHeight: 0 }} data-testid="menu-designer">
      <Toolbar size="small" aria-label="Menu designer actions">
        {btn('Insert Item', <AddRegular />, () => st.insertItem())}
        {btn('Insert Separator', <LineHorizontal1Regular />, () => st.insertItem('separator'))}
        {btn('Insert Submenu Item', <SubtractRegular />, () => st.insertChild(sel), !sel)}
        {btn('Delete Item', <DeleteRegular />, () => sel && st.removeItem(sel), !sel)}
        <ToolbarDivider />
        {btn('Move Up', <ArrowUpRegular />, () => sel && st.moveItem(sel, -1), !sel)}
        {btn('Move Down', <ArrowDownRegular />, () => sel && st.moveItem(sel, 1), !sel)}
        {btn('Outdent', <ArrowLeftRegular />, () => sel && st.outdent(sel), !sel)}
        {btn('Indent', <ArrowRightRegular />, () => sel && st.indent(sel), !sel)}
      </Toolbar>
      <div style={{ padding: '4px 8px', borderBottom: '1px solid var(--colorNeutralStroke2)' }}>
        <MenuPreview items={doc.items} onChoose={(path) => setLast(path)} />
        <Text size={200} style={{ color: 'var(--colorNeutralForeground3)' }} data-testid="menu-last-choice">
          {last ? `Chosen: ${last}` : 'Click the preview to try the menu.'}
        </Text>
      </div>
      <div style={{ display: 'flex', flex: 1, minHeight: 0 }}>
        <div style={{ width: 260, borderRight: '1px solid var(--colorNeutralStroke2)', display: 'flex', flexDirection: 'column' }}>
          <MenuItemTree />
        </div>
        <div style={{ flex: 1, display: 'flex', flexDirection: 'column', minWidth: 0 }}>
          <MenuItemEditor />
          <GeneralOptions />
        </div>
      </div>
    </div>
  );
}

function GeneralOptions() {
  const doc = useMenuDesigner((s) => s.doc);
  const setDocField = useMenuDesigner((s) => s.setDocField);
  const setLocation = useMenuDesigner((s) => s.setLocation);
  return (
    <div style={{ borderTop: '1px solid var(--colorNeutralStroke2)', padding: 8, display: 'grid', gridTemplateColumns: '1fr 1fr', gap: 8 }} data-testid="menu-general-options">
      <Field label="Menu name" size="small">
        <TextEditor meta={{ name: 'Menu name', editor: 'text', category: 'Other', default: '' }} value={doc.name} onCommit={(v) => setDocField('name', String(v))} />
      </Field>
      <Field label="Location" size="small">
        <Select size="small" aria-label="Location" value={doc.location} onChange={(_e, d) => setLocation(d.value as MenuLocation)}>
          {MENU_LOCATIONS.map((l) => (
            <option key={l} value={l}>
              {l}
            </option>
          ))}
        </Select>
      </Field>
      <Field label="Setup code" size="small">
        <Input size="small" aria-label="Setup code" value={doc.setup ?? ''} onChange={(_e, d) => setDocField('setup', d.value)} />
      </Field>
      <Field label="Cleanup code" size="small">
        <Input size="small" aria-label="Cleanup code" value={doc.cleanup ?? ''} onChange={(_e, d) => setDocField('cleanup', d.value)} />
      </Field>
    </div>
  );
}
