/**
 * File-level workflows that span the api, the project store and the documents store:
 * open a file into a tab, save a tab (with Save As when untitled), close with an unsaved prompt.
 * Pure async functions so they can be driven from menu commands and from tests alike.
 */
import { parseFormDocument, stringifyFormDocument, createEmptyFormDocument } from '@shared/form/serialize';
import { parseMenuDocument, stringifyMenuDocument, createEmptyMenuDocument } from '@shared/menu/serialize';
import { basename, dirname, extname, join } from '@shared/paths';
import type { FileFilter } from '@shared/ipc/api';
import { getApi } from '../api/foxdev';
import { useProjectStore } from './projectStore';
import { docTitle, isDocDirty, isFileDocument, useDocumentsStore, type OpenDocument } from './documentsStore';
import { isTableFile, isVfpFile, openVfpFile } from '../vfp/openVfpFile';
import { refreshed } from '../vfp/refreshImport';
import { importVfpProjectPath } from '../vfp/importActions';

export const FILTERS: Record<'form' | 'menu' | 'program' | 'project' | 'any', FileFilter[]> = {
  form: [{ name: 'FoxDev Form', extensions: ['fxf'] }],
  menu: [{ name: 'FoxDev Menu', extensions: ['fxm'] }],
  program: [{ name: 'Program', extensions: ['prg'] }],
  project: [{ name: 'FoxDev Project', extensions: ['fxproject'] }],
  any: [
    { name: 'FoxDev files', extensions: ['fxproject', 'fxf', 'fxm', 'prg'] },
    { name: 'Visual FoxPro files', extensions: ['pjx', 'scx', 'vcx', 'mnx', 'prg'] },
    { name: 'All files', extensions: ['*'] },
  ],
};

export async function openFile(path: string): Promise<string> {
  const docs = useDocumentsStore.getState();
  const existing = docs.findByPath(path);
  if (existing) {
    docs.activate(existing.id);
    return existing.id;
  }
  // Visual FoxPro's DBF-based documents are converted on the way in
  if (isVfpFile(path)) return openVfpFile(path);
  // data tables are shown as data: they are DBF files, and reading one as text is nonsense
  if (isTableFile(path)) return docs.openTable(path);

  const text = await getApi().files.readText(path);
  switch (extname(path).toLowerCase()) {
    case '.fxf': {
      const r = parseFormDocument(text);
      if (!r.ok) throw new Error(`${path}: ${r.error}`);
      // a document an older importer made is converted again from the file it came from
      return docs.openForm(await refreshed(r.doc, path), path);
    }
    case '.fxm': {
      const r = parseMenuDocument(text);
      if (!r.ok) throw new Error(`${path}: ${r.error}`);
      return docs.openMenu(r.doc, path);
    }
    default:
      return docs.openProgram(text, path);
  }
}

export async function openFileDialog(): Promise<string | null> {
  const path = await getApi().dialog.openFile({ title: 'Open', filters: FILTERS.any, defaultPath: useProjectStore.getState().dir() ?? undefined });
  if (!path) return null;
  if (extname(path).toLowerCase() === '.fxproject') {
    await useProjectStore.getState().openProject(path);
    return null;
  }
  if (extname(path).toLowerCase() === '.pjx') {
    await importVfpProjectPath(path);
    return null;
  }
  return openFile(path);
}

export function newForm(): string {
  return useDocumentsStore.getState().openForm(createEmptyFormDocument(), null);
}

export function newMenu(): string {
  return useDocumentsStore.getState().openMenu(createEmptyMenuDocument(), null);
}

export function newProgram(): string {
  return useDocumentsStore.getState().openProgram('', null);
}

/**
 * `CREATE FORM`, `CREATE MENU`, `CREATE PROJECT` and the rest: something new of that kind,
 * opened in its designer and saved where the command said, if it said.
 *
 * A kind this IDE has no designer for says so, which is what a program that asks for one has to
 * be told either way.
 */
export async function newDocument(kind: string, path: string): Promise<void> {
  const docs = useDocumentsStore.getState();
  const named = path.trim();
  switch (kind.toUpperCase()) {
    // SCREEN is what forms were called before there were forms
    case 'FORM':
    case 'SCREEN': {
      const id = docs.openForm(createEmptyFormDocument(), named || null);
      if (named) await saveDocument(id);
      return;
    }
    case 'MENU': {
      const id = docs.openMenu(createEmptyMenuDocument(), named || null);
      if (named) await saveDocument(id);
      return;
    }
    // a query is a program that holds one SELECT, which is what the designer would write
    case 'QUERY': {
      const id = docs.openProgram('SELECT * FROM ;\n', named || null);
      if (named) await saveDocument(id);
      return;
    }
    case 'PROJECT': {
      if (!named) throw new Error('CREATE PROJECT needs a name to make the project under');
      const project = useProjectStore.getState();
      await project.newProject(dirname(named), basename(named).replace(/\.[^.]+$/, ''));
      return;
    }
    default:
      throw new Error(`CREATE ${kind.toUpperCase()} has no designer here`);
  }
}

