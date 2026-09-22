/**
 * The absolute box around one live control. It is the only component subscribed to the
 * object, so a property write re-renders this control and nothing else; containers reach
 * their children through `renderChildren`, which nests another RuntimeControl per child.
 */

import type { ReactNode } from 'react';
import type { RuntimeObject } from '@shared/runtime/objectModel';
import { getDescriptor } from '@shared/registry';
import { runtimeRenderers } from './controls';
import { useRuntimeObject } from './useRuntimeObject';

export function RuntimeControl({ obj }: { obj: RuntimeObject }): ReactNode {
  const props = useRuntimeObject(obj);
  const type = obj.type;
  // A form is drawn by its window chrome, not as a control inside one.
  if (type === 'Form') return null;
  if (getDescriptor(type).nonVisual || props['Visible'] === false) return null;

  const width = Number(props['Width'] ?? 0);
  const height = Number(props['Height'] ?? 0);
  const Renderer = runtimeRenderers[type];
  const renderChildren = (parent: RuntimeObject): ReactNode => parent.children.map((c) => <RuntimeControl key={c.handle} obj={c} />);
  return (
    <div
      data-runtime-id={obj.handle}
      data-runtime-name={obj.name}
      style={{
        position: 'absolute',
        // `RightToLeft` says which way the control's own text reads, and nothing about what it
        // contains. A container must not pass it on: CSS `direction` cascades where the property
        // does not, and a bare PageFrame answers `.T.` in an ordinary English installation -
        // measured - so mirroring its children would turn most forms round.
        direction: props['RightToLeft'] === true && !getDescriptor(type).container ? 'rtl' : undefined,
        left: Number(props['Left'] ?? 0),
        top: Number(props['Top'] ?? 0),
        // A Line is a zero-thickness rectangle; it still needs a pixel to draw on.
        width: type === 'Line' ? Math.max(1, width) : width,
        height: type === 'Line' ? Math.max(1, height) : height,
        // AutoSize means the control is as big as what is written on it, and the Width the file
        // carries is only what the designer last measured. Fonts do not measure identically
        // twice, so honouring it is what keeps a caption from being cut off by a pixel: the
        // stored size becomes the least it may be rather than the most.
        ...(props['AutoSize'] === true && type !== 'Line'
          ? { width: 'max-content', minWidth: width, height: 'max-content', minHeight: height }
          : {}),
      }}
    >
      <Renderer obj={obj} props={props} renderChildren={renderChildren} />
    </div>
  );
}
