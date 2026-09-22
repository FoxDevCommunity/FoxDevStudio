import { useMemo, useRef, type KeyboardEvent, type MouseEvent, type PointerEvent as ReactPointerEvent } from 'react';
import type { ControlNode, ControlType } from '@shared/form/schema';
import { findNode, findLocation } from '@shared/form/tree';
import { FORM_DESCRIPTOR, getDescriptor, getProp } from '@shared/registry';
import { computeMove, computeResize, normalizeRect, snap, snapRect, type Handle, type Rect } from '../geometry';
import { FORM_ID, getRect, getSelectedNodes } from '../store/createFormDesignerStore';
import { useFormDesigner, useFormDesignerContext } from '../store/FormDesignerContext';
import { useSettingsStore } from '../../stores/settingsStore';
import { useDocumentsStore } from '../../stores/documentsStore';
import { colorCss } from '../surfaces';
import { startPointerDrag } from './usePointerDrag';
import { CanvasInteractionContext, type CanvasInteractions } from './CanvasInteractionContext';
import { ControlSurface } from './ControlSurface';

const MIN_SIZE = { width: 4, height: 4 };

/** The form being designed: title bar, client area with grid, controls, marquee. Handles all pointer/keyboard work. */
export function DesignerCanvas() {
  const { store, docId } = useFormDesignerContext();
  const form = useFormDesigner((s) => s.doc.form);
  const tool = useFormDesigner((s) => s.tool);
  const marquee = useFormDesigner((s) => s.drag?.marquee);
  const gridSize = useSettingsStore((s) => s.gridSize);
  const snapToGrid = useSettingsStore((s) => s.snapToGrid);
  const showGrid = useSettingsStore((s) => s.showGrid);
  const clientRef = useRef<HTMLDivElement>(null);

  const width = Number(getProp(form, 'Width'));
  const height = Number(getProp(form, 'Height'));
  const snapOpts = { grid: gridSize, snap: snapToGrid };

  const openMethod = (controlId: string, method: string) => useDocumentsStore.getState().openMethod(docId, controlId, method);

  const interactions = useMemo<CanvasInteractions>(
    () => ({
      onControlPointerDown(e, node) {
        const s = store.getState();
        if (s.tool) return; // let the click bubble to the canvas, which places the new control
        if (e.button !== 0) return;
        e.stopPropagation();
        const multi = e.shiftKey || e.ctrlKey || e.metaKey;
        const wasSelected = s.selection.includes(node.id);
        if (multi) s.select([node.id], 'toggle');
        else if (!wasSelected) s.select([node.id]);
        const ids = store.getState().selection;
        if (!ids.includes(node.id)) return;
        const movable = getSelectedNodes(store.getState()).filter((n) => getDescriptor(n.type).properties.some((p) => p.name === 'Left'));
        if (movable.length === 0) return;
        const startRects = movable.map((n) => getRect(n));
        startPointerDrag(e, {
          onMove(dx, dy) {
            const rects = computeMove(startRects, dx, dy, snapOpts);
            store.getState().setDrag({ kind: 'move', rects: Object.fromEntries(movable.map((n, i) => [n.id, rects[i]!])) });
          },
          onEnd(dx, dy, _ev, moved) {
            const st = store.getState();
            st.setDrag(null);
            if (!moved) return;
            const rects = computeMove(startRects, dx, dy, snapOpts);
            st.setRects(Object.fromEntries(movable.map((n, i) => [n.id, rects[i]!])), movable.length === 1 ? `Move ${movable[0]!.name}` : 'Move controls');
          },
          onCancel: () => store.getState().setDrag(null),
        });
      },

      onControlDoubleClick(e, node) {
        e.stopPropagation();
        openMethod(node.id, getDescriptor(node.type).defaultEvent);
      },

      onHandlePointerDown(e, node, handle: Handle) {
        if (e.button !== 0) return;
        const start = getRect(node);
        const opts = { ...snapOpts, min: MIN_SIZE, aspect: node.type === 'Line' ? ('line' as const) : undefined };
        startPointerDrag(e, {
          onMove(dx, dy) {
            store.getState().setDrag({ kind: 'resize', rects: { [node.id]: computeResize(start, handle, dx, dy, opts) } });
          },
          onEnd(dx, dy, _ev, moved) {
            const st = store.getState();
            st.setDrag(null);
            if (moved) st.setRects({ [node.id]: computeResize(start, handle, dx, dy, opts) }, `Resize ${node.name}`);
          },
          onCancel: () => store.getState().setDrag(null),
        });
      },
    }),
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [store, gridSize, snapToGrid, docId],
  );

  /** Pointer down on the form background or, in tool mode, anywhere: place a control or start a marquee. */
  const onClientPointerDown = (e: ReactPointerEvent<HTMLDivElement>) => {
    if (e.button !== 0) return;
    const s = store.getState();
    const target = e.target as HTMLElement;
    if (s.tool) {
      const type = s.tool;
      const containerEl = target.closest<HTMLElement>('[data-container-id]') ?? clientRef.current!;
      const parentId = containerEl.dataset['containerId'] || null;
      const origin = containerEl.getBoundingClientRect();
      const startX = e.clientX - origin.left;
      const startY = e.clientY - origin.top;
      e.stopPropagation();
      startPointerDrag(e, {
        onMove(dx, dy) {
          const r = snapRect(normalizeRect(startX, startY, startX + dx, startY + dy), gridSize, snapToGrid);
          store.getState().setDrag({ kind: 'create', marquee: parentId ? undefined : r });
        },
        onEnd(dx, dy, _ev, moved) {
          const st = store.getState();
          st.setDrag(null);
          const r = normalizeRect(startX, startY, startX + dx, startY + dy);
          const at = { left: snap(r.left, gridSize, snapToGrid), top: snap(r.top, gridSize, snapToGrid) };
          const size = moved && r.width >= MIN_SIZE.width && r.height >= MIN_SIZE.height ? snapRect(r, gridSize, snapToGrid) : null;
          placeControl(type, at, parentId, size ? { width: Math.max(size.width, MIN_SIZE.width), height: Math.max(size.height, MIN_SIZE.height) } : undefined);
        },
        onCancel: () => store.getState().setDrag(null),
      });
      return;
    }
    if (target !== clientRef.current && !target.classList.contains('fx-form-client')) return;
    s.select([]);
    const origin = clientRef.current!.getBoundingClientRect();
    const startX = e.clientX - origin.left;
    const startY = e.clientY - origin.top;
    startPointerDrag(e, {
      onMove(dx, dy) {
        store.getState().setDrag({ kind: 'marquee', marquee: normalizeRect(startX, startY, startX + dx, startY + dy) });
      },
      onEnd(dx, dy, _ev, moved) {
        const st = store.getState();
        st.setDrag(null);
        if (moved) st.marqueeSelect(normalizeRect(startX, startY, startX + dx, startY + dy));
      },
      onCancel: () => store.getState().setDrag(null),
    });
  };

  const placeControl = (type: ControlType, at: { left: number; top: number }, parentId: string | null, size?: { width: number; height: number }) => {
    const st = store.getState();
    let target = parentId;
    // fall back to the nearest ancestor that accepts the control (e.g. clicking a page frame's tab strip)
    while (target && !canPlace(st.doc.form, target, type)) target = findLocation(st.doc.form, target)?.ancestors.at(-1)?.id ?? null;
    st.addControl(type, at, { parentId: target, size });
    st.setTool(null);
  };

  const onClientDoubleClick = (e: MouseEvent<HTMLDivElement>) => {
    if ((e.target as HTMLElement) === clientRef.current) openMethod(FORM_ID, FORM_DESCRIPTOR.defaultEvent);
  };

  const onKeyDown = (e: KeyboardEvent<HTMLDivElement>) => {
    const s = store.getState();
    const ctrl = e.ctrlKey || e.metaKey;
    const key = e.key.toLowerCase();
    const handled = () => {
      e.preventDefault();
      e.stopPropagation(); // keep window-level shortcuts from running the same command twice
    };
    if (key === 'escape') {
      if (s.tool) s.setTool(null);
      else s.select([]);
      return handled();
    }
    if (key === 'delete' || key === 'backspace') {
      s.removeSelected();
      return handled();
    }
    if (key.startsWith('arrow')) {
      const step = ctrl ? gridSize : 1;
      const dx = key === 'arrowleft' ? -step : key === 'arrowright' ? step : 0;
      const dy = key === 'arrowup' ? -step : key === 'arrowdown' ? step : 0;
      if (s.selection.length === 0) return;
      if (e.shiftKey) {
        const rects: Record<string, Rect> = {};
        for (const n of getSelectedNodes(s)) {
          const r = getRect(n);
          rects[n.id] = { ...r, width: Math.max(0, r.width + dx), height: Math.max(0, r.height + dy) };
        }
        s.setRects(rects, 'Resize');
      } else s.moveBy(s.selection, dx, dy);
      return handled();
    }
    if (ctrl && key === 'a') {
      const first = s.selection[0];
      const parent = first ? findLocation(s.doc.form, first)?.parent : undefined;
      s.selectAll(parent && 'type' in parent ? parent.id : null);
      return handled();
    }
    if (ctrl && key === 'z') return (e.shiftKey ? s.redo() : s.undo(), handled());
    if (ctrl && key === 'y') return (s.redo(), handled());
    if (ctrl && key === 'x') return (s.cut(), handled());
    if (ctrl && key === 'c') return (s.copy(), handled());
    if (ctrl && key === 'v') return (s.paste(), handled());
  };

  const clientStyle = {
    width,
    height,
    background: colorCss(getProp(form, 'BackColor')),
    backgroundSize: showGrid ? `${gridSize}px ${gridSize}px` : undefined,
    cursor: tool ? 'crosshair' : 'default',
  };

  return (
    <CanvasInteractionContext.Provider value={interactions}>
      <div className="fx-canvas-area" tabIndex={0} data-testid="designer-canvas" onKeyDown={onKeyDown}>
        <div className="fx-form-window" data-testid="form-window">
          <div className="fx-form-title">{String(getProp(form, 'Caption'))}</div>
          <div
            ref={clientRef}
            className={`fx-form-client${showGrid ? ' fx-grid' : ''}`}
            data-container-id=""
            data-testid="form-client"
            style={clientStyle}
            onPointerDown={onClientPointerDown}
            onDoubleClick={onClientDoubleClick}
          >
            {form.children.map((c: ControlNode) => (
              <ControlSurface key={c.id} node={c} />
            ))}
            {marquee && <div className="fx-marquee" data-testid="marquee" style={marquee} />}
          </div>
        </div>
      </div>
    </CanvasInteractionContext.Provider>
  );
}

function canPlace(form: import('@shared/form/schema').FormNode, parentId: string, type: ControlType): boolean {
  const parent = findNode(form, parentId);
  if (!parent) return false;
  const cont = getDescriptor(parent.type).container;
  if (!cont) return false;
  return cont.accepts === 'visual' ? !getDescriptor(type).hideInToolbox : cont.accepts.includes(type);
}
