import { createStore, type StoreApi } from 'zustand/vanilla';
import { nanoid } from 'nanoid';
import type { MenuDocument, MenuHotkey, MenuItem, MenuLocation } from '@shared/menu/schema';
import { findMenuItem, indentMenuItem, moveMenuItem, outdentMenuItem, removeMenuItem } from '@shared/menu/tree';
import * as H from '../../stores/history';

export interface MenuDesignerState {
  doc: MenuDocument;
  selectedId: string | null;
  history: H.UndoState<string | null>;

  /** Inserts after the selected item (or at the end of the top level). Returns the new id. */
  insertItem(kind?: 'item' | 'separator'): string;
  /** Inserts as the last child of `parentId` (null = top level). */
  insertChild(parentId: string | null, kind?: 'item' | 'separator'): string;
  removeItem(id: string): void;
  moveItem(id: string, delta: -1 | 1): boolean;
  indent(id: string): boolean;
  outdent(id: string): boolean;
  setPrompt(id: string, prompt: string): void;
  setItemField(id: string, field: 'name' | 'skipFor' | 'message', value: string): void;
  setEnabled(id: string, enabled: boolean): void;
  setResult(id: string, type: MenuItem['result']['type'], text?: string): void;
  setHotkey(id: string, hotkey: MenuHotkey | undefined): void;
  setDocField(field: 'name' | 'setup' | 'cleanup', value: string): void;
  setLocation(location: MenuLocation): void;
  select(id: string | null): void;
  beginTxn(label: string): void;
  endTxn(): void;
  undo(): void;
  redo(): void;
  load(doc: MenuDocument): void;
  markSaved(): void;
}

export type MenuDesignerStore = StoreApi<MenuDesignerState>;

export const selectMenuIsDirty = (s: MenuDesignerState): boolean => H.isDirty(s.history);

export function createMenuDesignerStore(initial: MenuDocument, opts: { newId?: () => string } = {}): MenuDesignerStore {
  const newId = opts.newId ?? (() => nanoid(10));

  return createStore<MenuDesignerState>((set, get) => {
    const commit = (label: string, recipe: (doc: MenuDocument) => void, nextSelected?: string | null) => {
      const s = get();
      const after = nextSelected === undefined ? s.selectedId : nextSelected;
      const r = H.applyChange(s.history, s.doc, label, s.selectedId, after, (d) => recipe(d as MenuDocument));
      if (!r.changed) {
        if (nextSelected !== undefined) set({ selectedId: nextSelected });
        return;
      }
      set({ doc: r.doc, history: r.history, selectedId: after !== null && !findMenuItem(r.doc.items, after) ? null : after });
    };

    const makeItem = (kind: 'item' | 'separator', existing: MenuItem[]): MenuItem => {
      const id = newId();
      if (kind === 'separator') return { id, prompt: '\\-', result: { type: 'bar' } };
      let n = 1;
      const taken = new Set<string>();
      const walk = (items: MenuItem[]) => {
        for (const i of items) {
          taken.add(i.prompt.toLowerCase());
          if (i.children) walk(i.children);
        }
      };
      walk(existing);
      while (taken.has(`item${n}`)) n++;
      return { id, prompt: `Item${n}`, result: { type: 'command', text: '' } };
    };

    return {
      doc: initial,
      selectedId: null,
      history: H.createUndoState<string | null>(),

      insertItem(kind = 'item') {
        const s = get();
        const item = makeItem(kind, s.doc.items);
        commit(
          'Insert item',
          (d) => {
            const loc = s.selectedId ? findMenuItem(d.items, s.selectedId) : undefined;
            if (loc) loc.siblings.splice(loc.index + 1, 0, item);
            else d.items.push(item);
          },
          item.id,
        );
        return item.id;
      },

      insertChild(parentId, kind = 'item') {
        const s = get();
        const item = makeItem(kind, s.doc.items);
        commit(
          'Insert item',
          (d) => {
            if (parentId === null) {
              d.items.push(item);
              return;
            }
            const loc = findMenuItem(d.items, parentId);
            if (!loc) return;
            loc.item.children ??= [];
            loc.item.result = { type: 'submenu' };
            loc.item.children.push(item);
          },
          item.id,
        );
        return item.id;
      },

      removeItem(id) {
        const s = get();
        const loc = findMenuItem(s.doc.items, id);
        if (!loc) return;
        const neighbor = loc.siblings[loc.index + 1] ?? loc.siblings[loc.index - 1] ?? loc.parent;
        commit('Delete item', (d) => void removeMenuItem(d.items, id), neighbor ? neighbor.id : null);
      },

      moveItem(id, delta) {
        let ok = false;
        commit(delta < 0 ? 'Move up' : 'Move down', (d) => void (ok = moveMenuItem(d.items, id, delta)));
        return ok;
      },

      indent(id) {
        let ok = false;
        commit('Indent', (d) => void (ok = indentMenuItem(d.items, id)));
        return ok;
      },

      outdent(id) {
        let ok = false;
        commit('Outdent', (d) => void (ok = outdentMenuItem(d.items, id)));
        return ok;
      },

      setPrompt(id, prompt) {
        commit('Change prompt', (d) => {
          const loc = findMenuItem(d.items, id);
          if (loc) loc.item.prompt = prompt;
        });
      },

      setItemField(id, field, value) {
        commit(`Change ${field}`, (d) => {
          const loc = findMenuItem(d.items, id);
          if (!loc) return;
          if (value === '') delete loc.item[field];
          else loc.item[field] = value;
        });
      },

      setEnabled(id, enabled) {
        commit('Change enabled', (d) => {
          const loc = findMenuItem(d.items, id);
          if (!loc) return;
          if (enabled) delete loc.item.enabled;
          else loc.item.enabled = false;
        });
      },

      setResult(id, type, text) {
        commit('Change result', (d) => {
          const loc = findMenuItem(d.items, id);
          if (!loc) return;
          loc.item.result = text === undefined || type === 'submenu' ? { type } : { type, text };
          if (type === 'submenu') loc.item.children ??= [];
        });
      },

      setHotkey(id, hotkey) {
        commit('Change shortcut', (d) => {
          const loc = findMenuItem(d.items, id);
          if (!loc) return;
          if (hotkey) loc.item.hotkey = hotkey;
          else delete loc.item.hotkey;
        });
      },

      setDocField(field, value) {
        commit(`Change ${field}`, (d) => {
          if (field === 'name') d.name = value;
          else if (value === '') delete d[field];
          else d[field] = value;
        });
      },

      setLocation(location) {
        commit('Change location', (d) => void (d.location = location));
      },

      select(id) {
        set({ selectedId: id });
      },

      beginTxn(label) {
        const s = get();
        set({ history: H.beginTxn(s.history, label, s.selectedId) });
      },
      endTxn() {
        const s = get();
        set({ history: H.endTxn(s.history, s.selectedId) });
      },
      undo() {
        const s = get();
        const r = H.undo(s.history, s.doc);
        if (r) set({ doc: r.doc, history: r.history, selectedId: r.extra });
      },
      redo() {
        const s = get();
        const r = H.redo(s.history, s.doc);
        if (r) set({ doc: r.doc, history: r.history, selectedId: r.extra });
      },
      load(doc) {
        set({ doc, selectedId: null, history: H.createUndoState<string | null>() });
      },
      markSaved() {
        set({ history: H.markSaved(get().history) });
      },
    };
  });
}
