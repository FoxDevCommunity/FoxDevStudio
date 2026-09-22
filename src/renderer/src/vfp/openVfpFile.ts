/**
 * Opening a Visual FoxPro document directly, without a project around it.
 *
 * Plenty of VFP code lives as loose `.scx` and `.vcx` files rather than inside a `.pjx`, so
 * File > Open accepts them and converts on the way in. The converted document is opened as an
 * untitled tab: it is only written to disk when you save, which keeps the originals untouched.
 */

import { importClassLibrary, importFormFile } from '@shared/vfp/importForm';
import { importMenuTable } from '@shared/vfp/importMenu';
import { bundledClassLibraryDir, loadClassLibraries } from './classLibraries';
import { basename, dirname, extname, join } from '@shared/paths';
import type { DbfReadResult, DbfTableData } from '@shared/vfp/dbfTypes';
import { getApi } from '../api/foxdev';
import { useDocumentsStore } from '../stores/documentsStore';
import { useSessionStore } from '../runtime/session';
import { loadFoxVm } from '../../../wasm/foxvm/loader';

/** Extensions File > Open will convert rather than read as text. */
export const VFP_EXTENSIONS = ['scx', 'vcx', 'pjx', 'mnx'];

/** DBF-based files that hold data rather than a design, and open in the table browser. */
export const TABLE_EXTENSIONS = ['dbf', 'dbc'];

export function isTableFile(path: string): boolean {
  return TABLE_EXTENSIONS.includes(extname(path).toLowerCase().replace('.', ''));
}

export function isVfpFile(path: string): boolean {
  return VFP_EXTENSIONS.includes(extname(path).toLowerCase().replace('.', ''));
}

/** VFP pairs each DBF-based format with its own memo extension. */
function memoExtensionFor(path: string): string {
  switch (extname(path).toLowerCase()) {
    case '.scx':
      return 'sct';
    case '.vcx':
      return 'vct';
    case '.mnx':
      return 'mnt';
    case '.pjx':
      return 'pjt';
    default:
      return 'fpt';
  }
}

/** Reads a DBF and its memo sibling, whatever case the file system happens to use. */
export async function readVfpTable(path: string): Promise<DbfTableData> {
  const api = getApi();
  const bytes = await api.files.readBytes(path);

  const stem = basename(path).replace(/\.[^.]+$/, '').toLowerCase();
  const wanted = memoExtensionFor(path);
  let memo: Uint8Array | undefined;
  try {
    const names = await api.files.listDir(dirname(path));
    const match = names.find((n) => {
      const dot = n.lastIndexOf('.');
      return dot > 0 && n.slice(0, dot).toLowerCase() === stem && n.slice(dot + 1).toLowerCase() === wanted;
    });
    if (match) memo = await api.files.readBytes(join(dirname(path), match));
  } catch {
    // a table with no memo file still parses; its memo fields come back empty
  }

  const vm = await loadFoxVm();
  const result = vm.read_dbf(bytes, memo) as DbfReadResult;
  if (!result.ok) throw new Error(`${basename(path)}: ${result.error}`);
  return result;
}

/**
 * Opens a `.scx` as a form - or as one tab per form when it holds a formset - and a `.vcx` as one
 * tab per class it defines. Returns the id of the first tab opened.
 */
export async function openVfpFile(path: string): Promise<string> {
  const print = useSessionStore.getState().print;
  const table = await readVfpTable(path);
  const stem = basename(path).replace(/\.[^.]+$/, '');
  const docs = useDocumentsStore.getState();

  if (extname(path).toLowerCase() === '.mnx') {
    const { doc, warnings } = importMenuTable(table, stem);
    const id = docs.openMenu(doc, null);
    for (const w of warnings) print({ kind: 'error', text: `${basename(path)} (${w.object}): ${w.message}` });
    print({ kind: 'output', text: `Imported ${basename(path)} as menu ${doc.name}. Save it to keep the conversion.` });
    return id;
  }

  // controls stamped from a class need the library they came from, or they import empty
  const { libraries, missing } = await loadClassLibraries(table, dirname(path), readVfpTable, [await bundledClassLibraryDir()]);
  for (const ref of missing) print({ kind: 'error', text: `${basename(path)}: class library "${ref}" was not found` });

  if (extname(path).toLowerCase() === '.vcx') {
    const classes = importClassLibrary(table, stem, libraries);
    if (classes.length === 0) throw new Error(`${basename(path)}: no class definitions found`);
    let first: string | null = null;
    for (const entry of classes) {
      const id = docs.openForm(entry.imported.doc, null);
      first ??= id;
      for (const w of entry.imported.warnings) print({ kind: 'error', text: `${entry.className} (${w.object}): ${w.message}` });
    }
    print({ kind: 'output', text: `Imported ${classes.length} class(es) from ${basename(path)}` });
    return first!;
  }

  // a formset holds several forms and a form is what the designer draws, so one file opens as
  // several documents, the way a class library does
  const forms = importFormFile(table, stem, libraries);
  let first: string | null = null;
  for (const entry of forms) {
    const id = docs.openForm(entry.imported.doc, null);
    first ??= id;
    for (const w of entry.imported.warnings) print({ kind: 'error', text: `${basename(path)} (${w.object}): ${w.message}` });
  }
  const formset = forms[0]?.imported.doc.meta?.vfp?.formset;
  const named = formset ? `formset ${formset.name}: ${formset.forms.join(', ')}` : (forms[0]?.imported.doc.form.name ?? stem);
  print({ kind: 'output', text: `Imported ${basename(path)} as ${named}. Save ${formset ? 'them' : 'it'} to keep the conversion.` });
  return first!;
}
