import { useState } from 'react';
import { Button, Field, Input, Select, Switch, Text } from '@fluentui/react-components';
import { MENU_RESULT_TYPES, type MenuResultType } from '@shared/menu/schema';
import { findMenuItem } from '@shared/menu/tree';
import { TextEditor } from '../designer/properties/editors';
import { CodeEditor } from '../editor/CodeEditor';
import { formatHotkey, hotkeyFromEvent } from './hotkey';
import { useMenuDesigner } from './store/MenuDesignerContext';

const RESULT_LABELS: Record<MenuResultType, string> = { submenu: 'Submenu', command: 'Command', procedure: 'Procedure', pad: 'Pad Name', bar: 'Bar #' };

const meta = (name: string) => ({ name, editor: 'text' as const, category: 'Other' as const, default: '' });

/** Right pane: VFP's per-item fields (Prompt, Result, Options). */
export function MenuItemEditor() {
  const selectedId = useMenuDesigner((s) => s.selectedId);
  const item = useMenuDesigner((s) => (s.selectedId ? findMenuItem(s.doc.items, s.selectedId)?.item : undefined));
  const setPrompt = useMenuDesigner((s) => s.setPrompt);
  const setResult = useMenuDesigner((s) => s.setResult);
  const setItemField = useMenuDesigner((s) => s.setItemField);
  const setHotkey = useMenuDesigner((s) => s.setHotkey);
  const setEnabled = useMenuDesigner((s) => s.setEnabled);
  const [capturing, setCapturing] = useState(false);

  if (!item || !selectedId) {
    return (
      <div style={{ padding: 12 }}>
        <Text size={200}>Select a menu item to edit it.</Text>
      </div>
    );
  }
  const id = selectedId;
  return (
    <div style={{ padding: 8, display: 'flex', flexDirection: 'column', gap: 8, overflow: 'auto', flex: 1 }} data-testid="menu-item-editor">
      <Field label="Prompt" hint="Use \< before the hotkey letter, \- for a separator" size="small">
        <TextEditor meta={meta('Prompt')} value={item.prompt} onCommit={(v) => setPrompt(id, String(v))} />
      </Field>
      <Field label="Result" size="small">
        <Select size="small" aria-label="Result" value={item.result.type} onChange={(_e, d) => setResult(id, d.value as MenuResultType, item.result.text)}>
          {MENU_RESULT_TYPES.map((t) => (
            <option key={t} value={t}>
              {RESULT_LABELS[t]}
            </option>
          ))}
        </Select>
      </Field>
      {item.result.type === 'procedure' ? (
        <Field label="Procedure code" size="small">
          <div style={{ height: 160, display: 'flex', border: '1px solid var(--colorNeutralStroke1)' }}>
            <CodeEditor value={item.result.text ?? ''} onChange={(v) => setResult(id, 'procedure', v)} ariaLabel="Procedure code" />
          </div>
        </Field>
      ) : item.result.type !== 'submenu' ? (
        <Field label={item.result.type === 'command' ? 'Command' : item.result.type === 'pad' ? 'System pad name' : 'Bar number'} size="small">
          <TextEditor meta={meta('Result text')} value={item.result.text ?? ''} onCommit={(v) => setResult(id, item.result.type, String(v))} />
        </Field>
      ) : null}
      <Field label="Name" size="small">
        <TextEditor meta={meta('Name')} value={item.name ?? ''} onCommit={(v) => setItemField(id, 'name', String(v))} />
      </Field>
      <Field label="Shortcut" size="small">
        <div style={{ display: 'flex', gap: 4 }}>
          <Input
            size="small"
            aria-label="Shortcut"
            readOnly
            value={capturing ? 'Press keys...' : formatHotkey(item.hotkey)}
            onFocus={() => setCapturing(true)}
            onBlur={() => setCapturing(false)}
            onKeyDown={(e) => {
              e.preventDefault();
              const hk = hotkeyFromEvent(e);
              if (hk) {
                setHotkey(id, hk);
                setCapturing(false);
              }
            }}
          />
          <Button size="small" aria-label="Clear shortcut" onClick={() => setHotkey(id, undefined)}>
            Clear
          </Button>
        </div>
      </Field>
      <Field label="Skip For" size="small">
        <TextEditor meta={{ ...meta('Skip For'), editor: 'expression' }} value={item.skipFor ?? ''} onCommit={(v) => setItemField(id, 'skipFor', String(v))} />
      </Field>
      <Field label="Message" size="small">
        <TextEditor meta={meta('Message')} value={item.message ?? ''} onCommit={(v) => setItemField(id, 'message', String(v))} />
      </Field>
      <Switch label="Enabled" checked={item.enabled !== false} onChange={(_e, d) => setEnabled(id, d.checked)} />
    </div>
  );
}
