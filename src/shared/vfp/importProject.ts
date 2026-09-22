/**
 * Reads a Visual FoxPro project table (`.pjx`) and turns it into a FoxDev project.
 *
 * A `.pjx` is a DBF whose rows are the project's items: `NAME` holds the file path (relative
 * to the project, or absolute), `TYPE` a one-letter kind, `MAINPROG` marks the item VFP's
 * "Set Main" points at, and `EXCLUDE` the ones left out of a build. The first row is a header
 * describing the project itself and has no file.
 */

import { kindForPath, type ProjectDocument, type ProjectItem, type ProjectItemKind } from '../project/schema';
import { basename, normalize } from '../paths';
import { flag, text, type DbfTableData } from './dbfTypes';

/** VFP's project item type letters. Lower case marks an item VFP generated. */
const TYPE_KINDS: Record<string, ProjectItemKind> = {
  P: 'program',
  K: 'form',
  V: 'class',
  M: 'menu',
  D: 'database',
  d: 'table',
  R: 'report',
  B: 'other', // label
  Q: 'other', // query
  T: 'other', // text
  H: 'other', // header
  L: 'other', // API library
  Z: 'other', // image and other binaries
};

/** Extension a converted item ends up with, when we can convert it. */
const CONVERTED_EXTENSIONS: Partial<Record<ProjectItemKind, string>> = {
  form: '.fxf',
  menu: '.fxm',
  class: '.fxc',
};

export interface ImportedItem {
  /** Path as VFP recorded it, with backslashes normalised to forward slashes. */
  sourcePath: string;
  kind: ProjectItemKind;
  excluded: boolean;
  main: boolean;
  /** True when the source is a DBF-based file that has to be converted, not just referenced. */
  needsConversion: boolean;
}

export interface ImportedProject {
  name: string;
  items: ImportedItem[];
  /** Items VFP listed that this importer does not understand, for the report. */
  skipped: { path: string; reason: string }[];
}

/** Reads the project table. Throws only when the table is not a project at all. */
export function readProjectTable(table: DbfTableData, projectFileName: string): ImportedProject {
  if (!table.fields.some((f) => f.name.toUpperCase() === 'TYPE') || !table.fields.some((f) => f.name.toUpperCase() === 'NAME')) {
    throw new Error('This file is not a Visual FoxPro project table.');
  }

  const items: ImportedItem[] = [];
  const skipped: { path: string; reason: string }[] = [];

  for (const record of table.records) {
    if (record.deleted) continue;
    const raw = text(table, record, 'NAME');
    if (!raw) continue;

    const sourcePath = normalize(raw).replace(/\\/g, '/');
    // the first row describes the project itself: its NAME is the .pjx, absolute, with HOMEDIR set
    if (/\.pjx$/i.test(sourcePath)) continue;
    // VFP's type letters vary between versions, and a project may list anything at all: XML,
    // schemas, images, readmes. An unrecognised letter is no reason to drop a file, so fall
    // back to the extension and finally to "other", which keeps it listed and openable.
    const typeLetter = text(table, record, 'TYPE');
    const kind = TYPE_KINDS[typeLetter] ?? TYPE_KINDS[typeLetter.toUpperCase()] ?? kindForPath(sourcePath);

    items.push({
      sourcePath,
      kind,
      excluded: flag(table, record, 'EXCLUDE'),
      main: flag(table, record, 'MAINPROG'),
      needsConversion: kind === 'form' || kind === 'menu' || kind === 'class',
    });
  }

  return { name: basename(projectFileName).replace(/\.pjx$/i, ''), items, skipped };
}

/** The path an item will have in the imported project: converted files change extension. */
export function targetPath(item: ImportedItem): string {
  const extension = CONVERTED_EXTENSIONS[item.kind];
  if (!extension) return item.sourcePath;
  return item.sourcePath.replace(/\.[^./]+$/, extension);
}

/** Builds the FoxDev project document for an imported project. */
export function toProjectDocument(imported: ImportedProject, converted: ReadonlySet<string>): ProjectDocument {
  const items: ProjectItem[] = [];
  let main: string | undefined;

  for (const item of imported.items) {
    // a form or menu we could not convert would point at a file that does not exist
    if (item.needsConversion && !converted.has(item.sourcePath)) continue;
    const path = targetPath(item);
    items.push({ kind: item.kind, path, ...(item.excluded ? { excluded: true } : {}) });
    if (item.main) main = path;
  }

  return {
    $schema: 'foxdev-project',
    version: 1,
    name: imported.name,
    ...(main ? { main } : {}),
    items,
  };
}
