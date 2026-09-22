import { createStore, type StoreApi } from 'zustand/vanilla';
import { nanoid } from 'nanoid';
import type { ControlNode, ControlType, FormDocument, FormNode, PropValue } from '@shared/form/schema';
import { CONTROL_DESCRIPTORS, canContain, getDescriptor, getProp } from '@shared/registry';
import { dedupeName, isValidName, uniqueName } from '@shared/form/naming';
import { allNodes, cloneSubtree, findLocation, findNode, insertChild, isFormNode, removeNode, reorder, reparent, resolveParent, type ParentNode } from '@shared/form/tree';
import { alignRects, distribute, sameSize, marqueeHits, type AlignKind, type Rect, type SizeKind } from '../geometry';
import * as H from '../../stores/history';

/** Sentinel id addressing the form itself in setProp/setMethod. Selection [] also means "form selected". */
export const FORM_ID = '$form';

export type ZOrder = 'front' | 'back' | 'forward' | 'backward';
export type SelectMode = 'replace' | 'add' | 'toggle';

export interface DragState {
  kind: 'move' | 'resize' | 'marquee' | 'create';
  /** Live rects while dragging (not committed to the document). */
  rects?: Record<string, Rect>;
  marquee?: Rect;
}

export interface FormDesignerState {
  doc: FormDocument;
  /** Selected control ids; empty = the form. */
  selection: string[];
  history: H.UndoState<string[]>;
  drag: DragState | null;
  gridSize: number;
  /** Toolbox tool waiting for a click on the canvas. */
  tool: ControlType | null;
  /** Active page index per PageFrame id (designer-only UI state). */
  activePages: Record<string, number>;

  addControl(type: ControlType, at: { left: number; top: number }, opts?: { parentId?: string | null; size?: { width: number; height: number } }): string | null;
  removeControls(ids: string[]): void;
  removeSelected(): void;
  setProp(ids: string[], name: string, value: PropValue): void;
  setName(id: string, name: string): boolean;
  setMethod(id: string, method: string, source: string): void;
  moveBy(ids: string[], dx: number, dy: number): void;
  setRects(rects: Record<string, Rect>, label?: string): void;
  reparent(id: string, parentId: string | null, index?: number): boolean;
  setZOrder(ids: string[], where: ZOrder): void;
  align(kind: AlignKind): void;
  sameSize(kind: SizeKind): void;
  distribute(axis: 'horizontal' | 'vertical'): void;
  cut(): void;
  copy(): void;
  paste(): string[];
  select(ids: string[], mode?: SelectMode): void;
  selectAll(parentId?: string | null): void;
  marqueeSelect(rect: Rect, parentId?: string | null): void;
  setDrag(drag: DragState | null): void;
  setGridSize(size: number): void;
  setTool(tool: ControlType | null): void;
  setActivePage(pageFrameId: string, index: number): void;
  beginTxn(label: string): void;
  endTxn(): void;
  undo(): void;
  redo(): void;
  load(doc: FormDocument): void;
  markSaved(): void;
}

export type FormDesignerStore = StoreApi<FormDesignerState>;

/** Module-level clipboard shared by every open form. */
let clipboard: ControlNode[] | null = null;
export function getClipboard(): ControlNode[] | null {
  return clipboard;
}
export function setClipboard(nodes: ControlNode[] | null): void {
  clipboard = nodes;
}

export const selectIsDirty = (s: FormDesignerState): boolean => H.isDirty(s.history);
export const selectCanUndo = (s: FormDesignerState): boolean => H.canUndo(s.history);
export const selectCanRedo = (s: FormDesignerState): boolean => H.canRedo(s.history);

/**
 * Where a node sits on the canvas.
 *
 * A control that is never drawn - a Timer, a Custom - still has a Width and a Height, because
 * the class has them; what the designer shows is an icon of a fixed size, and that icon is what
 * a marquee catches and a handle drags.
 */
export function getRect(node: ControlNode | FormNode): Rect {
  const icon = 'type' in node ? getDescriptor(node.type) : undefined;
  const size = icon?.nonVisual ? icon.defaultSize : undefined;
  return {
    left: Number(getProp(node, 'Left') ?? 0),
    top: Number(getProp(node, 'Top') ?? 0),
    width: size ? size.Width : Number(getProp(node, 'Width') ?? 0),
    height: size ? size.Height : Number(getProp(node, 'Height') ?? 0),
  };
}

