/**
 * Finding the header files a form, a class library or a program names.
 *
 * A `.scx` names one header for the whole file and a `.vcx` one per class; the `#DEFINE`s in it
 * are in scope in every method that file holds. Visual FoxPro resolves the name when it compiles
 * the file, and where it looks was measured in the product: beside the file that named it first,
 * then the default directory. A name it finds nowhere is not an error - the file compiles, and
 * the constants in it are simply unknown names when a line that uses one runs.
 *
 * A header may `#INCLUDE` another, and that one is looked for beside the header that asked for
 * it, so the whole chain comes back from one call.
 */

import { basename, dirname, isAbsolute, join, resolveFrom } from '@shared/paths';
import { headerStem } from '@shared/runtime/programSource';
import { getApi } from '../api/foxdev';
import { includedHeaderNames } from './vmBridge';

/**
 * The text of every header in `references` and of every header those include in turn, keyed by
 * the name `#INCLUDE` asks for: the stem, upper-cased. `dirs` are searched in order, and a
 * reference is always tried against the folder of the file that named it first - which is what
 * the caller puts at the front of `dirs`.
 *
 * Two headers of the same stem in different folders would collide in one map; the first read
 * wins, as it does when the compiler is handed a program's headers.
 */
export async function readHeaderFiles(references: readonly string[], dirs: readonly string[]): Promise<Record<string, string>> {
  const out: Record<string, string> = {};
  const queue = references.map((reference) => ({ reference, from: dirs }));

  while (queue.length > 0) {
    const { reference, from } = queue.shift()!;
    const stem = headerStem(reference);
    if (stem === '' || stem in out) continue;
    const path = await locate(reference, from);
    if (path === null) continue;
    const text = await getApi()
      .files.readText(path)
      .catch(() => '');
    out[stem] = text;
    // and whatever it includes, looked for beside it before anywhere the caller named
    const beside = dirname(path);
    for (const nested of includedHeaderNames(text)) queue.push({ reference: nested, from: [beside, ...from] });
  }
  return out;
}

/** Where a reference really is, or `null`. A full path stands on its own; the rest are searched. */
async function locate(reference: string, dirs: readonly string[]): Promise<string | null> {
  const posix = reference.trim().replace(/\\/g, '/');
  if (posix === '') return null;
  const candidates = isAbsolute(posix)
    ? [posix]
    : // the reference as written counts from each folder in turn, and its bare file name after
      // that, because a path stored when the project sat somewhere else rarely still leads there
      [...dirs.map((dir) => resolveFrom(dir, posix)), ...dirs.map((dir) => join(dir, basename(posix)))];

  for (const candidate of candidates) {
    if (await getApi().files.exists(candidate).catch(() => false)) return candidate;
    // Visual FoxPro files are often stored upper-cased; the file system may or may not care
    const upper = join(dirname(candidate), basename(candidate).toUpperCase());
    if (upper !== candidate && (await getApi().files.exists(upper).catch(() => false))) return upper;
  }
  return null;
}
