import { readFileSync } from 'node:fs';
import { beforeAll, describe, expect, it } from 'vitest';
import { importClassLibraryDocument } from '@shared/vfp/importClass';
import { findClass, ownsMethod, ownsProperty } from '@shared/classlib/schema';
import { parseClassLibraryDocument, stringifyClassLibraryDocument } from '@shared/classlib/serialize';
import type { DbfReadResult, DbfTableData } from '@shared/vfp/dbfTypes';
import { loadFoxVm, type FoxVmModule } from '../../src/wasm/foxvm/loader';

/**
 * A genuine Visual FoxPro class library, not one we wrote: coolstuf.vcx from the Hacker's Guide
 * samples, whose classes subclass each other and one that lives in another library.
 */
const FIXTURES = 'crates/foxvm/tests/fixtures';
const read = (name: string) => new Uint8Array(readFileSync(`${FIXTURES}/${name}`));

let vm: FoxVmModule;

function table(dbf: string, memo: string): DbfTableData {
  const result = vm.read_dbf(read(dbf), read(memo)) as DbfReadResult;
  if (!result.ok) throw new Error(result.error);
  return result;
}

beforeAll(async () => {
  vm = await loadFoxVm();
});

describe('importing a real .vcx as one class library', () => {
  it('puts every class of the file in one library document', () => {
    const { doc } = importClassLibraryDocument(table('coolstuf.vcx', 'coolstuf.vct'), 'coolstuf');

    expect(doc.$schema).toBe('foxdev-classlib');
    expect(doc.name).toBe('coolstuf');
    expect(doc.classes.map((c) => c.name)).toEqual(['catmover', 'datespin', 'sortmoverlists', 'arraymover']);
    expect(doc.meta?.vfp?.importer).toBeGreaterThan(0);
  });

  it('keeps what the Class Info dialog holds: the base class, the parent and the description', () => {
    const { doc } = importClassLibraryDocument(table('coolstuf.vcx', 'coolstuf.vct'), 'coolstuf');

    const datespin = findClass(doc, 'datespin')!;
    expect(datespin.baseClass).toBe('container');
    expect(datespin.parentClass).toBeUndefined();
    expect(datespin.description).toBe('Three individual spinners are combined to provide an American style date.');

    // catmover subclasses a class in this same file, so the library is not worth repeating
    const catmover = findClass(doc, 'catmover')!;
    expect(catmover.parentClass).toBe('sortmoverlists');
    expect(catmover.parentLibrary).toBeUndefined();

    // sortmoverlists subclasses one that lives elsewhere, and that is worth keeping
    const sorted = findClass(doc, 'sortmoverlists')!;
    expect(sorted.parentClass).toBe('moverlists');
    expect(sorted.parentLibrary?.toLowerCase()).toContain('samples.vcx');
  });

  it('carries the whole object, so a class can be built without its parent library', () => {
    // coolstuf's classes climb into samples.vcx, so that is where the chain has to be read from
    const { doc } = importClassLibraryDocument(table('coolstuf.vcx', 'coolstuf.vct'), 'coolstuf', [
      { name: 'samples', table: table('samples.vcx', 'samples.vct') },
    ]);
    const catmover = findClass(doc, 'catmover')!;

    expect(catmover.props['Width']).toBe(410);
    expect(catmover.props['Height']).toBe(186);
    // the list boxes come from two classes up the chain, and they are here all the same
    expect(catmover.children.map((c) => c.name)).toContain('lstSource');
    expect(catmover.children.map((c) => c.name)).toContain('lstSelected');
    expect(catmover.methods['initmover']).toContain('LPARAMETERS');
  });

  it('says which values the class writes and which came from the parent', () => {
    const { doc } = importClassLibraryDocument(table('coolstuf.vcx', 'coolstuf.vct'), 'coolstuf', [
      { name: 'samples', table: table('samples.vcx', 'samples.vct') },
    ]);
    const catmover = findClass(doc, 'catmover')!;

    // its own record sets these two, and overrides its own initmover
    expect(ownsProperty(catmover, 'catmover', 'Width')).toBe(true);
    expect(ownsProperty(catmover, 'catmover', 'Height')).toBe(true);
    expect(ownsMethod(catmover, 'catmover', 'initmover')).toBe(true);
    // an override of something a class further up owns is written as a dotted name, and is an
    // override all the same: this is what the designer marks
    expect(ownsProperty(catmover, 'catmover.lstSource', 'Height')).toBe(true);
    expect(ownsProperty(catmover, 'catmover.lstSource', 'Top')).toBe(true);

    // and the rest of what it carries came down the chain, which is the point of saying so
    const inherited = Object.keys(catmover.props).filter((p) => !ownsProperty(catmover, 'catmover', p));
    expect(inherited.length).toBeGreaterThan(0);
    const child = catmover.children.find((c) => c.name === 'lstSource')!;
    expect(Object.keys(child.props).filter((p) => !ownsProperty(catmover, 'catmover.lstSource', p)).length).toBeGreaterThan(0);

    // a class that derives straight from a base class owns everything it has
    const datespin = findClass(doc, 'datespin')!;
    expect(datespin.own).toBeUndefined();
    expect(ownsProperty(datespin, 'datespin', 'Width')).toBe(true);
    expect(ownsMethod(datespin, 'datespin', 'Init')).toBe(true);
  });

  it('writes and reads back the library it imported', () => {
    const { doc } = importClassLibraryDocument(table('coolstuf.vcx', 'coolstuf.vct'), 'coolstuf');
    const text = stringifyClassLibraryDocument(doc);
    const parsed = parseClassLibraryDocument(text);

    expect(parsed.ok).toBe(true);
    if (!parsed.ok) return;
    expect(parsed.doc.classes.map((c) => c.name)).toEqual(doc.classes.map((c) => c.name));
    // the written form is the canonical one: reading it and writing it again changes nothing
    expect(stringifyClassLibraryDocument(parsed.doc)).toBe(text);
  });

  it('imports a second real library the same way', () => {
    const { doc, warnings } = importClassLibraryDocument(table('samples.vcx', 'samples.vct'), 'samples');

    expect(doc.classes.length).toBeGreaterThan(10);
    expect(findClass(doc, 'clock')?.description).toBe('day, date, and time control');
    expect(findClass(doc, 'moverlists')?.baseClass).toBe('container');
    expect(parseClassLibraryDocument(stringifyClassLibraryDocument(doc)).ok).toBe(true);
    // a real library imports without the importer losing its footing on any class in it
    expect(warnings.filter((w) => w.kind === 'other')).toEqual([]);
  });

  it('a table that is not a class library says so rather than throwing', () => {
    const { doc, warnings } = importClassLibraryDocument({ ok: true, version: 0x30, codepage: null, fields: [], records: [] }, 'empty');
    expect(doc.classes).toEqual([]);
    expect(warnings[0]?.message).toContain('No class definitions');
  });
});

describe('re-importing a document whose source is a class library', () => {
  it('says there is no form in it rather than handing back an empty one silently', async () => {
    const { importFormFile } = await import('@shared/vfp/importForm');
    const [first] = importFormFile(table('coolstuf.vcx', 'coolstuf.vct'), 'coolstuf');
    // a .vcx holds classes and no form; the warning is what stops a refresh writing this
    // empty document over the one the user has
    expect(first?.imported.warnings.some((w) => w.kind === 'noForm')).toBe(true);
  });
});
