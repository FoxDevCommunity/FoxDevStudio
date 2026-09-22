import { useState } from 'react';
import { Tree, TreeItem, TreeItemLayout } from '@fluentui/react-components';
import type { MenuItem } from '@shared/menu/schema';
import { isSeparator } from '@shared/menu/schema';
import { promptToLabel } from './hotkey';
import { useMenuDesigner } from './store/MenuDesignerContext';

/** Left pane: pads and bars as a tree; click selects. */
export function MenuItemTree() {
  const items = useMenuDesigner((s) => s.doc.items);
  const selectedId = useMenuDesigner((s) => s.selectedId);
  const select = useMenuDesigner((s) => s.select);
  const [closed, setClosed] = useState<Set<string>>(new Set());
  const allBranches = collectBranches(items);
  const openItems = allBranches.filter((id) => !closed.has(id));

  const renderItems = (list: MenuItem[]) =>
    list.map((item) => {
      const branch = !!item.children && item.result.type === 'submenu';
      const { label } = promptToLabel(item.prompt);
      const text = isSeparator(item) ? '────────' : label || '(empty prompt)';
      return (
        <TreeItem
          key={item.id}
          itemType={branch ? 'branch' : 'leaf'}
          value={item.id}
          data-menu-id={item.id}
          aria-selected={item.id === selectedId}
          onClick={(e) => {
            e.stopPropagation();
            select(item.id);
          }}
        >
          <TreeItemLayout style={{ fontWeight: item.id === selectedId ? 700 : undefined, opacity: item.enabled === false ? 0.6 : 1 }}>{text}</TreeItemLayout>
          {branch && <Tree>{renderItems(item.children!)}</Tree>}
        </TreeItem>
      );
    });

  return (
    <Tree
      aria-label="Menu items"
      openItems={openItems}
      onOpenChange={(_e, d) => setClosed(new Set(allBranches.filter((id) => !d.openItems.has(id))))}
      style={{ overflow: 'auto', flex: 1 }}
    >
      {renderItems(items)}
    </Tree>
  );
}

function collectBranches(items: MenuItem[]): string[] {
  const out: string[] = [];
  const walk = (list: MenuItem[]) => {
    for (const i of list) {
      if (i.children && i.result.type === 'submenu') {
        out.push(i.id);
        walk(i.children);
      }
    }
  };
  walk(items);
  return out;
}
