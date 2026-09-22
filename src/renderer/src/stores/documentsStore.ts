import { create } from 'zustand';
import { nanoid } from 'nanoid';
import type { FormDocument } from '@shared/form/schema';
import type { MenuDocument } from '@shared/menu/schema';
import { basename } from '@shared/paths';
import { findNode } from '@shared/form/tree';
import { createFormDesignerStore, FORM_ID, selectIsDirty, type FormDesignerStore } from '../designer/store/createFormDesignerStore';
import { createMenuDesignerStore, selectMenuIsDirty, type MenuDesignerStore } from '../menu-designer/store/createMenuDesignerStore';

export type OpenDocument =
  | { id: string; kind: 'form'; path: string | null; store: FormDesignerStore }
  | { id: string; kind: 'menu'; path: string | null; store: MenuDesignerStore }
  | { id: string; kind: 'program'; path: string | null; text: string; savedText: string; eol: '\r\n' | '\n' }
  | { id: string; kind: 'method'; formDocId: string; controlId: string; method: string }
  /** A table opened for browsing. It is read straight off disk, so there is nothing to save. */
  | { id: string; kind: 'table'; path: string }
  /** The `_SCREEN` tab where running forms appear; at most one is open. */
  | { id: string; kind: 'desktop' };

/** A document with a file behind it that can be edited and saved. A browsed table is not one. */
export type FileDocument = Extract<OpenDocument, { kind: 'form' | 'menu' | 'program' }>;

export interface DocumentsState {
  docs: Record<string, OpenDocument>;
  order: string[];
  activeId: string | null;

  openForm(doc: FormDocument, path: string | null): string;
  openMenu(doc: MenuDocument, path: string | null): string;
  openProgram(text: string, path: string | null): string;
  /** Opens (or focuses) the method editor tab for a control's event. */
  openMethod(formDocId: string, controlId: string, method: string): string;
  /** Opens (or focuses) the browser for a DBF-based table. */
  openTable(path: string): string;
  /** Opens (or focuses) the runtime desktop tab. */
  openDesktop(): string;
  findByPath(path: string): OpenDocument | undefined;
  activate(id: string): void;
  close(id: string): void;
  closeAll(): void;
  setPath(id: string, path: string): void;
  setProgramText(id: string, text: string): void;
  markProgramSaved(id: string): void;
  /** Switches an open method tab to another control/event. */
  retargetMethod(id: string, controlId: string, method: string): void;
}

