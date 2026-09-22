/**
 * Converting a document again when the importer has learned something since.
 *
 * A `.fxf` is the result of running the importer over a `.scx`, and it is only as good as the
 * importer that made it. When the importer is fixed - to keep a custom property it used to drop,
 * say - every project imported before the fix still carries the old, lossy document, and fails in
 * exactly the way that was just fixed. Telling people to import again is not an answer: they have
 * no way to know when it matters.
 *
 * So each converted document records the version that made it and the file it came from. Opening
 * one made by an older importer converts it again from that file, in place. It costs one read of
 * a `.scx` the first time each document is opened after an upgrade, and nothing after that.
 */

import { dirname, join } from '@shared/paths';
import { IMPORTER_VERSION, importFormFile } from '@shared/vfp/importForm';
import { stringifyFormDocument } from '@shared/form/serialize';
import type { FormDocument } from '@shared/form/schema';
import type { DbfReadResult, DbfTableData } from '@shared/vfp/dbfTypes';
import { getApi } from '../api/foxdev';
import { loadFoxVm } from '../../../wasm/foxvm/loader';
import { loadClassLibraries, reachable } from './classLibraries';

/** The memo file that goes with each design format. */
const MEMO: Record<string, string> = { scx: 'sct', vcx: 'vct', mnx: 'mnt', pjx: 'pjt' };

/**
 * The document to use for `path`, converting it again first when an older importer made it.
 *
 * Anything that stops the re-conversion - the Visual FoxPro file has been moved, the folder is
 * read-only, the file no longer parses - leaves the document exactly as it was. A stale document
 * is worse than a fresh one and better than none.
 */
export async function refreshed(doc: FormDocument, path: string): Promise<FormDocument> {
  const vfp = doc.meta?.vfp;
  if ((vfp?.importer ?? 0) >= IMPORTER_VERSION) return doc;

  try {
    const source = await findSource(doc, path);
    if (!source) return doc;

    const table = await readTable(source);
    const { libraries } = await loadClassLibraries(table, dirname(source), readTable, []);
    const stem = source.replace(/^.*[\\/]/, '').replace(/\.[^.]+$/, '');
    const forms = importFormFile(table, stem, libraries);
    // a formset produced several documents; the one being opened is the one whose form matches
    const match = forms.find((f) => f.imported.doc.form.name.toLowerCase() === doc.form.name.toLowerCase()) ?? forms[0];
    if (!match) return doc;

    // A .vcx has no form in it, and importing one as a form gives an empty document back.
    // Writing that over the document being opened would throw the user's file away, so a
    // conversion that found no form leaves what is already there alone.
    if (match.imported.warnings.some((w) => w.kind === 'noForm')) return doc;

    const fresh = match.imported.doc;
    fresh.meta = { ...fresh.meta, vfp: { ...fresh.meta?.vfp, source: vfp?.source ?? source.replace(/^.*[\\/]/, '') } };
    await getApi().files.writeText(path, stringifyFormDocument(fresh));
    return fresh;
  } catch {
    return doc;
  }
}

/**
 * The Visual FoxPro file a document came from.
 *
 * A document imported recently says so. One imported before the importer started saying so - which
 * is every document in every project that already exists - does not, and for those the file is
 * looked for beside the document under the same name, because that is where the importer writes
 * its output. Without that fallback the refresh would help only projects imported after it, which
 * is nobody who already has one.
 */
async function findSource(doc: FormDocument, path: string): Promise<string | null> {
  const dir = dirname(path);
  const recorded = doc.meta?.vfp?.source;
  const candidates = recorded ? [join(dir, recorded.replace(/^.*[\\/]/, '')), join(dir, recorded)] : [];

  // `Form1.fxf` came from `Form1.scx`; a class document is named `lib.ClassName.fxf`
  const file = path.replace(/^.*[\\/]/, '');
  const stem = file.replace(/\.fxf$/i, '');
  const roots = [stem, stem.replace(/\.[^.]+$/, '')];
  for (const root of roots) {
    for (const ext of ['scx', 'vcx']) {
      candidates.push(join(dir, `${root}.${ext}`));
    }
  }

  for (const candidate of candidates) {
    if (await reachable(candidate)) return candidate;
  }
  return null;
}

/** Reads a DBF and the memo its own extension calls for. */
async function readTable(path: string): Promise<DbfTableData> {
  const vm = await loadFoxVm();
  const api = getApi();
  const bytes = await api.files.readBytes(path);

  const dir = dirname(path);
  const file = path.replace(/^.*[\\/]/, '');
  const stem = file.replace(/\.[^.]+$/, '').toLowerCase();
  const want = MEMO[file.slice(file.lastIndexOf('.') + 1).toLowerCase()] ?? 'fpt';
  let memo: Uint8Array | undefined;
  try {
    const names = await api.files.listDir(dir);
    const match = names.find((n) => {
      const dot = n.lastIndexOf('.');
      return dot > 0 && n.slice(0, dot).toLowerCase() === stem && n.slice(dot + 1).toLowerCase() === want;
    });
    if (match) memo = await api.files.readBytes(join(dir, match));
  } catch {
    // a table with no memo beside it reads as one without memo fields
  }

  const result = vm.read_dbf(bytes, memo) as DbfReadResult;
  if (!result.ok) throw new Error(result.error);
  return result;
}
