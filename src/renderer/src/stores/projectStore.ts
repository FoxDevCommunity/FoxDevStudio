import { create } from 'zustand';
import type { ProjectDocument, ProjectItem } from '@shared/project/schema';
import { kindForPath } from '@shared/project/schema';
import { basename, dirname, relative, resolveFrom } from '@shared/paths';
import { getApi } from '../api/foxdev';
import { attachSourceControl, setSourceControl } from '../runtime/sourceControlProvider';
import { NO_SOURCE_CONTROL } from '@shared/runtime/sourceControl';

export interface ProjectState {
  path: string | null;
  doc: ProjectDocument | null;
  dirty: boolean;
  recent: string[];
  error: string | null;

  /** Directory containing the project file. */
  dir(): string | null;
  resolvePath(rel: string): string;
  relativePath(abs: string): string;
  hasItem(abs: string): boolean;

  openProject(path: string): Promise<boolean>;
  newProject(dir: string, name: string): Promise<boolean>;
  save(): Promise<void>;
  /** Writes the project as another file and opens it there, which SaveAs does. */
  saveAs(path: string, doc: ProjectDocument): Promise<boolean>;
  addItem(absPath: string, kind?: ProjectItem['kind']): void;
  removeItem(rel: string): void;
  setMain(rel: string | undefined): void;
  setExcluded(rel: string, excluded: boolean): void;
  close(): void;
  loadRecent(): Promise<void>;
  clearError(): void;
}

export const useProjectStore = create<ProjectState>((set, get) => ({
  path: null,
  doc: null,
  dirty: false,
  recent: [],
  error: null,

  dir: () => (get().path ? dirname(get().path!) : null),
  resolvePath: (rel) => {
    const dir = get().dir();
    return dir ? resolveFrom(dir, rel) : rel;
  },
  relativePath: (abs) => {
    const dir = get().dir();
    return dir ? relative(dir, abs) : abs;
  },
  hasItem: (abs) => {
    const rel = get().relativePath(abs).toLowerCase();
    return !!get().doc?.items.some((i) => i.path.toLowerCase() === rel);
  },

  async openProject(path) {
    try {
      const handle = await getApi().project.open(path);
      set({ path: handle.path, doc: handle.doc, dirty: false, error: null });
      // whether the project is under source control is a question about where it sits, so it
      // is asked once here rather than every time a file's SCCStatus is read. It is asked after
      // the project is open, and its answer cannot stop it being open.
      void attachSourceControl(dirname(handle.path));
      await getApi().app.addRecentProject(handle.path);
      await get().loadRecent();
      return true;
    } catch (e) {
      set({ error: (e as Error).message });
      return false;
    }
  },

  async saveAs(path, doc) {
    await getApi().project.save(path, doc);
    return get().openProject(path);
  },

  async newProject(dir, name) {
    try {
      const handle = await getApi().project.create(dir, name);
      set({ path: handle.path, doc: handle.doc, dirty: false, error: null });
      await getApi().app.addRecentProject(handle.path);
      await get().loadRecent();
      return true;
    } catch (e) {
      set({ error: (e as Error).message });
      return false;
    }
  },

  async save() {
    const { path, doc } = get();
    if (!path || !doc) return;
    await getApi().project.save(path, doc);
    set({ dirty: false });
  },

  addItem(absPath, kind) {
    const { doc } = get();
    if (!doc || get().hasItem(absPath)) return;
    const rel = get().relativePath(absPath);
    set({ doc: { ...doc, items: [...doc.items, { kind: kind ?? kindForPath(rel), path: rel }] }, dirty: true });
  },

  removeItem(rel) {
    const { doc } = get();
    if (!doc) return;
    const items = doc.items.filter((i) => i.path !== rel);
    const main = doc.main === rel ? undefined : doc.main;
    set({ doc: { ...doc, items, ...(main === undefined ? { main: undefined } : { main }) }, dirty: true });
  },

  setMain(rel) {
    const { doc } = get();
    if (!doc) return;
    const next = { ...doc };
    if (rel === undefined) delete next.main;
    else next.main = rel;
    set({ doc: next, dirty: true });
  },

  setExcluded(rel, excluded) {
    const { doc } = get();
    if (!doc) return;
    set({
      doc: { ...doc, items: doc.items.map((i) => (i.path === rel ? (excluded ? { ...i, excluded: true } : { kind: i.kind, path: i.path }) : i)) },
      dirty: true,
    });
  },

  close() {
    setSourceControl(NO_SOURCE_CONTROL);
    set({ path: null, doc: null, dirty: false, error: null });
  },

  async loadRecent() {
    set({ recent: await getApi().app.getRecentProjects() });
  },

  clearError: () => set({ error: null }),
}));

export function projectTitle(s: ProjectState): string {
  return s.doc ? s.doc.name : s.path ? basename(s.path, true) : '';
}
