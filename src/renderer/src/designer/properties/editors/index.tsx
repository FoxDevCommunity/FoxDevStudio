import { useState, type KeyboardEvent } from 'react';
import { Button, Input, Select, Textarea } from '@fluentui/react-components';
import type { PropValue } from '@shared/form/schema';
import type { PropertyMeta } from '@shared/registry';
import { formatColor, parseColor, toHex, fromHex } from '@shared/form/color';
import { relative } from '@shared/paths';
import { getApi } from '../../../api/foxdev';
import { useProjectStore } from '../../../stores/projectStore';

export interface EditorProps {
  meta: PropertyMeta;
  /** undefined = mixed values across a multi-selection. */
  value: PropValue | undefined;
  onCommit(value: PropValue): void;
}

const MIXED = '';

/** Text-like editor: local draft that resets whenever the external value changes. */
function useDraft(external: string) {
  const [draft, setDraft] = useState(external);
  const [prev, setPrev] = useState(external);
  if (prev !== external) {
    setPrev(external);
    setDraft(external);
  }
  return [draft, setDraft] as const;
}

function commitKeys(commit: () => void, revert: () => void) {
  return (e: KeyboardEvent<HTMLElement>) => {
    if (e.key === 'Enter') {
      e.preventDefault();
      commit();
    } else if (e.key === 'Escape') {
      e.preventDefault();
      revert();
    }
  };
}

export function TextEditor({ meta, value, onCommit, type = 'text' }: EditorProps & { type?: 'text' | 'number' }) {
  const external = value === undefined ? MIXED : String(value);
  const [draft, setDraft] = useDraft(external);
  const commit = () => {
    if (draft === external) return;
    if (type === 'number') {
      const n = Number(draft);
      if (draft.trim() === '' || !Number.isFinite(n)) return setDraft(external);
      let v = n;
      if (meta.min !== undefined) v = Math.max(meta.min, v);
      if (meta.max !== undefined) v = Math.min(meta.max, v);
      onCommit(v);
    } else onCommit(draft);
  };
  return (
    <Input
      size="small"
      type={type}
      aria-label={meta.name}
      value={draft}
      placeholder={value === undefined ? '(mixed)' : meta.editor === 'expression' ? 'expression' : undefined}
      readOnly={meta.readOnly}
      onChange={(_e, d) => setDraft(d.value)}
      onBlur={commit}
      onKeyDown={commitKeys(commit, () => setDraft(external))}
      style={{ width: '100%' }}
    />
  );
}

export function MultilineEditor({ meta, value, onCommit }: EditorProps) {
  const external = value === undefined ? MIXED : String(value);
  const [draft, setDraft] = useDraft(external);
  return (
    <Textarea
      size="small"
      aria-label={meta.name}
      value={draft}
      rows={2}
      onChange={(_e, d) => setDraft(d.value)}
      onBlur={() => draft !== external && onCommit(draft)}
      style={{ width: '100%' }}
    />
  );
}

export function BooleanEditor({ meta, value, onCommit }: EditorProps) {
  return (
    <Select size="small" aria-label={meta.name} value={value === undefined ? '' : value ? 'T' : 'F'} onChange={(_e, d) => onCommit(d.value === 'T')}>
      {value === undefined && <option value="">(mixed)</option>}
      <option value="T">.T. - True</option>
      <option value="F">.F. - False</option>
    </Select>
  );
}

export function EnumEditor({ meta, value, onCommit }: EditorProps) {
  const values = meta.enumValues ?? [];
  return (
    <Select
      size="small"
      aria-label={meta.name}
      value={value === undefined ? '' : String(value)}
      onChange={(_e, d) => {
        const hit = values.find((v) => String(v.value) === d.value);
        if (hit) onCommit(hit.value);
      }}
    >
      {value === undefined && <option value="">(mixed)</option>}
      {values.map((v) => (
        <option key={String(v.value)} value={String(v.value)}>
          {v.label}
        </option>
      ))}
    </Select>
  );
}

export function ColorEditor({ meta, value, onCommit }: EditorProps) {
  const external = typeof value === 'number' ? formatColor(value) : MIXED;
  const [draft, setDraft] = useDraft(external);
  const commit = () => {
    if (draft === external) return;
    const parsed = parseColor(draft);
    if (parsed === null) setDraft(external);
    else onCommit(parsed);
  };
  return (
    <div style={{ display: 'flex', alignItems: 'center' }}>
      <input
        type="color"
        aria-label={`${meta.name} swatch`}
        value={typeof value === 'number' ? toHex(value) : '#000000'}
        onChange={(e) => {
          const n = fromHex(e.target.value);
          if (n !== null) onCommit(n);
        }}
      />
      <Input size="small" aria-label={meta.name} value={draft} onChange={(_e, d) => setDraft(d.value)} onBlur={commit} onKeyDown={commitKeys(commit, () => setDraft(external))} style={{ flex: 1 }} />
    </div>
  );
}

export const COMMON_FONTS = ['Arial', 'Calibri', 'Consolas', 'Courier New', 'Georgia', 'Segoe UI', 'Tahoma', 'Times New Roman', 'Verdana'];

export function FontEditor(props: EditorProps) {
  return (
    <>
      <TextEditor {...props} />
      <datalist id="fx-font-list">
        {COMMON_FONTS.map((f) => (
          <option key={f} value={f} />
        ))}
      </datalist>
    </>
  );
}

export function PictureEditor(props: EditorProps) {
  const pick = async () => {
    const chosen = await getApi().dialog.openFile({
      title: `Select ${props.meta.name}`,
      filters: [{ name: 'Images', extensions: ['bmp', 'png', 'jpg', 'jpeg', 'gif', 'ico'] }],
    });
    if (!chosen) return;
    const dir = useProjectStore.getState().dir();
    props.onCommit(dir ? relative(dir, chosen) : chosen);
  };
  return (
    <div style={{ display: 'flex', gap: 2 }}>
      <div style={{ flex: 1, minWidth: 0 }}>
        <TextEditor {...props} />
      </div>
      <Button size="small" aria-label={`Browse ${props.meta.name}`} onClick={pick}>
        ...
      </Button>
    </div>
  );
}

/** Picks the editor component for a property. */
export function PropertyEditor(props: EditorProps) {
  switch (props.meta.editor) {
    case 'boolean':
      return <BooleanEditor {...props} />;
    case 'enum':
      return <EnumEditor {...props} />;
    case 'number':
      return <TextEditor {...props} type="number" />;
    case 'color':
      return <ColorEditor {...props} />;
    case 'multiline':
      return <MultilineEditor {...props} />;
    case 'font':
      return <FontEditor {...props} />;
    case 'picture':
      return <PictureEditor {...props} />;
    default:
      return <TextEditor {...props} />;
  }
}
