import { memo, useMemo, type ReactNode } from 'react';
import type { ControlNode } from '@shared/form/schema';
import { getDescriptor, resolveProps } from '@shared/registry';
import { getRect } from '../store/createFormDesignerStore';
import { useFormDesigner } from '../store/FormDesignerContext';
import { designSurfaces } from '../surfaces';
import { HANDLES } from '../geometry';
import { useCanvasInteractions } from './CanvasInteractionContext';

/** Absolute-positioned box for one control; draws its design surface and, when selected, the resize handles. */
export const ControlSurface = memo(function ControlSurface({ node }: { node: ControlNode }) {
  const selected = useFormDesigner((s) => s.selection.includes(node.id));
  const dragRect = useFormDesigner((s) => s.drag?.rects?.[node.id]);
  const interactions = useCanvasInteractions();
  const desc = getDescriptor(node.type);
  const resolved = useMemo(() => resolveProps(node), [node]);
  const rect = dragRect ?? getRect(node);
  const Surface = designSurfaces[node.type];
  const isLine = node.type === 'Line';

  const renderChildren = (parent: ControlNode): ReactNode => (parent.children ?? []).map((c) => <ControlSurface key={c.id} node={c} />);

  return (
    <div
      data-control-id={node.id}
      data-control-type={node.type}
      data-control-name={node.name}
      className={`fx-control${selected ? ' fx-selected' : ''}${resolved['Visible'] === false ? ' fx-hidden' : ''}`}
      style={{
        left: rect.left,
        top: rect.top,
        width: isLine ? Math.max(rect.width, 1) : rect.width,
        height: isLine ? Math.max(rect.height, 1) : rect.height,
      }}
      onPointerDown={(e) => interactions.onControlPointerDown(e, node)}
      onDoubleClick={(e) => interactions.onControlDoubleClick(e, node)}
    >
      <Surface node={node} resolved={resolved} renderChildren={renderChildren} />
      {selected &&
        HANDLES.filter((h) => !desc.nonVisual || h === 'se').map((h) => (
          <div
            key={h}
            className="fx-handle"
            data-handle={h}
            onPointerDown={(e) => {
              e.stopPropagation();
              interactions.onHandlePointerDown(e, node, h);
            }}
          />
        ))}
    </div>
  );
});
