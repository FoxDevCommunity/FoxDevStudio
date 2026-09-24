/**
 * Supplies the running session with code from the IDE.
 *
 * Open documents win over files on disk, so Run Form reflects unsaved edits the way the
 * Milestone-1 preview did. VFP compiles the saved file instead; the status line says which.
 */

import { parseFormDocument } from '@shared/form/serialize';
import { parseMenuDocument } from '@shared/menu/serialize';
import type { FormDocument } from '@shared/form/schema';
import type { MenuDocument } from '@shared/menu/schema';
import { basename, dirname } from '@shared/paths';
import {
  CompileCache,
  baseName,
  formHeaderRefs,
  formMethodSources,
  headerStem,
  requireBytes,
  type CompiledForm,
  type CompiledProgram,
  type ProgramSource,
} from '@shared/runtime/programSource';
import { getApi } from '../api/foxdev';
import { bundledClassLibraryDir } from '../vfp/classLibraries';
import { refreshed } from '../vfp/refreshImport';
import { useDocumentsStore, type OpenDocument } from '../stores/documentsStore';
import { useProjectStore } from '../stores/projectStore';
import { readHeaderFiles } from './headerFiles';
import { compileForm, compileProgram, includedHeaderNames } from './vmBridge';

/** Finds an open document whose file name matches, ignoring directory and extension. */
function openDocumentNamed(name: string, kind: OpenDocument['kind']): OpenDocument | undefined {
  const wanted = baseName(name).toLowerCase();
  return Object.values(useDocumentsStore.getState().docs).find((d) => {
    if (d.kind !== kind || !('path' in d)) return false;
    if (d.path && baseName(basename(d.path)).toLowerCase() === wanted) return true;
    return d.kind === 'form' && d.store.getState().doc.form.name.toLowerCase() === wanted;
  });
}

/** Absolute path of a project item whose file name matches, if the project lists one. */
function projectItemPath(name: string, extensions: string[]): string | null {
  const project = useProjectStore.getState();
  if (!project.doc) return null;
  const wanted = baseName(name).toLowerCase();
  const item = project.doc.items.find((i) => {
    const file = basename(i.path);
    return extensions.some((e) => file.toLowerCase().endsWith(e)) && baseName(file).toLowerCase() === wanted;
  });
  return item ? project.resolvePath(item.path) : null;
}

/**
 * The text of every header file a program includes, by name without folder or extension.
 *
 * A header the project lists is taken from there. The rest are looked for the way a form's are:
 * beside the program, then in the default directory - the project folder - then among the
 * Foundation Classes, following the headers they include in turn. A Visual FoxPro project
 * rarely lists its `.h` files at all, so without the search every constant an application
 * defines is an unknown name when the line that uses it runs. One found nowhere is left out,
 * and the compiler warns about it by name.
 */
async function headerFiles(source: string, dir?: string): Promise<Record<string, string>> {
  const out: Record<string, string> = {};
  const unlisted: string[] = [];
  for (const reference of includedHeaderNames(source)) {
    const stem = headerStem(reference);
    if (stem === '' || stem in out) continue;
    // the extension the reference wrote is the file it means: `#include "CodeMine.h"` in
    // codemine.prg is the header beside it, and never the program itself, whose text would
    // include the header again
    const file = reference.trim().split(/[\\/]/).pop() ?? '';
    const dot = file.lastIndexOf('.');
    const extensions = dot > 0 ? [file.slice(dot).toLowerCase()] : ['.h', '.prg'];
    const path = projectItemPath(stem, extensions);
    if (path) out[stem] = await getApi().files.readText(path).catch(() => '');
    else unlisted.push(reference);
  }
  if (unlisted.length > 0) {
    const project = useProjectStore.getState();
    const dirs = [dir ?? '', project.path ? dirname(project.path) : '', await bundledClassLibraryDir()].filter((d) => d !== '');
    const found = await readHeaderFiles(unlisted, dirs.length > 0 ? dirs : ['']);
    for (const [stem, text] of Object.entries(found)) if (!(stem in out)) out[stem] = text;
  }
  return out;
}

