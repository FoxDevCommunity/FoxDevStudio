import { useMemo, useState } from 'react';
import { Input, Select, Tab, TabList, Text } from '@fluentui/react-components';
import type { ControlNode, FormNode, PropValue } from '@shared/form/schema';
import { allNodes, findLocation, findNode } from '@shared/form/tree';
import { getObjectDescriptor, isPropChanged, type PropCategory, type PropertyMeta } from '@shared/registry';
import { FORM_ID } from '../store/createFormDesignerStore';
import { useFormDesigner, useOptionalFormDesignerContext } from '../store/FormDesignerContext';
import { PropertyEditor, TextEditor } from './editors';
import { MethodsTab } from './MethodsTab';

type TabKey = 'all' | 'data' | 'methods' | 'layout' | 'other';
const TAB_CATEGORIES: Record<Exclude<TabKey, 'all' | 'methods'>, PropCategory[]> = {
  data: ['Data'],
  layout: ['Layout', 'Appearance'],
  other: ['Behavior', 'Other'],
};

/** VFP-style Properties window for the active form designer. Renders a placeholder without one. */
export function PropertiesWindow() {
  const ctx = useOptionalFormDesignerContext();
  if (!ctx) {
    return (
      <div className="fx-props" data-testid="properties-window">
        <div style={{ padding: 12 }}>
          <Text size={200}>No form is active.</Text>
        </div>
      </div>
    );
  }
  return <PropertiesBody />;
}

function PropertiesBody() {
  const form = useFormDesigner((s) => s.doc.form);
  const selection = useFormDesigner((s) => s.selection);
  const select = useFormDesigner((s) => s.select);
  const setProp = useFormDesigner((s) => s.setProp);
  const setName = useFormDesigner((s) => s.setName);
  const [tab, setTab] = useState<TabKey>('all');
  const [filter, setFilter] = useState('');

  const targets = useMemo<(ControlNode | FormNode)[]>(() => {
    const nodes = selection.map((id) => findNode(form, id)).filter((n): n is ControlNode => !!n);
    return nodes.length ? nodes : [form];
  }, [form, selection]);
  const targetIds = selection.length ? selection : [FORM_ID];

  /** Properties present on every selected object (intersection), with the shared value when all agree. */
  const rows = useMemo(() => {
    const first = getObjectDescriptor(targets[0]!);
    const out: { meta: PropertyMeta; value: PropValue | undefined; changed: boolean }[] = [];
    for (const meta of first.properties) {
      // what an object answers to but Visual FoxPro does not put in the property sheet
      if (meta.hidden) continue;
      if (!targets.every((t) => getObjectDescriptor(t).properties.some((p) => p.name === meta.name))) continue;
      const values = targets.map((t) => (meta.name in t.props ? t.props[meta.name] : meta.default));
      const same = values.every((v) => v === values[0]);
      out.push({ meta, value: same ? values[0] : undefined, changed: targets.some((t) => isPropChanged(t, meta.name)) });
    }
    return out;
  }, [targets]);

  const visibleRows = rows.filter((r) => {
    if (tab !== 'all' && tab !== 'methods' && !TAB_CATEGORIES[tab].includes(r.meta.category)) return false;
    return !filter || r.meta.name.toLowerCase().includes(filter.toLowerCase());
  });

  const objectOptions = useMemo(() => {
    const opts: { id: string; label: string }[] = [{ id: FORM_ID, label: form.name }];
    for (const n of allNodes(form)) {
      const depth = findLocation(form, n.id)?.ancestors.length ?? 0;
      // non-breaking spaces: an ordinary run of spaces would be collapsed in the option label
      opts.push({ id: n.id, label: `${'\u00a0\u00a0'.repeat(depth + 1)}${n.name}` });
    }
    return opts;
  }, [form]);

  const objectValue = selection.length === 1 ? selection[0]! : selection.length === 0 ? FORM_ID : '';
  const single = targets.length === 1 ? targets[0]! : null;

  return (
    <div className="fx-props" data-testid="properties-window">
      <div className="fx-props-toolbar">
        <Select
          size="small"
          aria-label="Object"
          value={objectValue}
          onChange={(_e, d) => select(d.value === FORM_ID ? [] : [d.value])}
        >
          {selection.length > 1 && <option value="">({selection.length} objects)</option>}
          {objectOptions.map((o) => (
            <option key={o.id} value={o.id}>
              {o.label}
            </option>
          ))}
        </Select>
        <TabList size="small" selectedValue={tab} onTabSelect={(_e, d) => setTab(d.value as TabKey)}>
          <Tab value="all">All</Tab>
          <Tab value="data">Data</Tab>
          <Tab value="methods">Methods</Tab>
          <Tab value="layout">Layout</Tab>
          <Tab value="other">Other</Tab>
        </TabList>
        {tab !== 'methods' && <Input size="small" aria-label="Filter properties" placeholder="Filter" value={filter} onChange={(_e, d) => setFilter(d.value)} />}
      </div>
      <div className="fx-props-body">
        {tab === 'methods' ? (
          single ? (
            <MethodsTab target={single} />
          ) : (
            <div style={{ padding: 8 }}>
              <Text size={200}>Select a single object to see its methods.</Text>
            </div>
          )
        ) : (
          <table>
            <tbody>
              {single && (
                <tr data-prop="Name">
                  <td className="fx-prop-name fx-changed">Name</td>
                  <td className="fx-prop-value">
                    <TextEditor
                      meta={{ name: 'Name', editor: 'text', category: 'Other', default: '' }}
                      value={single.name}
                      onCommit={(v) => setName('type' in single ? single.id : FORM_ID, String(v))}
                    />
                  </td>
                </tr>
              )}
              {visibleRows.map(({ meta, value, changed }) => (
                <tr key={meta.name} data-prop={meta.name}>
                  <td className={`fx-prop-name${changed ? ' fx-changed' : ''}`} title={meta.description}>
                    {meta.name}
                  </td>
                  <td className="fx-prop-value">
                    <PropertyEditor meta={meta} value={value} onCommit={(v) => setProp(targetIds, meta.name, v)} />
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        )}
      </div>
    </div>
  );
}
