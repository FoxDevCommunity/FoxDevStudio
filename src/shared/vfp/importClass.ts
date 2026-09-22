/**
 * Turns an imported `.vcx` into one class library document.
 *
 * `importClassLibrary` does the work of reading the table: every class in the file comes back as
 * a form-shaped document with its lineage beside it. All that is left is to put them in one
 * library, which is what a `.vcx` is and what an `.fxc` is - a file of classes, not a file per
 * class.
 */

import { CLASSLIB_SCHEMA_ID, CLASSLIB_VERSION } from '../classlib/schema';
import type { ClassDefinition, ClassLibraryDocument, ClassMembers } from '../classlib/schema';
import type { DocumentMeta, FormNode } from '../form/schema';
import { IMPORTER_VERSION, importClassLibrary } from './importForm';
import type { VfpClassFacts, VfpClassLibrary, VfpImportWarning } from './importForm';
import type { DbfTableData } from './dbfTypes';

export interface ImportedClassLibrary {
  doc: ClassLibraryDocument;
  warnings: VfpImportWarning[];
}

/**
 * `name` is the file stem, which is the name the library answers to: `SET CLASSLIB TO` and a
 * subclass's `CLASSLOC` both name a library by its file.
 */
export function importClassLibraryDocument(
  table: DbfTableData,
  name: string,
  libraries: readonly VfpClassLibrary[] = [],
): ImportedClassLibrary {
  const warnings: VfpImportWarning[] = [];
  const classes: ClassDefinition[] = [];

  for (const entry of importClassLibrary(table, name, libraries)) {
    warnings.push(...entry.imported.warnings);
    classes.push(toDefinition(entry.className, entry.imported.doc.form, entry.facts, entry.imported.doc.meta));
  }
  if (classes.length === 0) {
    warnings.push({ object: name, kind: 'other', message: 'No class definitions were found in this file.' });
  }

  return {
    doc: {
      $schema: CLASSLIB_SCHEMA_ID,
      version: CLASSLIB_VERSION,
      name,
      classes,
      meta: { vfp: { importer: IMPORTER_VERSION } },
    },
    warnings,
  };
}

/**
 * One class: the object it defines, and what the class itself says about it.
 *
 * The class is filed under the name its row carries, which is what `NEWOBJECT()` and a subclass
 * ask for. The object's own `Name` property is normally the same word, and where a library has
 * been edited by hand and it is not, the row wins and the paths in `own` are moved onto it.
 */
function toDefinition(className: string, form: FormNode, facts: VfpClassFacts, meta: DocumentMeta | undefined): ClassDefinition {
  // a class that derives straight from a base class owns all of it, and listing that is noise
  const own: Record<string, ClassMembers> = {};
  if (facts.parentClass) {
    for (const [path, members] of Object.entries(facts.own)) {
      const entry: ClassMembers = {};
      if (members.props.length > 0) entry.props = [...new Set(members.props)];
      if (members.methods.length > 0) entry.methods = [...new Set(members.methods)];
      own[rename(path, form.name, className)] = entry;
    }
  }

  return {
    name: className,
    baseClass: facts.baseClass || 'custom',
    ...(facts.parentClass ? { parentClass: facts.parentClass } : {}),
    ...(facts.parentLibrary ? { parentLibrary: facts.parentLibrary } : {}),
    ...(facts.description ? { description: facts.description } : {}),
    props: form.props,
    methods: form.methods,
    children: form.children,
    ...(Object.keys(own).length > 0 ? { own } : {}),
    ...(meta ? { meta } : {}),
  };
}

/** Puts a path built from the object's name back under the name the class is filed as. */
function rename(path: string, from: string, to: string): string {
  if (path.toLowerCase() === from.toLowerCase()) return to;
  if (path.toLowerCase().startsWith(`${from.toLowerCase()}.`)) return to + path.slice(from.length);
  return path;
}
