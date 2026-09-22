/**
 * Every object and method of the form being designed, in one searchable list.
 *
 * A Visual FoxPro form of any size has its code spread across dozens of objects, and the
 * properties window only shows one at a time. This is the other view: the whole document at once,
 * so `valid` finds every Valid method in the form and `cmd` every command button, and clicking
 * one goes there: an object selects on the canvas, a method opens its editor.
 */

import { useMemo, useState, type FC } from 'react';
import { SearchBox, Text, Tree, TreeItem, TreeItemLayout } from '@fluentui/react-components';
import type { ControlNode, ControlType, FormNode } from '@shared/form/schema';
import { getDescriptor, type ToolboxGroup } from '@shared/registry';
import { allNodes } from '@shared/form/tree';
import { fuzzyFilter } from '@shared/fuzzy';
import { useDocumentsStore } from '../../stores/documentsStore';
import { FORM_ID } from '../store/createFormDesignerStore';
import { useFormDesigner, useFormDesignerContext, useOptionalFormDesignerContext } from '../store/FormDesignerContext';
import { resolveIcon } from '../icons';

const MethodIcon = resolveIcon('Code');
const FormIcon = resolveIcon('Window');

/**
 * A colour per family of control, so the list is scannable at a glance rather than a wall of
 * identical grey glyphs. Fluent's palette tokens are used so both themes stay legible.
 */
const GROUP_COLOUR: Record<ToolboxGroup, string> = {
  Standard: 'var(--colorPaletteBlueForeground2)',
  Container: 'var(--colorPaletteMarigoldForeground2)',
  Data: 'var(--colorPaletteGreenForeground2)',
  Other: 'var(--colorPalettePurpleForeground2)',
};
const FORM_COLOUR = 'var(--colorPaletteTealForeground2)';
const METHOD_COLOUR = 'var(--colorPaletteGreenForeground2)';

/** The icon and colour for one row. */
function iconFor(symbol: Symbol): { Icon: FC; colour: string } {
  if (symbol.method) return { Icon: MethodIcon, colour: METHOD_COLOUR };
  if (symbol.type === 'Form') return { Icon: FormIcon, colour: FORM_COLOUR };
  const descriptor = getDescriptor(symbol.type as ControlType);
  return { Icon: resolveIcon(descriptor.icon), colour: GROUP_COLOUR[descriptor.toolboxGroup] };
}

/** One line of the list: an object, or a method that belongs to one. */
interface Symbol {
  /** Id of the object, or of the object the method belongs to. */
  id: string;
  object: string;
  /** Absent for the object's own row. */
  method?: string;
  type: string;
  /** What a search matches against. */
  search: string;
}

export function SymbolExplorer() {
  const ctx = useOptionalFormDesignerContext();
  if (!ctx) return null;
  return <SymbolBody />;
}

function SymbolBody() {
  const form = useFormDesigner((s) => s.doc.form);
  const select = useFormDesigner((s) => s.select);
  const { docId } = useFormDesignerContext();
  const [query, setQuery] = useState('');

  const symbols = useMemo(() => collect(form), [form]);
  const matches = useMemo(() => fuzzyFilter(query.trim(), symbols, (s) => s.search), [query, symbols]);

  const go = (symbol: Symbol) => {
    select([symbol.id === FORM_ID ? FORM_ID : symbol.id]);
    if (symbol.method) useDocumentsStore.getState().openMethod(docId, symbol.id, symbol.method);
  };

  return (
    <div className="fx-symbols" data-testid="symbol-explorer">
      <div className="fx-symbols__head">
        <Text size={100} weight="semibold" style={{ textTransform: 'uppercase', letterSpacing: 0.4 }}>
          Symbols
        </Text>
        <SearchBox
          size="small"
          placeholder="Search objects and methods"
          aria-label="Search symbols"
          value={query}
          onChange={(_e, d) => setQuery(d.value)}
          style={{ width: 'auto' }}
        />
      </div>
      <div className="fx-symbols__list">
        {matches.length === 0 ? (
          <Text size={100} style={{ padding: '4px 8px', color: 'var(--colorNeutralForeground3)' }}>
            Nothing matches.
          </Text>
        ) : (
          <Tree aria-label="Symbols">
            {matches.map(({ item }) => {
              const { Icon, colour } = iconFor(item);
              return (
              <TreeItem
                key={`${item.id}.${item.method ?? ''}`}
                itemType="leaf"
                value={`${item.id}.${item.method ?? ''}`}
                data-symbol={item.search}
              >
                <TreeItemLayout
                  iconBefore={<span style={{ color: colour, display: 'inline-flex' }}><Icon /></span>}
                  aria-label={item.search}
                  onClick={() => go(item)}
                  onKeyDown={(e) => e.key === 'Enter' && go(item)}
                >
                  {item.method ? (
                    <>
                      <span style={{ color: 'var(--colorNeutralForeground3)' }}>{item.object}.</span>
                      {item.method}
                    </>
                  ) : (
                    <>
                      {item.object}
                      <span style={{ color: 'var(--colorNeutralForeground3)' }}> {item.type}</span>
                    </>
                  )}
                </TreeItemLayout>
              </TreeItem>
              );
            })}
          </Tree>
        )}
      </div>
    </div>
  );
}

/** The form, every control under it, and the methods each one carries, in document order. */
function collect(form: FormNode): Symbol[] {
  const out: Symbol[] = [];
  const add = (id: string, node: ControlNode | FormNode, type: string) => {
    out.push({ id, object: node.name, type, search: node.name });
    for (const method of Object.keys(node.methods)) {
      out.push({ id, object: node.name, method, type, search: `${node.name}.${method}` });
    }
  };

  add(FORM_ID, form, 'Form');
  for (const node of allNodes(form)) add(node.id, node, node.type);
  return out;
}
