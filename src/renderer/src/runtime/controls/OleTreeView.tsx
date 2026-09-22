/**
 * Draws the TreeView that stands in for the ActiveX one.
 *
 * The nodes live on the control's `TreeView` object, where FoxPro code put them; this only
 * reads them. Clicking one selects it and dispatches `NodeClick` with the node as its
 * parameter, which is the signature VFP's control has and the samples are written against.
 */

import type { CSSProperties, FC } from 'react';
import { ChevronDownRegular, ChevronRightRegular } from '@fluentui/react-icons';
import type { RuntimeObject } from '@shared/runtime/objectModel';
import type { TreeNode, TreeView } from '@shared/runtime/oleObjects';

const row: CSSProperties = {
  display: 'flex',
  alignItems: 'center',
  gap: 2,
  height: 20,
  padding: '0 4px',
  cursor: 'default',
  whiteSpace: 'nowrap',
  userSelect: 'none',
};

export const OleTreeView: FC<{ obj: RuntimeObject; tree: TreeView }> = ({ obj, tree }) => {
  const dispatch = (node: TreeNode, event: string) => {
    obj.desktop.dispatch(obj, event, [{ $obj: obj.desktop.hostHandle(node) }]);
  };

  const select = (node: TreeNode) => {
    tree.select(node);
    obj.notifyChanged();
    dispatch(node, 'NodeClick');
  };

  const toggle = (node: TreeNode) => {
    const open = tree.toggle(node);
    obj.notifyChanged();
    dispatch(node, open ? 'Expand' : 'Collapse');
  };

  return (
    <div
      style={{
        width: '100%',
        height: '100%',
        boxSizing: 'border-box',
        overflow: 'auto',
        background: 'var(--colorNeutralBackground1)',
        border: '1px solid var(--colorNeutralStroke1)',
        fontSize: 12,
      }}
      aria-label={obj.name}
      role="tree"
    >
      {tree.visible().map(({ node, depth }) => {
        const selected = tree.selected === node;
        return (
          <div
            key={`${node.index}:${node.key}`}
            role="treeitem"
            aria-selected={selected}
            aria-expanded={node.children.length > 0 ? node.expanded : undefined}
            style={{
              ...row,
              paddingLeft: 4 + depth * 16,
              fontWeight: node.bold ? 600 : 400,
              background: selected ? 'var(--colorNeutralBackground1Selected)' : undefined,
            }}
            onClick={() => select(node)}
            onDoubleClick={() => obj.desktop.dispatch(obj, 'DblClick')}
          >
            <span
              style={{ width: 14, display: 'inline-flex', justifyContent: 'center', color: 'var(--colorNeutralForeground3)' }}
              onClick={(e) => {
                e.stopPropagation();
                if (node.children.length > 0) toggle(node);
              }}
            >
              {node.children.length > 0 ? node.expanded ? <ChevronDownRegular fontSize={12} /> : <ChevronRightRegular fontSize={12} /> : null}
            </span>
            {node.text}
          </div>
        );
      })}
    </div>
  );
};
