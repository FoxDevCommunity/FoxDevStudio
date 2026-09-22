/**
 * File > Import Visual FoxPro Project: converts a `.pjx` and everything it can into FoxDev
 * documents written beside the originals, then opens the result.
 *
 * Nothing is overwritten and the VFP files are left untouched, so an import is repeatable and
 * you can keep working in VFP alongside.
 */

import { stringifyProjectDocument } from '@shared/project/serialize';
import { basename, dirname, join } from '@shared/paths';
import { readProjectTable, targetPath, toProjectDocument, type ImportedItem, type ImportedProject } from '@shared/vfp/importProject';
import { importFormFile } from '@shared/vfp/importForm';
import { importClassLibraryDocument } from '@shared/vfp/importClass';
import { stringifyClassLibraryDocument } from '@shared/classlib/serialize';
import { importMenuTable } from '@shared/vfp/importMenu';
import { stringifyMenuDocument } from '@shared/menu/serialize';
import { bundledClassLibraryDir, loadClassLibraries, reachable } from './classLibraries';
import { stringifyFormDocument } from '@shared/form/serialize';
import { formsetDocumentName } from '@shared/form/formset';
import type { DocumentMeta } from '@shared/form/schema';
import type { DbfReadResult, DbfTableData } from '@shared/vfp/dbfTypes';
import { getApi } from '../api/foxdev';
import { useSessionStore } from '../runtime/session';
import { loadFoxVm } from '../../../wasm/foxvm/loader';

/**
 * Whether an item's file can be read, or why not.
 *
 * A `.pjx` records paths relative to wherever the developer's project sat, and plenty of them
 * climb out of the project's own folder - solution.pjx refers to `..\..\classes\dragmove.cur`.
 * The session is only given the project folder, so the path guard refuses those. That is an
 * answer about one item, not a reason to abandon the other two hundred.
 */
async function available(path: string): Promise<true | string> {
  return (await reachable(path)) || 'file not found or outside the project folder';
}

/** Item kinds the IDE can actually open; a project with none of them has nothing to show. */
const OPENABLE = new Set(['form', 'menu', 'class', 'program']);

export interface ImportReport {
  projectPath: string;
  converted: string[];
  referenced: string[];
  skipped: { path: string; reason: string }[];
}

/** Reads a DBF and its memo sibling, whatever case the memo extension happens to have. */
async function readTable(path: string): Promise<DbfTableData> {
  const api = getApi();
  const bytes = await api.files.readBytes(path);

  const dir = dirname(path);
  const stem = basename(path).replace(/\.[^.]+$/, '');
  const memoExtensions = memoExtensionsFor(path);
  let memo: Uint8Array | undefined;
  try {
    const names = await api.files.listDir(dir);
    const match = names.find((n) => {
      const dot = n.lastIndexOf('.');
      if (dot < 0) return false;
      return n.slice(0, dot).toLowerCase() === stem.toLowerCase() && memoExtensions.includes(n.slice(dot + 1).toLowerCase());
    });
    if (match) memo = await api.files.readBytes(join(dir, match));
  } catch {
    // a project with no memo file is unusual but not fatal
  }

  const vm = await loadFoxVm();
  const result = vm.read_dbf(bytes, memo) as DbfReadResult;
  if (!result.ok) throw new Error(`${basename(path)}: ${result.error}`);
  return result;
}

/** VFP pairs each DBF-based format with its own memo extension. */
function memoExtensionsFor(path: string): string[] {
  const ext = path.slice(path.lastIndexOf('.') + 1).toLowerCase();
  switch (ext) {
    case 'pjx':
      return ['pjt'];
    case 'scx':
      return ['sct'];
    case 'vcx':
      return ['vct'];
    case 'mnx':
      return ['mnt'];
    case 'frx':
      return ['frt'];
    default:
      return ['fpt'];
  }
}

/**
 * Imports the project at `pjxPath`. Programs and other plain files are referenced where they
 * are; DBF-based documents are converted next to the original.
 */