/**
 * The text of every header file a form's methods are compiled with.
 *
 * Visual FoxPro looks for one beside the file that named it and then in the default directory,
 * which here is the project folder; a control taken from a class library brings a reference that
 * already leads to that library's folder.
 */
async function formHeaders(doc: FormDocument, dir?: string): Promise<Record<string, string>> {
  const references = formHeaderRefs(doc);
  if (references.length === 0) return {};
  const project = useProjectStore.getState();
  // and the Foundation Classes that ship with FoxDev last, because `foxpro.h` and the headers
  // beside those libraries are ones Visual FoxPro finds in its own installation rather than in
  // anything the project can point at
  const dirs = [dir ?? '', project.path ? dirname(project.path) : '', await bundledClassLibraryDir()].filter((d) => d !== '');
  return readHeaderFiles(references, dirs.length > 0 ? dirs : ['']);
}

export function createProjectSource(): ProgramSource {
  const forms = new CompileCache<CompiledForm>();
  const programs = new CompileCache<CompiledProgram>();

  const compileFormDoc = async (name: string, doc: FormDocument, cacheKey: string, dir?: string): Promise<CompiledForm> => {
    const cached = forms.get(name, cacheKey);
    if (cached) return cached;
    const bytes = requireBytes(doc.form.name, compileForm(doc.form.name, formMethodSources(doc), await formHeaders(doc, dir)));
    return forms.set(name, cacheKey, { name: doc.form.name, doc, bytes });
  };

  return {
    async getForm(name) {
      const open = openDocumentNamed(name, 'form');
      if (open?.kind === 'form') {
        const doc = open.store.getState().doc;
        return compileFormDoc(name, doc, `open:${open.id}:${open.store.getState().history.past.length}`, open.path ? dirname(open.path) : undefined);
      }
      const path = projectItemPath(name, ['.fxf', '.scx']);
      if (!path) return null;
      const text = await getApi().files.readText(path);
      const parsed = parseFormDocument(text);
      if (!parsed.ok) throw new Error(`${name}: ${parsed.error}`);
      // running a form built by an older importer would repeat a fault that is already fixed
      const doc = await refreshed(parsed.doc, path);
      return compileFormDoc(name, doc, `file:${text.length}:${text.slice(0, 64)}`, dirname(path));
    },

    async getProgram(name) {
      const open = openDocumentNamed(name, 'program');
      let source: string;
      let key: string;
      let display = baseName(name);
      let dir: string | undefined;

      if (open?.kind === 'program') {
        source = open.text;
        key = `open:${open.id}:${open.text.length}:${open.text.slice(0, 64)}`;
        if (open.path) {
          display = baseName(basename(open.path));
          dir = dirname(open.path);
        }
      } else {
        const path = projectItemPath(name, ['.prg']);
        if (!path) return null;
        source = await getApi().files.readText(path);
        key = `file:${source.length}:${source.slice(0, 64)}`;
        display = baseName(basename(path));
        dir = dirname(path);
      }

      const headers = await headerFiles(source, dir);
      const cached = programs.get(display, `${key}:${Object.keys(headers).join(',')}`);
      if (cached) return cached;
      const bytes = requireBytes(display, compileProgram(source, display, headers));
      return programs.set(display, `${key}:${Object.keys(headers).join(',')}`, { name: display, bytes });
    },

    async getMenu(name): Promise<MenuDocument | null> {
      const open = openDocumentNamed(name, 'menu');
      if (open?.kind === 'menu') return open.store.getState().doc;
      const path = projectItemPath(name, ['.fxm', '.mnx']);
      if (!path) return null;
      const parsed = parseMenuDocument(await getApi().files.readText(path));
      return parsed.ok ? parsed.doc : null;
    },
  };
}