export function getSelectedNodes(state: FormDesignerState): ControlNode[] {
  return state.selection.map((id) => findNode(state.doc.form, id)).filter((n): n is ControlNode => !!n);
}

/** Nodes in the selection that are not nested under another selected node. */
export function topLevelSelection(form: FormNode, selection: string[]): ControlNode[] {
  const set = new Set(selection);
  const out: ControlNode[] = [];
  for (const id of selection) {
    const loc = findLocation(form, id);
    if (loc && !loc.ancestors.some((a) => set.has(a.id))) out.push(loc.node);
  }
  return out;
}

function siblingNames(parent: ParentNode): string[] {
  return (parent.children ?? []).map((c) => c.name);
}

function sanitizeSelection(selection: string[], form: FormNode): string[] {
  const ids = new Set(allNodes(form).map((n) => n.id));
  const out = selection.filter((id) => ids.has(id));
  return out.length === selection.length ? selection : out;
}

function createNode(type: ControlType, parent: ParentNode, newId: () => string, props: Record<string, PropValue> = {}, id?: string): ControlNode {
  const desc = getDescriptor(type);
  const node: ControlNode = { id: id ?? newId(), type, name: uniqueName(desc.namePrefix, siblingNames(parent)), props: { ...props }, methods: {} };
  // the size and the caption a new control gets are the designer's choices, not the class's: a
  // CommandButton is 17 high until something puts it on a form, and it is called Command until
  // it is Command1. Visual FoxPro writes both into the document, and so does this.
  const has = (name: string) => desc.properties.some((p) => p.name === name);
  if (has('Width')) node.props['Width'] ??= desc.defaultSize.Width;
  if (has('Height')) node.props['Height'] ??= desc.defaultSize.Height;
  if (has('Caption')) node.props['Caption'] ??= node.name;
  if (desc.container) node.children = [];
  const auto = desc.container?.autoChildren;
  if (auto) {
    for (let i = 0; i < auto.count; i++) node.children!.push(createAutoChild(node, auto.type, i, newId));
  }
  return node;
}

/** Pages/OptionButtons/CommandButtons/Columns/Headers created by their parent, laid out like VFP does. */
function createAutoChild(parent: ControlNode, type: ControlType, index: number, newId: () => string): ControlNode {
  const props: Record<string, PropValue> = {};
  if (type === 'OptionButton') Object.assign(props, { Left: 5, Top: 5 + index * 17 });
  if (type === 'CommandButton') Object.assign(props, { Left: 5, Top: 5 + index * 27, Width: 74 });
  // the caption comes from the name, which createNode works out from the siblings already there
  return createNode(type, parent, newId, props);
}

function syncAutoChildren(node: ControlNode, newId: () => string): void {
  const auto = getDescriptor(node.type).container?.autoChildren;
  if (!auto?.countProp) return;
  const wanted = Math.max(0, Number(getProp(node, auto.countProp)));
  node.children ??= [];
  while (node.children.length > wanted) node.children.pop();
  while (node.children.length < wanted) node.children.push(createAutoChild(node, auto.type, node.children.length, newId));
}

function applyRect(node: ControlNode, rect: Rect): void {
  const desc = getDescriptor(node.type);
  const has = (n: string) => desc.properties.some((p) => p.name === n);
  if (has('Left')) node.props['Left'] = rect.left;
  if (has('Top')) node.props['Top'] = rect.top;
  if (has('Width')) node.props['Width'] = rect.width;
  if (has('Height')) node.props['Height'] = rect.height;
}

