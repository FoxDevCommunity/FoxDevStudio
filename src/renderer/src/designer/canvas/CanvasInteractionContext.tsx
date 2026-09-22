import { createContext, useContext, type PointerEvent as ReactPointerEvent } from 'react';
import type { ControlNode } from '@shared/form/schema';
import type { Handle } from '../geometry';

export interface CanvasInteractions {
  onControlPointerDown(e: ReactPointerEvent, node: ControlNode): void;
  onControlDoubleClick(e: ReactPointerEvent | React.MouseEvent, node: ControlNode): void;
  onHandlePointerDown(e: ReactPointerEvent, node: ControlNode, handle: Handle): void;
}

export const CanvasInteractionContext = createContext<CanvasInteractions | null>(null);

export function useCanvasInteractions(): CanvasInteractions {
  const ctx = useContext(CanvasInteractionContext);
  if (!ctx) throw new Error('ControlSurface must be rendered inside DesignerCanvas');
  return ctx;
}
