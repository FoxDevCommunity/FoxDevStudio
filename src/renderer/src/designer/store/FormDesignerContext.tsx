import { createContext, useContext, type ReactNode } from 'react';
import { useStore } from 'zustand';
import type { FormDesignerState, FormDesignerStore } from './createFormDesignerStore';

interface Ctx {
  store: FormDesignerStore;
  /** Id of the owning document in the documents store (used to open method tabs). */
  docId: string;
}

const FormDesignerCtx = createContext<Ctx | null>(null);

export function FormDesignerProvider({ store, docId, children }: { store: FormDesignerStore; docId: string; children: ReactNode }) {
  return <FormDesignerCtx.Provider value={{ store, docId }}>{children}</FormDesignerCtx.Provider>;
}

export function useFormDesignerContext(): Ctx {
  const ctx = useContext(FormDesignerCtx);
  if (!ctx) throw new Error('useFormDesignerContext must be used inside FormDesignerProvider');
  return ctx;
}

export function useOptionalFormDesignerContext(): Ctx | null {
  return useContext(FormDesignerCtx);
}

export function useFormDesignerStore(): FormDesignerStore {
  return useFormDesignerContext().store;
}

export function useFormDesigner<T>(selector: (s: FormDesignerState) => T): T {
  return useStore(useFormDesignerContext().store, selector);
}
