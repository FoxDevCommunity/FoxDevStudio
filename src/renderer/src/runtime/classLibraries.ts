/**
 * The class libraries a running program can name classes out of.
 *
 * `SET CLASSLIB TO` loads them and they stay loaded until the program says otherwise, which is
 * what makes `CREATEOBJECT("tbrbackcolor")` find a class nobody has written down in the program
 * itself. `NEWOBJECT()`'s second argument names a file for one call: measured, that file is read
 * and is *not* one of the loaded libraries afterwards, so it is kept apart here too.
 *
 * Reading a `.vcx` is the importer's job already - the same code that resolves a form's parent
 * classes when a project is imported - so all this adds is who is loaded, in what order, and
 * which library a class name belongs to.
 */

import { basename, dirname } from '@shared/paths';
import { findClass, type ClassDefinition, type ClassLibraryDocument } from '@shared/classlib/schema';
import { importClassLibraryDocument } from '@shared/vfp/importClass';
import type { DbfTableData } from '@shared/vfp/dbfTypes';
import { loadClassLibraries } from '../vfp/classLibraries';

export interface LoadedClassLibrary {
  /** The file as the host sees it, which is what `SET("CLASSLIB")` reports. */
  path: string;
  /** What the library answers to: its file stem, or the name `ALIAS` gave it. */
  alias: string;
  doc: ClassLibraryDocument;
}

/** A class found by name, and the library it was found in. */
export interface FoundClass {
  library: LoadedClassLibrary;
  definition: ClassDefinition;
}

export class ClassLibraries {
  /** In the order they were loaded, which is the order a class name is looked for in. */
  private readonly loaded: LoadedClassLibrary[] = [];
  /** Every library read this run, by path: a file is not read twice for one session. */
  private readonly read = new Map<string, ClassLibraryDocument>();

  /**
   * `search` are the folders a library is looked for in when it is not where the program said,
   * which is where the Foundation Classes we ship live: a program written against Visual FoxPro
   * finds those on the product's own path and names them by file alone.
   */
  constructor(
    private readonly readTable: (path: string) => Promise<DbfTableData>,
    private readonly search: readonly string[] = [],
    /** Told when a library a class stands on cannot be found, so an object is not silently short of its parent's code. */
    private readonly onMissing: (library: string, from: string) => void = () => {},
  ) {}

  /**
   * `SET CLASSLIB TO file[, file...] [ALIAS name] [ADDITIVE]`. Answers with what
   * `SET("CLASSLIB")` is to say afterwards, and throws for a file that is not there - which is
   * error 1 in the product, with the list left as it was.
   *
   * Measured: a library already on the list is not added a second time, and one loaded again
   * after being dropped goes on the end of the list.
   */
  async set(files: readonly string[], alias: string, additive: boolean): Promise<string> {
    const opened: LoadedClassLibrary[] = [];
    for (const file of files) opened.push(await this.open(file, alias));
    if (!additive) this.loaded.length = 0;
    for (const library of opened) {
      if (!this.loaded.some((l) => samePath(l.path, library.path))) this.loaded.push(library);
    }
    return this.setting();
  }

  /** What `SET("CLASSLIB")` answers: every library as its file and the name it answers to. */
  setting(): string {
    return this.loaded.map((l) => `"${l.path.toUpperCase()}" ALIAS ${l.alias.toUpperCase()}`).join(', ');
  }

  /**
   * The class of that name, in the file named or - when no file is named - in the loaded
   * libraries, the first one that has it. Measured: with the same class in two loaded
   * libraries, the object comes from the one that was loaded first.
   */
  async find(className: string, file: string): Promise<FoundClass | null> {
    const libraries = file === '' ? this.loaded : [await this.open(file, '')];
    for (const library of libraries) {
      const definition = findClass(library.doc, className);
      if (definition) return { library, definition };
    }
    return null;
  }

  /** Reads one library, or hands back the classes already read out of that file. */
  async open(path: string, alias: string): Promise<LoadedClassLibrary> {
    const name = basename(path, true);
    const already = this.read.get(path.toLowerCase());
    if (already) return { path, alias: alias || name, doc: already };
    const table = await this.readTable(path);
    // a class in the file may stand on one in another file, and an object of it needs the whole
    // lineage; the importer follows those references itself, given somewhere to look
    const { libraries, missing } = await loadClassLibraries(table, dirname(path), this.readTable, [dirname(path), ...this.search]);
    // a class standing on another in the same file names its own library, which is not missing
    const own = basename(path).toLowerCase();
    for (const library of missing) {
      if (basename(library.replace(/\\/g, '/')).toLowerCase() !== own) this.onMissing(library, path);
    }
    const doc = importClassLibraryDocument(table, name, libraries).doc;
    this.read.set(path.toLowerCase(), doc);
    return { path, alias: alias || name, doc };
  }

  /** Lets go of everything, as `SET CLASSLIB TO` with nothing after it does. */
  clear(): void {
    this.loaded.length = 0;
    this.read.clear();
  }
}

function samePath(one: string, two: string): boolean {
  return one.toLowerCase() === two.toLowerCase();
}