function serialize(doc: OpenDocument): string {
  switch (doc.kind) {
    case 'form':
      return stringifyFormDocument(doc.store.getState().doc);
    case 'menu':
      return stringifyMenuDocument(doc.store.getState().doc);
    case 'program':
      return doc.eol === '\n' ? doc.text : doc.text.replace(/\n/g, doc.eol);
    default:
      return '';
  }
}

function markSaved(doc: OpenDocument): void {
  if (doc.kind === 'form' || doc.kind === 'menu') doc.store.getState().markSaved();
  else if (doc.kind === 'program') useDocumentsStore.getState().markProgramSaved(doc.id);
}

/** Saves the document, asking for a path when it has none. Returns false when the user cancelled. */
export async function saveDocument(id: string, saveAs = false): Promise<boolean> {
  const docs = useDocumentsStore.getState();
  const doc = docs.docs[id];
  if (!doc || !isFileDocument(doc)) return false;
  let path = doc.path;
  if (!path || saveAs) {
    const kind = doc.kind;
    const defaultName = docTitle(doc).replace(/ \(new\)$/, '') + (path ? '' : { form: '.fxf', menu: '.fxm', program: '.prg' }[kind]);
    const project = useProjectStore.getState();
    const chosen = await getApi().dialog.saveFile({
      title: saveAs ? 'Save As' : 'Save',
      filters: FILTERS[kind],
      defaultPath: project.dir() ? join(project.dir()!, defaultName) : defaultName,
    });
    if (!chosen) return false;
    path = chosen;
    docs.setPath(id, path);
  }
  await getApi().files.writeText(path, serialize(doc));
  markSaved(useDocumentsStore.getState().docs[id]!);
  const project = useProjectStore.getState();
  if (project.doc && !project.hasItem(path)) {
    project.addItem(path);
    await project.save();
  }
  return true;
}

export async function saveAll(): Promise<boolean> {
  for (const id of useDocumentsStore.getState().order) {
    const doc = useDocumentsStore.getState().docs[id]!;
    if (isFileDocument(doc) && isDocDirty(doc) && !(await saveDocument(id))) return false;
  }
  const project = useProjectStore.getState();
  if (project.dirty) await project.save();
  return true;
}

/** Closes the tab; when dirty asks Save / Don't Save / Cancel. Returns false when cancelled. */
export async function closeDocument(id: string): Promise<boolean> {
  const docs = useDocumentsStore.getState();
  const doc = docs.docs[id];
  if (!doc) return true;
  if (isFileDocument(doc) && isDocDirty(doc)) {
    const choice = await getApi().dialog.message({
      type: 'question',
      message: `Save changes to ${docTitle(doc)}?`,
      buttons: ['Save', "Don't Save", 'Cancel'],
      defaultId: 0,
      cancelId: 2,
    });
    if (choice === 2) return false;
    if (choice === 0 && !(await saveDocument(id))) return false;
  }
  useDocumentsStore.getState().close(id);
  return true;
}

/** Closes every tab with prompts; used before closing the project or the window. */
export async function closeAllDocuments(): Promise<boolean> {
  for (const id of [...useDocumentsStore.getState().order]) {
    if (!useDocumentsStore.getState().docs[id]) continue; // already closed as a dependant
    if (!(await closeDocument(id))) return false;
  }
  return true;
}

export async function closeProject(): Promise<boolean> {
  if (!(await closeAllDocuments())) return false;
  const project = useProjectStore.getState();
  if (project.dirty) {
    const choice = await getApi().dialog.message({
      type: 'question',
      message: `Save changes to project ${project.doc?.name ?? ''}?`,
      buttons: ['Save', "Don't Save", 'Cancel'],
      defaultId: 0,
      cancelId: 2,
    });
    if (choice === 2) return false;
    if (choice === 0) await project.save();
  }
  project.close();
  return true;
}

export async function openProjectDialog(): Promise<boolean> {
  const path = await getApi().dialog.openFile({ title: 'Open Project', filters: FILTERS.project });
  if (!path) return false;
  if (!(await closeProject())) return false;
  return useProjectStore.getState().openProject(path);
}

export async function newProjectDialog(): Promise<boolean> {
  const dir = await getApi().dialog.pickFolder({ title: 'Choose a folder for the new project' });
  if (!dir) return false;
  if (!(await closeProject())) return false;
  const name = dir.split(/[\\/]/).filter(Boolean).pop() ?? 'Project1';
  return useProjectStore.getState().newProject(dir, name);
}

/** Opens a project item by its relative path. */
export async function openProjectItem(rel: string): Promise<string> {
  return openFile(useProjectStore.getState().resolvePath(rel));
}
