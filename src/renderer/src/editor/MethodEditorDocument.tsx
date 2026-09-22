import { useEffect, useMemo, useRef } from 'react';
import { Select, Text } from '@fluentui/react-components';
import { useStore } from 'zustand';
import type { ControlNode, FormNode } from '@shared/form/schema';
import { allNodes, findLocation, findNode } from '@shared/form/tree';
import { getObjectDescriptor } from '@shared/registry';
import { FORM_ID, type FormDesignerStore } from '../designer/store/createFormDesignerStore';
import { useDocumentsStore } from '../stores/documentsStore';
import { breakpointGutter } from './breakpointGutter';
import { currentLine } from './currentLine';
import { CodeEditor } from './CodeEditor';

/** VFP code window: object and event pickers on top, the method source below. */
export function MethodEditorDocument({ docId }: { docId: string }) {
  const doc = useDocumentsStore((s) => s.docs[docId]);
  const formDoc = useDocumentsStore((s) => (doc?.kind === 'method' ? s.docs[doc.formDocId] : undefined));
  if (!doc || doc.kind !== 'method' || !formDoc || formDoc.kind !== 'form') return null;
  return <MethodEditorBody key={`${doc.controlId}/${doc.method}`} docId={docId} controlId={doc.controlId} method={doc.method} store={formDoc.store} />;
}

/**
 * Every keystroke writes straight into the form store inside one undo transaction, which closes
 * when the editor blurs or the tab switches target. The editor therefore always shows the store
 * text, and designer undo/redo (only possible while no transaction is open) flows back into it.
 */
function MethodEditorBody({ docId, controlId, method, store }: { docId: string; controlId: string; method: string; store: FormDesignerStore }) {
  const form = useStore(store, (s) => s.doc.form);
  const retarget = useDocumentsStore((s) => s.retargetMethod);
  const target: ControlNode | FormNode | undefined = controlId === FORM_ID ? form : findNode(form, controlId);
  const source = target?.methods[method] ?? '';
  const txnOpen = useRef(false);
  // a method's frames answer to `objPath.Event`, and the form's own methods to `Form.Event`;
  // the body is remounted whenever the pickers point it somewhere else, so this is stable
  const extensions = useMemo(() => {
    const named = () => `${controlId === FORM_ID ? form.name : objectPath(form, controlId)}.${method}`;
    return [breakpointGutter(named), currentLine(named)];
  }, [controlId, method, form]);

  const endTxn = () => {
    if (!txnOpen.current) return;
    txnOpen.current = false;
    store.getState().endTxn();
  };
  const onChange = (text: string) => {
    if (!txnOpen.current) {
      txnOpen.current = true;
      store.getState().beginTxn(`Edit ${method}`);
    }
    store.getState().setMethod(controlId, method, text);
  };
  useEffect(() => endTxn, []); // eslint-disable-line react-hooks/exhaustive-deps

  if (!target) {
    return (
      <div style={{ padding: 16 }}>
        <Text>The object this method belonged to no longer exists.</Text>
      </div>
    );
  }
  const desc = getObjectDescriptor(target);
  const event = desc.events.find((e) => e.name === method);
  const objects = [{ id: FORM_ID, label: form.name }, ...allNodes(form).map((n) => ({ id: n.id, label: `${'  '.repeat((findLocation(form, n.id)?.ancestors.length ?? 0) + 1)}${n.name}` }))];

  return (
    <div style={{ display: 'flex', flexDirection: 'column', flex: 1, minHeight: 0 }} data-testid="method-editor">
      <div className="fx-method-header">
        <Select
          size="small"
          aria-label="Object"
          value={controlId}
          onChange={(_e, d) => {
            endTxn();
            retarget(docId, d.value, defaultMethodFor(form, d.value, method));
          }}
        >
          {objects.map((o) => (
            <option key={o.id} value={o.id}>
              {o.label}
            </option>
          ))}
        </Select>
        <Select
          size="small"
          aria-label="Procedure"
          value={method}
          onChange={(_e, d) => {
            endTxn();
            retarget(docId, controlId, d.value);
          }}
        >
          {desc.events.map((e) => (
            <option key={e.name} value={e.name}>
              {e.name}
              {target.methods[e.name] ? ' *' : ''}
            </option>
          ))}
        </Select>
        <Text size={200} style={{ color: 'var(--colorNeutralForeground3)' }}>
          {event?.params ? `PARAMETERS ${event.params}` : ''}
        </Text>
      </div>
      <CodeEditor
        value={source}
        onChange={onChange}
        onBlur={endTxn}
        ariaLabel={`${method} source`}
        autoFocus
        languageContext={{ kind: 'method', params: event?.params }}
        extensions={extensions}
      />
    </div>
  );
}

/** A control's path from the form, dotted, which is how the runtime names its methods. */
function objectPath(form: FormNode, controlId: string): string {
  const at = findLocation(form, controlId);
  if (!at) return '';
  return [...at.ancestors, at.node].map((n) => n.name).join('.');
}

/** Keeps the same event when the new object has it, else falls back to its default event. */
function defaultMethodFor(form: FormNode, controlId: string, method: string): string {
  const target = controlId === FORM_ID ? form : findNode(form, controlId);
  if (!target) return method;
  const desc = getObjectDescriptor(target);
  return desc.events.some((e) => e.name === method) ? method : desc.defaultEvent;
}
