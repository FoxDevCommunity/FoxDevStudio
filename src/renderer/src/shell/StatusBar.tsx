import { Text } from '@fluentui/react-components';
import { useStore } from 'zustand';
import type { FormDesignerStore } from '../designer/store/createFormDesignerStore';
import { getRect, getSelectedNodes } from '../designer/store/createFormDesignerStore';
import { docTitle, useDocumentsStore } from '../stores/documentsStore';
import { useProjectStore } from '../stores/projectStore';
import { useActiveDoc, useActiveFormDoc } from './activeDocument';
import { useSessionStore } from '../runtime/session';

export function StatusBar() {
  const project = useProjectStore((s) => s.doc?.name);
  const active = useActiveDoc();
  const docs = useDocumentsStore((s) => s.docs);
  const form = useActiveFormDoc();
  return (
    <div style={{ display: 'flex', gap: 16, padding: '2px 8px', borderTop: '1px solid var(--colorNeutralStroke2)', fontSize: 12, alignItems: 'center' }} data-testid="status-bar">
      <Text size={200}>{project ? `Project: ${project}` : 'No project'}</Text>
      {active && <Text size={200}>{docTitle(active, docs)}</Text>}
      {form && <SelectionStatus store={form.store} />}
      <span style={{ flex: 1 }} />
      <RunStatus />
    </div>
  );
}

/** What the runtime is doing: VFP shows this in the corner of the status bar. */
function RunStatus() {
  const status = useSessionStore((s) => s.status);
  const target = useSessionStore((s) => s.target);
  const label =
    status === 'running' ? `Running: ${target ?? ''}` : status === 'waiting' ? 'Waiting for events' : status === 'error' ? 'Program error' : 'Ready';
  return (
    <Text size={200} data-testid="run-status">
      {label}
    </Text>
  );
}

function SelectionStatus({ store }: { store: FormDesignerStore }) {
  const summary = useStore(store, (s) => {
    const nodes = getSelectedNodes(s);
    if (nodes.length === 0) return `${s.doc.form.name} (Form)`;
    if (nodes.length > 1) return `${nodes.length} objects selected`;
    const n = nodes[0]!;
    const r = getRect(n);
    return `${n.name} (${n.type})  ${r.left}, ${r.top}  ${r.width} x ${r.height}`;
  });
  return <Text size={200} data-testid="selection-status">{summary}</Text>;
}