export const useDocumentsStore = create<DocumentsState>((set, get) => {
  const add = (doc: OpenDocument) => {
    set((s) => ({ docs: { ...s.docs, [doc.id]: doc }, order: [...s.order, doc.id], activeId: doc.id }));
    return doc.id;
  };
  const focusExisting = (pred: (d: OpenDocument) => boolean): string | null => {
    const existing = Object.values(get().docs).find(pred);
    if (!existing) return null;
    set({ activeId: existing.id });
    return existing.id;
  };

  return {
    docs: {},
    order: [],
    activeId: null,

    openForm(doc, path) {
      if (path) {
        const id = focusExisting((d) => d.kind === 'form' && d.path === path);
        if (id) return id;
      }
      return add({ id: nanoid(8), kind: 'form', path, store: createFormDesignerStore(doc) });
    },

    openMenu(doc, path) {
      if (path) {
        const id = focusExisting((d) => d.kind === 'menu' && d.path === path);
        if (id) return id;
      }
      return add({ id: nanoid(8), kind: 'menu', path, store: createMenuDesignerStore(doc) });
    },

    openProgram(text, path) {
      if (path) {
        const id = focusExisting((d) => d.kind === 'program' && d.path === path);
        if (id) return id;
      }
      // The editor works in newlines. A file written by Visual FoxPro uses CRLF, so it is
      // normalised on the way in and restored on the way out; otherwise every imported file
      // would look modified the moment it opened.
      const eol = text.includes('\r\n') ? '\r\n' : '\n';
      const body = text.replace(/\r\n?/g, '\n');
      return add({ id: nanoid(8), kind: 'program', path, text: body, savedText: body, eol });
    },

    openMethod(formDocId, controlId, method) {
      const id = focusExisting((d) => d.kind === 'method' && d.formDocId === formDocId && d.controlId === controlId && d.method === method);
      if (id) return id;
      return add({ id: nanoid(8), kind: 'method', formDocId, controlId, method });
    },

    openTable(path) {
      const id = focusExisting((d) => d.kind === 'table' && d.path === path);
      if (id) return id;
      return add({ id: nanoid(8), kind: 'table', path });
    },

    openDesktop() {
      const id = focusExisting((d) => d.kind === 'desktop');
      if (id) return id;
      return add({ id: nanoid(8), kind: 'desktop' });
    },

    findByPath: (path) => Object.values(get().docs).find((d) => 'path' in d && d.path === path),

    activate(id) {
      if (get().docs[id]) set({ activeId: id });
    },

    close(id) {
      const s = get();
      const doc = s.docs[id];
      if (!doc) return;
      // method tabs belong to their form and close with it; the desktop is independent
      const victims = new Set<string>([id]);
      if (doc.kind === 'form') {
        for (const d of Object.values(s.docs)) if (d.kind === 'method' && d.formDocId === id) victims.add(d.id);
      }
      const order = s.order.filter((x) => !victims.has(x));
      const docs = { ...s.docs };
      for (const v of victims) delete docs[v];
      let activeId = s.activeId;
      if (activeId && victims.has(activeId)) {
        const idx = s.order.indexOf(id);
        activeId = order[Math.min(idx, order.length - 1)] ?? null;
      }
      set({ docs, order, activeId });
    },

    closeAll() {
      set({ docs: {}, order: [], activeId: null });
    },

    setPath(id, path) {
      set((s) => {
        const d = s.docs[id];
        if (!d || !('path' in d)) return s;
        return { docs: { ...s.docs, [id]: { ...d, path } } };
      });
    },

    setProgramText(id, text) {
      set((s) => {
        const d = s.docs[id];
        if (!d || d.kind !== 'program') return s;
        return { docs: { ...s.docs, [id]: { ...d, text } } };
      });
    },

    markProgramSaved(id) {
      set((s) => {
        const d = s.docs[id];
        if (!d || d.kind !== 'program') return s;
        return { docs: { ...s.docs, [id]: { ...d, savedText: d.text } } };
      });
    },

    retargetMethod(id, controlId, method) {
      set((s) => {
        const d = s.docs[id];
        if (!d || d.kind !== 'method') return s;
        return { docs: { ...s.docs, [id]: { ...d, controlId, method } } };
      });
    },
  };
});

export function isDocDirty(doc: OpenDocument): boolean {
  switch (doc.kind) {
    case 'form':
      return selectIsDirty(doc.store.getState());
    case 'menu':
      return selectMenuIsDirty(doc.store.getState());
    case 'program':
      return doc.text !== doc.savedText;
    default:
      return false;
  }
}

export function docTitle(doc: OpenDocument, all: Record<string, OpenDocument> = useDocumentsStore.getState().docs): string {
  switch (doc.kind) {
    case 'form':
      return doc.path ? basename(doc.path) : `${doc.store.getState().doc.form.name} (new)`;
    case 'menu':
      return doc.path ? basename(doc.path) : `${doc.store.getState().doc.name} (new)`;
    case 'program':
      return doc.path ? basename(doc.path) : 'Untitled.prg';
    case 'method': {
      const form = all[doc.formDocId];
      if (!form || form.kind !== 'form') return doc.method;
      const f = form.store.getState().doc.form;
      const owner = doc.controlId === FORM_ID ? f.name : (findNode(f, doc.controlId)?.name ?? '?');
      return `${f.name}.${owner === f.name ? '' : owner + '.'}${doc.method}`;
    }
    case 'table':
      return basename(doc.path);
    case 'desktop':
      return 'Screen';
  }
}

export function isFileDocument(doc: OpenDocument): doc is FileDocument {
  return doc.kind === 'form' || doc.kind === 'menu' || doc.kind === 'program';
}
