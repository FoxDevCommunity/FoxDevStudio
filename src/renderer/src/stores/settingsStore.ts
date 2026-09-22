import { create } from 'zustand';

export interface SettingsState {
  theme: 'light' | 'dark';
  gridSize: number;
  snapToGrid: boolean;
  showGrid: boolean;
  setTheme(theme: 'light' | 'dark'): void;
  toggleTheme(): void;
  setGridSize(size: number): void;
  setSnapToGrid(on: boolean): void;
  setShowGrid(on: boolean): void;
}

const KEY = 'foxdev.settings';

function loadPersisted(): Partial<Pick<SettingsState, 'theme' | 'gridSize' | 'snapToGrid' | 'showGrid'>> {
  try {
    const raw = globalThis.localStorage?.getItem(KEY);
    return raw ? (JSON.parse(raw) as Partial<SettingsState>) : {};
  } catch {
    return {};
  }
}

function persist(s: SettingsState): void {
  try {
    globalThis.localStorage?.setItem(KEY, JSON.stringify({ theme: s.theme, gridSize: s.gridSize, snapToGrid: s.snapToGrid, showGrid: s.showGrid }));
  } catch {
    /* storage unavailable */
  }
}

export const useSettingsStore = create<SettingsState>((set, get) => {
  const saved = loadPersisted();
  const update = (patch: Partial<SettingsState>) => {
    set(patch);
    persist(get());
  };
  return {
    theme: saved.theme ?? 'light',
    gridSize: saved.gridSize ?? 8,
    snapToGrid: saved.snapToGrid ?? true,
    showGrid: saved.showGrid ?? true,
    setTheme: (theme) => update({ theme }),
    toggleTheme: () => update({ theme: get().theme === 'light' ? 'dark' : 'light' }),
    setGridSize: (gridSize) => update({ gridSize: Math.max(1, Math.min(64, Math.round(gridSize))) }),
    setSnapToGrid: (snapToGrid) => update({ snapToGrid }),
    setShowGrid: (showGrid) => update({ showGrid }),
  };
});