export async function importVfpProject(pjxPath: string): Promise<ImportReport> {
  const print = useSessionStore.getState().print;
  const api = getApi();
  const dir = dirname(pjxPath);

  const table = await readTable(pjxPath);
  const imported: ImportedProject = readProjectTable(table, pjxPath);
  print({ kind: 'output', text: `Importing ${basename(pjxPath)}: ${imported.items.length} item(s)` });

  const converted = new Set<string>();
  const referenced: string[] = [];
  // The project's own folder, then the Foundation Classes that ship with FoxDev, which is the
  // only place a `CLASSLOC` of `_base.vcx` can be found. Grows as libraries turn up elsewhere.
  const searchDirs: string[] = [dir, await bundledClassLibraryDir()];
  const skipped = [...imported.skipped];

  for (const item of imported.items) {
    const source = join(dir, item.sourcePath);
    if (!item.needsConversion) {
      const found = await available(source);
      if (found === true) referenced.push(item.sourcePath);
      else skipped.push({ path: item.sourcePath, reason: found });
      continue;
    }
    const outcome = await convertItem(item, dir, searchDirs);
    if (outcome.ok) {
      converted.add(item.sourcePath);
      print({ kind: 'output', text: `Converted ${item.sourcePath} -> ${targetPath(item)}` });
    } else {
      skipped.push({ path: item.sourcePath, reason: outcome.reason });
    }
  }

  const projectPath = join(dir, `${imported.name}.fxproject`);
  const doc = toProjectDocument(imported, converted);
  await api.files.writeText(projectPath, stringifyProjectDocument(doc));

  for (const entry of skipped) print({ kind: 'error', text: `Skipped ${entry.path}: ${entry.reason}` });
  print({ kind: 'output', text: `Wrote ${projectPath}` });
  // A project can list nothing FoxDev opens - VFP ships stubs that track a bitmap and little
  // else - and an empty explorer looks like a failed import unless the reason is said out loud.
  if (!doc.items.some((i) => OPENABLE.has(i.kind))) {
    print({
      kind: 'error',
      text: `${basename(pjxPath)} lists ${imported.items.length} item(s) and none of them is a form, menu, class library or program, so the project opens empty. Its other files are still listed.`,
    });
  }

  return { projectPath, converted: [...converted], referenced, skipped };
}

/** Notes the Visual FoxPro file a document was converted from, relative to the project folder. */
function withSource(doc: { meta?: DocumentMeta }, sourcePath: string): void {
  doc.meta = { ...doc.meta, vfp: { ...doc.meta?.vfp, source: sourcePath } };
}

/** Converts one DBF-based item, writing the FoxDev document beside the original. */
async function convertItem(
  item: ImportedItem,
  dir: string,
  searchDirs: string[],
): Promise<{ ok: true } | { ok: false; reason: string }> {
  const print = useSessionStore.getState().print;
  const source = join(dir, item.sourcePath);
  const found = await available(source);
  if (found !== true) return { ok: false, reason: found };

  try {
    const table = await readTable(source);
    const stem = basename(item.sourcePath).replace(/.[^.]+$/, '');

    if (item.kind === 'menu') {
      const { doc, warnings } = importMenuTable(table, stem);
      await getApi().files.writeText(join(dir, targetPath(item)), stringifyMenuDocument(doc));
      for (const w of warnings) print({ kind: 'error', text: `${item.sourcePath} (${w.object}): ${w.message}` });
      return { ok: true };
    }

    // a control stamped from a class library needs that library to import as more than a shell
    const { libraries, missing, found } = await loadClassLibraries(table, dirname(source), readTable, searchDirs);
    // where a library turned up is worth trying first for the rest of the project
    for (const d of found) if (!searchDirs.includes(d)) searchDirs.push(d);
    for (const ref of missing) print({ kind: 'error', text: `${item.sourcePath}: class library "${ref}" was not found` });

    if (item.kind === 'form') {
      // a formset holds more than one form, and each of them is a window of its own
      const forms = importFormFile(table, stem, libraries);
      for (const [i, entry] of forms.entries()) {
        const named = formsetDocumentName(stem, entry.formName, i);
        const target = targetPath(item).replace(/[^\\/]+\.fxf$/i, `${named}.fxf`);
        await getApi().files.writeText(join(dir, target), stringifyFormDocument(entry.imported.doc));
        if (i > 0) print({ kind: 'output', text: `Converted ${item.sourcePath} -> ${target}` });
        for (const w of entry.imported.warnings) print({ kind: 'error', text: `${item.sourcePath} (${w.object}): ${w.message}` });
      }
      return { ok: true };
    }

    if (item.kind === 'class') {
      // a class library is one file of classes here as it is there: one .vcx, one .fxc
      const { doc, warnings } = importClassLibraryDocument(table, stem, libraries);
      if (doc.classes.length === 0) return { ok: false, reason: 'no class definitions found' };
      withSource(doc, item.sourcePath);
      await getApi().files.writeText(join(dir, targetPath(item)), stringifyClassLibraryDocument(doc));
      for (const w of warnings) print({ kind: 'error', text: `${item.sourcePath} (${w.object}): ${w.message}` });
      return { ok: true };
    }

    return { ok: false, reason: `${item.kind} items are not converted` };
  } catch (err) {
    return { ok: false, reason: err instanceof Error ? err.message : String(err) };
  }
}