export function createFormDesignerStore(initial: FormDocument, opts: { gridSize?: number; newId?: () => string } = {}): FormDesignerStore {
  const newId = opts.newId ?? (() => nanoid(10));

  return createStore<FormDesignerState>((set, get) => {
    /** Applies a recipe to the document as one undo entry (or into the open transaction). */
    const commit = (label: string, recipe: (form: FormNode, doc: FormDocument) => void, nextSelection?: string[]) => {
      const s = get();
      const after = nextSelection ?? s.selection;
      const r = H.applyChange(s.history, s.doc, label, s.selection, after, (d) => recipe(d.form, d));
      if (!r.changed) {
        if (nextSelection) set({ selection: nextSelection });
        return false;
      }
      set({ doc: r.doc, history: r.history, selection: sanitizeSelection(after, r.doc.form) });
      return true;
    };

    const targets = (form: FormNode, ids: string[]): (ControlNode | FormNode)[] =>
      ids.map((id) => (id === FORM_ID ? form : findNode(form, id))).filter((n): n is ControlNode | FormNode => !!n);

    const rectItems = (s: FormDesignerState) => getSelectedNodes(s).map((n) => ({ id: n.id, rect: getRect(n) }));

    return {
      doc: initial,
      selection: [],
      history: H.createUndoState<string[]>(),
      drag: null,
      gridSize: opts.gridSize ?? 8,
      tool: null,
      activePages: {},

      addControl(type, at, o = {}) {
        const form = get().doc.form;
        const parent = resolveParent(form, o.parentId ?? null);
        if (!parent) return null;
        const parentType = isFormNode(parent) ? undefined : parent.type;
        if (!canContain(parentType, type)) return null;
        const id = newId();
        commit(
          `Add ${type}`,
          (f) => {
            const p = resolveParent(f, o.parentId ?? null)!;
            const desc = getDescriptor(type);
            const props: Record<string, PropValue> = { Left: at.left, Top: at.top };
            if (o.size) {
              props['Width'] = o.size.width;
              props['Height'] = o.size.height;
            }
            const node = createNode(type, p, newId, props, id);
            if (desc.properties.some((pm) => pm.name === 'Caption') && !('Caption' in props)) node.props['Caption'] = node.name;
            insertChild(p, node);
          },
          [id],
        );
        return id;
      },

      removeControls(ids) {
        const form = get().doc.form;
        const removable = topLevelSelection(form, ids).filter((n) => !getDescriptor(n.type).hideInToolbox);
        if (removable.length === 0) return;
        commit(
          removable.length === 1 ? `Delete ${removable[0]!.name}` : `Delete ${removable.length} controls`,
          (f) => {
            for (const n of removable) removeNode(f, n.id);
          },
          [],
        );
      },

      removeSelected() {
        get().removeControls(get().selection);
      },

      setProp(ids, name, value) {
        commit(`Change ${name}`, (f) => {
          for (const node of targets(f, ids)) {
            node.props[name] = value;
            if (!isFormNode(node)) syncAutoChildren(node, newId);
          }
        });
      },

      setName(id, name) {
        const form = get().doc.form;
        if (!isValidName(name)) return false;
        if (id === FORM_ID) {
          if (form.name === name) return true;
          commit('Rename form', (f) => void (f.name = name));
          return true;
        }
        const loc = findLocation(form, id);
        if (!loc) return false;
        if (loc.node.name === name) return true;
        const clash = siblingNames(loc.parent).some((n) => n.toLowerCase() === name.toLowerCase());
        if (clash) return false;
        commit(`Rename ${loc.node.name}`, (f) => void (findNode(f, id)!.name = name));
        return true;
      },

      setMethod(id, method, source) {
        commit(`Edit ${method}`, (f) => {
          const node = id === FORM_ID ? f : findNode(f, id);
          if (!node) return;
          if (source === '') delete node.methods[method];
          else node.methods[method] = source;
        });
      },

      moveBy(ids, dx, dy) {
        if (dx === 0 && dy === 0) return;
        commit('Move', (f) => {
          for (const node of targets(f, ids)) {
            if (isFormNode(node)) continue;
            const desc = getDescriptor(node.type);
            if (!desc.properties.some((p) => p.name === 'Left')) continue;
            node.props['Left'] = Number(getProp(node, 'Left')) + dx;
            node.props['Top'] = Number(getProp(node, 'Top')) + dy;
          }
        });
      },

      setRects(rects, label = 'Move/Resize') {
        commit(label, (f) => {
          for (const [id, rect] of Object.entries(rects)) {
            const node = findNode(f, id);
            if (node) applyRect(node, rect);
          }
        });
      },

      reparent(id, parentId, index) {
        const form = get().doc.form;
        const node = findNode(form, id);
        const parent = resolveParent(form, parentId);
        if (!node || !parent) return false;
        if (!canContain(isFormNode(parent) ? undefined : parent.type, node.type)) return false;
        let ok = false;
        commit(`Move ${node.name}`, (f) => {
          ok = reparent(f, id, parentId, index);
          if (ok) {
            const moved = findNode(f, id)!;
            const p = resolveParent(f, parentId)!;
            moved.name = dedupeName(moved.name, new Set((p.children ?? []).filter((c) => c.id !== moved.id).map((c) => c.name)));
          }
        });
        return ok;
      },

      setZOrder(ids, where) {
        commit(`Z-order ${where}`, (f) => {
          const ordered = where === 'front' || where === 'forward' ? ids : [...ids].reverse();
          for (const id of ordered) reorder(f, id, where);
        });
      },

      align(kind) {
        const changes = alignRects(rectItems(get()), kind);
        if (Object.keys(changes).length) get().setRects(changes, `Align ${kind}`);
      },

      sameSize(kind) {
        const changes = sameSize(rectItems(get()), kind);
        if (Object.keys(changes).length) get().setRects(changes, `Same ${kind}`);
      },

      distribute(axis) {
        const changes = distribute(rectItems(get()), axis);
        if (Object.keys(changes).length) get().setRects(changes, `Distribute ${axis}`);
      },

      copy() {
        const s = get();
        const nodes = topLevelSelection(s.doc.form, s.selection).filter((n) => !getDescriptor(n.type).hideInToolbox);
        clipboard = nodes.length ? nodes.map((n) => cloneSubtree(n, () => n.id)) : null;
      },

      cut() {
        get().copy();
        if (clipboard) get().removeSelected();
      },

      paste() {
        const s = get();
        if (!clipboard || clipboard.length === 0) return [];
        const form = s.doc.form;
        // paste into the single selected container, otherwise into the form
        let parentId: string | null = null;
        if (s.selection.length === 1) {
          const sel = findNode(form, s.selection[0]!);
          if (sel && getDescriptor(sel.type).container?.accepts === 'visual') parentId = sel.id;
        }
        const parent = resolveParent(form, parentId)!;
        const parentType = isFormNode(parent) ? undefined : parent.type;
        const pasteable = clipboard.filter((n) => canContain(parentType, n.type));
        if (pasteable.length === 0) return [];
        const ids: string[] = [];
        commit(
          'Paste',
          (f) => {
            const p = resolveParent(f, parentId)!;
            const taken = new Set(siblingNames(p));
            for (const src of pasteable) {
              const node = cloneSubtree(src, newId);
              node.name = dedupeName(node.name, taken);
              taken.add(node.name);
              if ('Left' in node.props || getDescriptor(node.type).properties.some((pm) => pm.name === 'Left')) {
                node.props['Left'] = Number(getProp(node, 'Left')) + s.gridSize;
                node.props['Top'] = Number(getProp(node, 'Top')) + s.gridSize;
              }
              insertChild(p, node);
              ids.push(node.id);
            }
          },
          ids,
        );
        set({ selection: ids });
        return ids;
      },

      select(ids, mode = 'replace') {
        const s = get();
        let next: string[];
        if (mode === 'replace') next = ids;
        else if (mode === 'add') next = [...s.selection, ...ids.filter((id) => !s.selection.includes(id))];
        else next = ids.reduce((acc, id) => (acc.includes(id) ? acc.filter((x) => x !== id) : [...acc, id]), s.selection);
        set({ selection: sanitizeSelection(next, s.doc.form) });
      },

      selectAll(parentId = null) {
        const parent = resolveParent(get().doc.form, parentId);
        set({ selection: (parent?.children ?? []).map((c) => c.id) });
      },

      marqueeSelect(rect, parentId = null) {
        const parent = resolveParent(get().doc.form, parentId);
        const items = (parent?.children ?? []).map((c) => ({ id: c.id, rect: getRect(c) }));
        set({ selection: marqueeHits(items, rect) });
      },

      setDrag(drag) {
        set({ drag });
      },

      setGridSize(size) {
        set({ gridSize: Math.max(1, size) });
      },

      setTool(tool) {
        set({ tool });
      },

      setActivePage(pageFrameId, index) {
        set({ activePages: { ...get().activePages, [pageFrameId]: index } });
      },

      beginTxn(label) {
        const s = get();
        set({ history: H.beginTxn(s.history, label, s.selection) });
      },

      endTxn() {
        const s = get();
        set({ history: H.endTxn(s.history, s.selection) });
      },

      undo() {
        const s = get();
        const r = H.undo(s.history, s.doc);
        if (r) set({ doc: r.doc, history: r.history, selection: sanitizeSelection(r.extra, r.doc.form) });
      },

      redo() {
        const s = get();
        const r = H.redo(s.history, s.doc);
        if (r) set({ doc: r.doc, history: r.history, selection: sanitizeSelection(r.extra, r.doc.form) });
      },

      load(doc) {
        set({ doc, selection: [], history: H.createUndoState<string[]>(), drag: null });
      },

      markSaved() {
        set({ history: H.markSaved(get().history) });
      },
    };
  });
}

export { CONTROL_DESCRIPTORS };
