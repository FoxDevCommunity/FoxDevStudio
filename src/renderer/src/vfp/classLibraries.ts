/**
 * Finding the `.vcx` files a Visual FoxPro form was built from.
 *
 * A form stores one row per control with a `CLASSLOC` naming the library its class came from,
 * as a path relative to wherever the developer's project happened to sit: `..\utils\coolstuf.vcx`
 * is typical. Those paths rarely survive a move, so the file name is tried in the form's own
 * directory as well, which is where a copied project usually keeps them.
 *
 * A Foundation Class carries no path at all: `CLASSLOC` is `_base.vcx`, because Visual FoxPro
 * finds those on its own search path. Copies of them ship with FoxDev for exactly that reason,
 * and the folder holding them is the last place searched.
 */

import { basename, dirname, join, resolveFrom } from '@shared/paths';
import { classLibrariesReferenced, type VfpClassLibrary } from '@shared/vfp/importForm';
import type { DbfTableData } from '@shared/vfp/dbfTypes';
import { getApi } from '../api/foxdev';

/**
 * Whether a file can be read, asking for access to its folder if that has not been granted yet.
 *
 * A Visual FoxPro project refers to files outside the folder it sits in, so the session is given
 * only the project directory and has to ask for the rest. Asking is best effort: a refusal means
 * the file is reported as outside the project and everything else still imports.
 */
export async function reachable(path: string): Promise<boolean> {
  try {
    if (await getApi().files.exists(path)) return true;
  } catch {
    // not granted yet, which is what the request below is for
  }
  try {
    if (!(await getApi().project.allowNear(path))) return false;
    return await getApi().files.exists(path);
  } catch {
    return false;
  }
}

/**
 * The folder holding the Foundation Classes that ship with FoxDev. Asking also grants access to
 * it; an empty string when the host has none, which is what the in-memory API used by tests does.
 */
export async function bundledClassLibraryDir(): Promise<string> {
  try {
    return await getApi().project.classLibraryDir();
  } catch {
    return '';
  }
}

export interface LoadedClassLibraries {
  libraries: VfpClassLibrary[];
  /** References that could not be found; the importer warns per control anyway. */
  missing: string[];
  /** Directories a library was actually found in, worth trying first next time. */
  found: string[];
}

/**
 * Loads every library `table` refers to, and every library those refer to in turn.
 *
 * The transitive step is not an optimisation: `buttons.vcx` builds its `mailbtn` from
 * `_commandbutton` in `_base.vcx`, and nothing at the form level ever names `_base.vcx`. Without
 * following the libraries' own references, a control two classes deep arrives empty however many
 * libraries are on hand.
 *
 * `searchDirs` are tried after the reference's own path, and grow as libraries are found, so a
 * project that names the Foundation Classes once teaches the rest of the import where they live.
 */
export async function loadClassLibraries(
  table: DbfTableData,
  baseDir: string,
  readTable: (path: string) => Promise<DbfTableData>,
  searchDirs: readonly string[] = [],
): Promise<LoadedClassLibraries> {
  const libraries: VfpClassLibrary[] = [];
  const missing: string[] = [];
  const dirs = [...searchDirs];
  const seen = new Set<string>();
  const queue = classLibrariesReferenced(table).map((ref) => ({ ref, from: baseDir }));

  while (queue.length > 0) {
    const { ref, from } = queue.shift()!;
    // libraries are identified by file name: the same one is reached by many different paths
    const key = basename(ref.replace(/\\/g, '/')).toLowerCase();
    if (seen.has(key)) continue;
    seen.add(key);

    const path = await locate(ref, from, dirs);
    if (!path) {
      missing.push(ref);
      continue;
    }
    try {
      const loaded = await readTable(path);
      libraries.push({ name: ref, table: loaded, path });
      const dir = dirname(path);
      if (!dirs.includes(dir)) dirs.push(dir);
      for (const next of classLibrariesReferenced(loaded)) queue.push({ ref: next, from: dir });
    } catch {
      missing.push(ref);
    }
  }
  return { libraries, missing, found: dirs };
}

async function locate(ref: string, baseDir: string, searchDirs: readonly string[]): Promise<string | null> {
  const posix = ref.replace(/\\/g, '/');
  const file = basename(posix);
  const candidates = [resolveFrom(baseDir, posix), join(baseDir, file), ...searchDirs.map((d) => join(d, file))];

  for (const candidate of candidates) {
    if (await reachable(candidate)) return candidate;
    // Visual FoxPro files are often stored upper-cased; the file system may or may not care
    const upper = join(dirname(candidate), basename(candidate).toUpperCase());
    if (upper !== candidate && (await reachable(upper))) return upper;
  }
  return null;
}

