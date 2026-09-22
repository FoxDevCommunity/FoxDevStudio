import { useSyncExternalStore } from 'react';
import type { FormDesignerStore } from '../designer/store/createFormDesignerStore';
import { isDocDirty, useDocumentsStore, type OpenDocument } from '../stores/documentsStore';

export function getActiveDoc(): OpenDocument | undefined {
  const s = useDocumentsStore.getState();
  return s.activeId ? s.docs[s.activeId] : undefined;
}

/** The form document that owns `doc`: itself, or the form a method tab belongs to. */
export function ownerFormDoc(doc: OpenDocument | undefined): Extract<OpenDocument, { kind: 'form' }> | undefined {
  if (!doc) return undefined;
  if (doc.kind === 'form') return doc;
  if (doc.kind === 'method') {
    const owner = useDocumentsStore.getState().docs[doc.formDocId];
    return owner?.kind === 'form' ? owner : undefined;
  }
  return undefined;
}

export function getActiveFormStore(): FormDesignerStore | undefined {
  return ownerFormDoc(getActiveDoc())?.store;
}

export function useActiveDoc(): OpenDocument | undefined {
  return useDocumentsStore((s) => (s.activeId ? s.docs[s.activeId] : undefined));
}

export function useActiveFormDoc() {
  const docs = useDocumentsStore((s) => s.docs);
  const activeId = useDocumentsStore((s) => s.activeId);
  return ownerFormDoc(activeId ? docs[activeId] : undefined);
}

/** Subscribes to a document's dirty flag, whichever store backs it. */
export function useDocDirty(doc: OpenDocument | undefined): boolean {
  return useSyncExternalStore(
    (cb) => {
      if (!doc) return () => {};
      const unsubs = [useDocumentsStore.subscribe(cb)];
      if (doc.kind === 'form' || doc.kind === 'menu') unsubs.push(doc.store.subscribe(cb));
      return () => unsubs.forEach((u) => u());
    },
    () => {
      if (!doc) return false;
      const live = useDocumentsStore.getState().docs[doc.id] ?? doc;
      return isDocDirty(live);
    },
  );
}

/** True when any open document or the project has unsaved changes. */
export function useAnyDirty(): boolean {
  return useSyncExternalStore(subscribeAllDocs, () => Object.values(useDocumentsStore.getState().docs).some(isDocDirty));
}

/** Subscribes to the documents store and to every document store, re-subscribing as documents open and close. */
function subscribeAllDocs(cb: () => void): () => void {
  let inner: (() => void)[] = [];
  const resubscribe = () => {
    inner.forEach((u) => u());
    inner = Object.values(useDocumentsStore.getState().docs).flatMap((d) => (d.kind === 'form' || d.kind === 'menu' ? [d.store.subscribe(cb)] : []));
  };
  resubscribe();
  const off = useDocumentsStore.subscribe(() => {
    resubscribe();
    cb();
  });
  return () => {
    off();
    inner.forEach((u) => u());
  };
}
