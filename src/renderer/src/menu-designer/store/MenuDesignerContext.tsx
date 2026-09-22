import { createContext, useContext, type ReactNode } from 'react';
import { useStore } from 'zustand';
import type { MenuDesignerState, MenuDesignerStore } from './createMenuDesignerStore';

const Ctx = createContext<MenuDesignerStore | null>(null);

export function MenuDesignerProvider({ store, children }: { store: MenuDesignerStore; children: ReactNode }) {
  return <Ctx.Provider value={store}>{children}</Ctx.Provider>;
}

export function useMenuDesignerStore(): MenuDesignerStore {
  const s = useContext(Ctx);
  if (!s) throw new Error('useMenuDesignerStore outside MenuDesignerProvider');
  return s;
}

export function useMenuDesigner<T>(selector: (s: MenuDesignerState) => T): T {
  return useStore(useMenuDesignerStore(), selector);
}
