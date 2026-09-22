import type { PointerEvent as ReactPointerEvent } from 'react';

export interface DragCallbacks {
  /** Called after the pointer moved beyond the threshold. dx/dy are in client pixels from the start. */
  onMove?(dx: number, dy: number, e: PointerEvent): void;
  /** Called on pointer up. `moved` is false for a plain click. */
  onEnd?(dx: number, dy: number, e: PointerEvent, moved: boolean): void;
  onCancel?(): void;
  threshold?: number;
}

/**
 * Starts tracking a drag from a pointerdown event using window listeners.
 * Only clientX/clientY deltas are used, so it behaves identically under jsdom
 * (where layout is absent) and in the real renderer.
 */
export function startPointerDrag(e: ReactPointerEvent | PointerEvent, cb: DragCallbacks): void {
  const startX = e.clientX;
  const startY = e.clientY;
  const threshold = cb.threshold ?? 3;
  let moved = false;
  const target = e.currentTarget as Element | null;
  try {
    target?.setPointerCapture?.(e.pointerId);
  } catch {
    /* jsdom or detached element */
  }

  const onMove = (ev: PointerEvent) => {
    const dx = ev.clientX - startX;
    const dy = ev.clientY - startY;
    if (!moved && Math.abs(dx) < threshold && Math.abs(dy) < threshold) return;
    moved = true;
    cb.onMove?.(dx, dy, ev);
  };
  const cleanup = () => {
    window.removeEventListener('pointermove', onMove);
    window.removeEventListener('pointerup', onUp);
    window.removeEventListener('keydown', onKey);
    try {
      target?.releasePointerCapture?.(e.pointerId);
    } catch {
      /* ignore */
    }
  };
  const onUp = (ev: PointerEvent) => {
    cleanup();
    cb.onEnd?.(ev.clientX - startX, ev.clientY - startY, ev, moved);
  };
  const onKey = (ev: KeyboardEvent) => {
    if (ev.key === 'Escape') {
      cleanup();
      cb.onCancel?.();
    }
  };
  window.addEventListener('pointermove', onMove);
  window.addEventListener('pointerup', onUp);
  window.addEventListener('keydown', onKey);
}
