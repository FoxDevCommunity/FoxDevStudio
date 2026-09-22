import { readFileSync } from 'node:fs';
import { beforeAll, describe, expect, it } from 'vitest';
import { classLibrariesReferenced, importClassLibrary, importFormFile, importFormTable } from '@shared/vfp/importForm';
import type { DbfReadResult, DbfTableData } from '@shared/vfp/dbfTypes';
import { parseFormDocument, stringifyFormDocument } from '@shared/form/serialize';
import { allNodes } from '@shared/form/tree';
import { loadFoxVm, type FoxVmModule } from '../../src/wasm/foxvm/loader';

/**
 * These run against genuine Visual FoxPro files (the Hacker's Guide samples), not fixtures we
 * wrote. They are the only real evidence that the importer handles what VFP actually produces.
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

describe('importing real Visual FoxPro forms', () => {
  it('imports testdate.scx: the form, its properties and its methods', () => {
    const { doc, warnings } = importFormTable(table('testdate.scx', 'testdate.sct'), 'testdate');

    expect(doc.form.name).toBe('Form1');
    expect(doc.form.props['Caption']).toBe('Date Spinner Sample');
    expect(doc.form.props['Height']).toBe(71);
    expect(doc.form.props['Width']).toBe(240);
    expect(doc.form.props['AutoCenter']).toBe(true);

    // the form's own methods came across, with the comment header intact
    expect(Object.keys(doc.form.methods)).toContain('about');
    expect(doc.form.methods['about']).toContain("Hacker's Guide to Visual FoxPro");

    // the one control is a subclass of a class in forms.vcx, so it warns and imports as its base
    const control = doc.form.children[0];
    expect(control?.name).toBe('Datespin1');
    expect(control?.props['Top']).toBe(17);
    expect(control?.props['Left']).toBe(24);
    expect(warnings.some((w) => w.message.toLowerCase().includes('datespin'))).toBe(true);
  });

  it('keeps properties it has no editor for, rather than losing them', () => {
    const { doc } = importFormTable(table('testdate.scx', 'testdate.sct'), 'testdate');
    const reserved = doc.meta?.vfp?.reserved ?? {};

    // DoCreate is a real VFP form property FoxDev does not model
    expect(reserved['Form1.DoCreate']).toBe('.T.');
    // and the dotted member properties of the subclassed control
    expect(Object.keys(reserved).some((k) => k.includes('Datespin1.') && k.endsWith('.Name'))).toBe(true);
  });

  it('carries the properties a form added for itself, not only the ones the registry knows', () => {
    // a form's own properties are as real as Caption to the code that reads them; dropping them
    // makes every method that touches one fail with "Property ... is not found"
    const { doc } = importFormTable(table('testdate.scx', 'testdate.sct'), 'testdate');
    const known = new Set(Object.keys(doc.form.props).map((k) => k.toLowerCase()));
    const reserved = doc.meta?.vfp?.reserved ?? {};
    const own = Object.keys(reserved)
      .filter((k) => k.startsWith('Form1.') && !k.slice('Form1.'.length).includes('.'))
      .map((k) => k.slice('Form1.'.length).toLowerCase())
      .filter((k) => k !== 'docreate');
    for (const name of own) {
      expect(known.has(name), `${name} should be a property of the form`).toBe(true);
    }
  });

  it('imports a form with many controls and produces a valid document', () => {
    const { doc } = importFormTable(table('testdraw.scx', 'testdraw.sct'), 'testdraw');

    const nodes = allNodes(doc.form);
    expect(nodes.length).toBeGreaterThan(3);
    // every control has a name and a type the designer knows
    expect(nodes.every((n) => n.name.length > 0)).toBe(true);
    expect(new Set(nodes.map((n) => n.name.toLowerCase())).size).toBe(nodes.length);

    const round = parseFormDocument(stringifyFormDocument(doc));
    expect(round.ok).toBe(true);
  });

  it('imports thrmdemo.scx and round-trips it', () => {
    const { doc } = importFormTable(table('thrmdemo.scx', 'thrmdemo.sct'), 'thrmdemo');
    expect(doc.form.name.length).toBeGreaterThan(0);
    const round = parseFormDocument(stringifyFormDocument(doc));
    if (!round.ok) throw new Error(round.error);
    expect(round.doc.form.name).toBe(doc.form.name);
  });

  it('splits a real class library into one document per class', () => {
    const classes = importClassLibrary(table('forms.vcx', 'forms.vct'), 'forms');

    expect(classes.length).toBeGreaterThan(0);
    for (const entry of classes) {
      expect(entry.className.length).toBeGreaterThan(0);
      const round = parseFormDocument(stringifyFormDocument(entry.imported.doc));
      expect(round.ok, `${entry.className}: ${round.ok ? '' : round.error}`).toBe(true);
    }
    // forms.vcx defines one form class with an edit box and two buttons
    const showResults = classes.find((c) => c.className.toLowerCase() === 'frmshowresults');
    expect(showResults).toBeDefined();
    const children = showResults!.imported.doc.form.children;
    expect(children.map((c) => c.name).sort()).toEqual(['cmdClear', 'cmdClose', 'edtMessages']);
    expect(children.find((c) => c.name === 'edtMessages')?.type).toBe('EditBox');
    expect(children.find((c) => c.name === 'cmdClose')?.type).toBe('CommandButton');
  });

  it('builds a subclassed control from the class library it came from', () => {
    // Datespin1 is an instance of the `datespin` class in coolstuf.vcx
    const libraries = [{ name: 'coolstuf.vcx', table: table('coolstuf.vcx', 'coolstuf.vct') }];
    const { doc, warnings } = importFormTable(table('testdate.scx', 'testdate.sct'), 'testdate', libraries);

    const control = doc.form.children[0]!;
    expect(control.name).toBe('Datespin1');
    // the instance still wins on what it sets itself
    expect(control.props['Top']).toBe(17);
    expect(control.props['Left']).toBe(24);
    // and the members the class defines are there now, which is the whole point
    expect(control.children?.length).toBeGreaterThan(0);
    expect(control.children?.map((c) => c.type)).toContain('Spinner');
    expect(warnings.some((w) => w.message.includes('was not found'))).toBe(false);
  });

  it('still says so when the class library is not available', () => {
    const { doc, warnings } = importFormTable(table('testdate.scx', 'testdate.sct'), 'testdate');
    expect(doc.form.children[0]?.children ?? []).toHaveLength(0);
    expect(warnings.some((w) => w.message.includes('Datespin') && w.message.includes('was not found'))).toBe(true);
  });

  it('builds the pages a pageframe names in its own memo, and parents rows to them', () => {
    // pfsam2.scx writes no row for either page: pgfPeople carries PageCount = 2 and names them
    // with Page1.Name = "pagCustomers", which is what every later row parents itself to.
    const { doc, warnings } = importFormTable(table('pfsam2.scx', 'pfsam2.sct'), 'pfsam2');
    const frame = doc.form.children.find((c) => c.name === 'pgfPeople')!;
    expect(frame.type).toBe('PageFrame');
    expect(frame.children?.map((c) => c.name)).toEqual(['pagCustomers', 'pagEmployees']);

    const customers = frame.children![0]!;
    expect(customers.props['Caption']).toBe('Page1');
    expect(customers.children?.map((c) => c.name)).toContain('txtCompany');
    expect(frame.children![1]!.children?.map((c) => c.name)).toContain('lblLast_name');

    // nothing orphaned onto the form, and no duplicate names invented for the pages' rows
    expect(warnings.filter((w) => w.message.includes('Parent'))).toEqual([]);
    expect(warnings.filter((w) => w.message.includes('renamed'))).toEqual([]);
  });

  it('imports each form of a formset, without mixing their controls', () => {
    // objects.scx is a formset of two: frmleft and frmright, each with its own buttons
    const forms = importFormFile(table('objects.scx', 'objects.sct'), 'objects');
    expect(forms.map((f) => f.formName)).toEqual(['frmleft', 'frmright']);

    const [left, right] = forms;
    const leftNames = left!.imported.doc.form.children.map((c) => c.name);
    const rightNames = right!.imported.doc.form.children.map((c) => c.name);

    // frmright's buttons belong to frmright, and used to land on frmleft instead
    expect(rightNames).toContain('cmdQuit');
    expect(leftNames).not.toContain('cmdQuit');
    expect(leftNames.some((n) => rightNames.includes(n))).toBe(false);

    // and nothing is orphaned onto a form that is not its own
    const orphans = forms.flatMap((f) => f.imported.warnings).filter((w) => w.kind === 'parentNotFound');
    expect(orphans.map((w) => w.object)).toEqual([]);

    // the formset is a thing of its own, not a warning: every one of its forms carries it whole
    expect(forms.flatMap((f) => f.imported.warnings).filter((w) => w.kind === 'formset')).toEqual([]);
    for (const { imported } of forms) {
      const formset = imported.doc.meta?.vfp?.formset;
      expect(formset?.name).toBe('Formset1');
      expect(formset?.forms).toEqual(['frmleft', 'frmright']);
      // its own values and code, which belong to no form and would otherwise be lost
      expect(formset?.props['AutoRelease']).toBe(true);
      expect(formset?.props['HelpContextID']).toBe(1231558);
    }
  });

  it('follows an inheritance chain across two class libraries', () => {
    // catmover is a sortmoverlists (coolstuf.vcx), which is a moverlists (samples.vcx)
    const samples = [{ name: 'samples.vcx', table: table('samples.vcx', 'samples.vct') }];
    const withoutIt = importClassLibrary(table('coolstuf.vcx', 'coolstuf.vct'), 'coolstuf');
    const withIt = importClassLibrary(table('coolstuf.vcx', 'coolstuf.vct'), 'coolstuf', samples);
    const controls = (classes: typeof withIt, name: string) =>
      classes.find((c) => c.className.toLowerCase() === name)!.imported.doc.form.children.map((c) => c.name);

    // without the far library the chain stops one link short and says so
    expect(controls(withoutIt, 'catmover')).toEqual(['cboCategories']);
    expect(withoutIt.flatMap((c) => c.imported.warnings).some((w) => w.message.includes('samples.vcx'))).toBe(true);

    // with it, the inherited controls arrive first and the class's own come after
    expect(controls(withIt, 'catmover')).toEqual(['lstSource', 'lstSelected', 'cmdAdd', 'cmdAddAll', 'cmdRemove', 'cmdRemoveAll', 'cboCategories']);
    expect(controls(withIt, 'sortmoverlists')).toEqual(['lstSource', 'lstSelected', 'cmdAdd', 'cmdAddAll', 'cmdRemove', 'cmdRemoveAll']);
    expect(withIt.flatMap((c) => c.imported.warnings)).toEqual([]);
  });

  it('resolves a class that inherits from another class in the same library', () => {
    const classes = importClassLibrary(table('coolstuf.vcx', 'coolstuf.vct'), 'coolstuf');
    const arraymover = classes.find((c) => c.className.toLowerCase() === 'arraymover');
    expect(arraymover).toBeDefined();
    // arraymover is a sortmoverlists, defined a few rows further down the same file
    expect(arraymover!.imported.doc.form.children.map((c) => c.name)).toContain('cmdOk');
    expect(arraymover!.imported.warnings.map((w) => w.message).join(' ')).not.toContain('"sortmoverlists"');
  });

  it('reports the class libraries a form needs before they are loaded', () => {
    expect(classLibrariesReferenced(table('testdate.scx', 'testdate.sct'))).toEqual(['..\\utils\\coolstuf.vcx']);
    expect(classLibrariesReferenced(table('forms.vcx', 'forms.vct'))).toEqual([]);
  });
});
