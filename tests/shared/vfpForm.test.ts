import { describe, expect, it } from 'vitest';
import { parseFormDocument, stringifyFormDocument } from '@shared/form/serialize';
import { rgb } from '@shared/form/color';
import type { DbfFieldInfo, DbfRecordData, DbfTableData } from '@shared/vfp/dbfTypes';
import {
  baseClassToControlType,
  importClassLibrary,
  importFormTable,
  parseVfpMethods,
  parseVfpProperties,
  vfpLiteral,
  type VfpClassLibrary,
} from '@shared/vfp/importForm';

// ---------------------------------------------------------------- fixtures

/** The field list of a real VFP 9 `.scx`/`.vcx`, in file order. */
const FIELD_NAMES = [
  'PLATFORM',
  'UNIQUEID',
  'TIMESTAMP',
  'CLASS',
  'CLASSLOC',
  'BASECLASS',
  'OBJNAME',
  'PARENT',
  'PROPERTIES',
  'PROTECTED',
  'METHODS',
  'OBJCODE',
  'OLE',
  'OLE2',
  'RESERVED1',
  'RESERVED2',
  'RESERVED3',
  'RESERVED4',
  'RESERVED5',
  'RESERVED6',
  'RESERVED7',
  'RESERVED8',
  'USER',
] as const;

const FIELDS: DbfFieldInfo[] = FIELD_NAMES.map((name) => {
  if (name === 'PLATFORM') return { name, kind: 'C', length: 8, decimals: 0 };
  if (name === 'UNIQUEID') return { name, kind: 'C', length: 10, decimals: 0 };
  if (name === 'TIMESTAMP') return { name, kind: 'N', length: 10, decimals: 0 };
  return { name, kind: 'M', length: 4, decimals: 0 };
});

type RowSpec = Partial<Record<(typeof FIELD_NAMES)[number], string | number>> & { deleted?: boolean };

/** VFP separates PROPERTIES entries with a blank line, i.e. CRLF CRLF. */
function props(...lines: string[]): string {
  return lines.join('\r\n\r\n') + '\r\n';
}

function row(spec: RowSpec): DbfRecordData {
  const { deleted = false, ...fields } = spec;
  return {
    deleted,
    values: FIELD_NAMES.map((name) => {
      const v = fields[name];
      if (v !== undefined) return v;
      return name === 'TIMESTAMP' ? 0 : '';
    }),
  };
}

function table(...rows: DbfRecordData[]): DbfTableData {
  return { ok: true, version: 0x30, codepage: 1252, fields: FIELDS, records: rows };
}

/** Record 0 of every .scx: a header row with no OBJNAME and every memo pointer at 0. */
const HEADER_ROW = row({ PLATFORM: 'WINDOWS', UNIQUEID: '_3O91BQ4YF', TIMESTAMP: 0 });

/** The form row of `testdate.SCX`, verbatim. */
const TESTDATE_FORM = row({
  PLATFORM: 'WINDOWS',
  UNIQUEID: '_3O91BQ4YG',
  CLASS: 'form',
  BASECLASS: 'form',
  OBJNAME: 'Form1',
  PARENT: '',
  PROPERTIES: props(
    'Height = 71',
    'Width = 240',
    'DoCreate = .T.',
    'AutoCenter = .T.',
    'Caption = "Date Spinner Sample"',
    'Name = "Form1"',
  ),
  METHODS: [
    'PROCEDURE about',
    '*=====================================================',
    '* Program:\t\t\t\tTestDate.SCX',
    '*=====================================================',
    'WAIT WINDOW "TestDate"',
    'ENDPROC',
    'PROCEDURE Load',
    '',
    'ENDPROC',
    '',
  ].join('\r\n'),
});

/** The `Datespin1` row: an instance of the custom class `datespin`, whose base is a container. */
const TESTDATE_DATESPIN = row({
  PLATFORM: 'WINDOWS',
  UNIQUEID: '_3O91BQ4YH',
  CLASS: 'datespin',
  CLASSLOC: 'datespin.vcx',
  BASECLASS: 'container',
  OBJNAME: 'Datespin1',
  PARENT: 'Form1',
  PROPERTIES: props('Top = 17', 'Left = 24', 'Name = "Datespin1"', 'Label1.Name = "Label1"', 'spnMonths.Name = "spnMonths"'),
});

/** A .scx also ends with a stray row that carries PROPERTIES but no OBJNAME. */
const TRAILING_ROW = row({ PLATFORM: 'WINDOWS', PROPERTIES: props('Comment = ""') });

function testdateTable(): DbfTableData {
  return table(HEADER_ROW, TESTDATE_FORM, TESTDATE_DATESPIN, TRAILING_ROW);
}

function commandButton(objName: string, extra: string[] = [], parent = 'Form1'): DbfRecordData {
  return row({
    PLATFORM: 'WINDOWS',
    CLASS: 'commandbutton',
    BASECLASS: 'commandbutton',
    OBJNAME: objName,
    PARENT: parent,
    PROPERTIES: props('Top = 40', 'Left = 8', 'Height = 27', 'Enabled = .T.', 'Caption = "OK"', `Name = "${objName}"`, ...extra),
    METHODS: ['PROCEDURE click', 'THISFORM.Release()', 'ENDPROC', ''].join('\r\n'),
  });
}

// ---------------------------------------------------------------- tests

describe('vfpLiteral', () => {
  it('reads VFP value literals', () => {
    expect(vfpLiteral('71')).toBe(71);
    expect(vfpLiteral('-1')).toBe(-1);
    expect(vfpLiteral('1.5')).toBe(1.5);
    expect(vfpLiteral('.T.')).toBe(true);
    expect(vfpLiteral('.y.')).toBe(true);
    expect(vfpLiteral('.F.')).toBe(false);
    expect(vfpLiteral('.N.')).toBe(false);
    expect(vfpLiteral('.NULL.')).toBeNull();
    expect(vfpLiteral('"Date Spinner Sample"')).toBe('Date Spinner Sample');
    expect(vfpLiteral("'single'")).toBe('single');
    expect(vfpLiteral('[bracketed]')).toBe('bracketed');
    expect(vfpLiteral('"a = b, c"')).toBe('a = b, c');
    expect(vfpLiteral('RGB(255,0,0)')).toBe(rgb(255, 0, 0));
    expect(vfpLiteral('rgb(0, 128, 255)')).toBe(rgb(0, 128, 255));
    expect(vfpLiteral('')).toBe('');
    expect(vfpLiteral('THISFORM.Text1.Value')).toBe('THISFORM.Text1.Value');
    expect(vfpLiteral('{^2020-01-01}')).toBe('{^2020-01-01}');
  });
});

describe('parseVfpProperties', () => {
  it('reads entries in source order and ignores blank lines', () => {
    const entries = parseVfpProperties(props('Height = 71', 'Caption = "a = b, c"', 'AutoCenter = .T.', 'Label1.Name = "Label1"'));
    expect(entries.map((e) => e.name)).toEqual(['Height', 'Caption', 'AutoCenter', 'Label1.Name']);
    expect(entries.map((e) => e.value)).toEqual([71, 'a = b, c', true, 'Label1']);
    expect(entries[1]!.raw).toBe('"a = b, c"');
  });
  it('returns nothing for an empty memo', () => {
    expect(parseVfpProperties('')).toEqual([]);
    expect(parseVfpProperties('\r\n\r\n')).toEqual([]);
  });
  it('continues a value over lines that have no assignment', () => {
    const entries = parseVfpProperties('Comment = first\r\nsecond line\r\nTag = "t"');
    expect(entries).toHaveLength(2);
    expect(entries[0]!.value).toBe('first\nsecond line');
    expect(entries[1]!.name).toBe('Tag');
  });
  it('continues an unterminated string even when the next line looks like an assignment', () => {
    const entries = parseVfpProperties('Comment = "line one\r\nx = 1"\r\nTag = "t"');
    expect(entries).toHaveLength(2);
    expect(entries[0]!.value).toBe('line one\nx = 1');
    expect(entries[1]!.name).toBe('Tag');
  });
});

describe('parseVfpMethods', () => {
  it('splits PROCEDURE blocks, keeps comments and drops empty bodies', () => {
    const methods = parseVfpMethods(String(TESTDATE_FORM.values[10]));
    expect(Object.keys(methods)).toEqual(['about']);
    expect(methods['about']).toBe(
      [
        '*=====================================================',
        '* Program:\t\t\t\tTestDate.SCX',
        '*=====================================================',
        'WAIT WINDOW "TestDate"',
      ].join('\n'),
    );
  });
  it("keeps an overridden method's earlier versions for DODEFAULT to reach", () => {
    // a class chain's memos arrive joined, the class the chain starts from first: the last
    // definition is what runs, and the ones before it are its ancestors', nearest first
    const memo = [
      'PROCEDURE Init',
      'grand = 1',
      'ENDPROC',
      'PROCEDURE Click',
      'only = 1',
      'ENDPROC',
      'PROCEDURE init',
      'parent = 1',
      'ENDPROC',
      'PROCEDURE INIT',
      'own = 1',
      'RETURN DODEFAULT()',
      'ENDPROC',
    ].join('\n');
    expect(parseVfpMethods(memo)).toEqual({
      INIT: 'own = 1\nRETURN DODEFAULT()',
      'INIT#1': 'parent = 1',
      'INIT#2': 'grand = 1',
      Click: 'only = 1',
    });
  });
  it('ends a body at the next PROCEDURE when ENDPROC is missing', () => {
    const methods = parseVfpMethods('PROCEDURE Init\r\nx = 1\r\nPROCEDURE Click\r\ny = 2\r\nENDPROC');
    expect(methods).toEqual({ Init: 'x = 1', Click: 'y = 2' });
  });
  it('returns nothing for an empty memo', () => {
    expect(parseVfpMethods('')).toEqual({});
  });
});

describe('baseClassToControlType', () => {
  it('maps VFP base classes case-insensitively', () => {
    expect(baseClassToControlType('form')).toBe('Form');
    expect(baseClassToControlType('CommandButton')).toBe('CommandButton');
    expect(baseClassToControlType(' pageframe ')).toBe('PageFrame');
    expect(baseClassToControlType('optionbutton')).toBe('OptionButton');
    // the objects a real form carries that are not ordinary controls are kept, not dropped
    expect(baseClassToControlType('custom')).toBe('Custom');
    expect(baseClassToControlType('olecontrol')).toBe('OleControl');
    expect(baseClassToControlType('separator')).toBe('Separator');
    expect(baseClassToControlType('toolbar')).toBe('Toolbar');
    // the data side and formsets still have no equivalent
    for (const unsupported of ['dataenvironment', 'cursor', 'relation', 'formset', '']) {
      expect(baseClassToControlType(unsupported)).toBeNull();
    }
  });
});

describe('importFormTable', () => {
  it('turns the form row into the root, applying known properties and the Name', () => {
    const { doc } = importFormTable(testdateTable(), 'testdate');
    expect(doc.form.name).toBe('Form1');
    expect(doc.form.props).toEqual({ Caption: 'Date Spinner Sample', Height: 71, Width: 240, AutoCenter: true });
    expect(doc.form.methods).toEqual({ about: expect.stringContaining('TestDate.SCX') as string });
    expect(doc.$schema).toBe('foxdev-form');
    expect(doc.version).toBe(1);
  });

  it('reads the members an object adds for itself, arrays and all', () => {
    // RESERVED3 is the only record of a member that was never given a value: a plain name is a
    // property, `*` marks a method, and `^` an array with its shape in brackets. The SDI Form
    // sample declares `^owindows[1,0]` and its Add Window button reads ALEN() of it.
    const withMembers = row({
      PLATFORM: 'WINDOWS',
      CLASS: 'form',
      BASECLASS: 'form',
      OBJNAME: 'Form1',
      PROPERTIES: props('Name = "Form1"'),
      RESERVED3: 'otoolbar\r\n^owindows[1,0] \r\n^aicon[5,2]\r\n*addwindow\r\n',
    });
    const { doc } = importFormTable(table(HEADER_ROW, withMembers, TRAILING_ROW), 'sdiform');
    expect(doc.meta?.vfp?.arrays).toEqual({ 'Form1.owindows': [1, 0], 'Form1.aicon': [5, 2] });
    // a property with no value is .F.; a method with no source is empty; an array is neither
    expect(doc.form.props['otoolbar']).toBe(false);
    expect(doc.form.methods['addwindow']).toBe('');
    expect(doc.form.props['owindows']).toBeUndefined();
  });

  it('skips the header row, the trailing row and rows for other platforms', () => {
    const mac = row({ PLATFORM: 'MAC', CLASS: 'label', BASECLASS: 'label', OBJNAME: 'Label9', PARENT: 'Form1', PROPERTIES: props('Name = "Label9"') });
    const deleted = row({ PLATFORM: 'WINDOWS', CLASS: 'label', BASECLASS: 'label', OBJNAME: 'Label8', PARENT: 'Form1', deleted: true });
    const { doc } = importFormTable(table(HEADER_ROW, TESTDATE_FORM, mac, deleted, TRAILING_ROW), 'testdate');
    expect(doc.form.children).toEqual([]);
  });

  it('names the document after the file stem when there is no usable form row', () => {
    const anonymous = row({ PLATFORM: 'WINDOWS', CLASS: 'form', BASECLASS: 'form', OBJNAME: '', PARENT: '', PROPERTIES: props('Width = 100') });
    // OBJNAME is empty so the row is skipped entirely and there is no form definition.
    const noForm = importFormTable(table(HEADER_ROW, anonymous), 'testdate');
    expect(noForm.doc.form.name).toBe('testdate');
    expect(noForm.warnings).toHaveLength(1);

    const unnamed = row({ PLATFORM: 'WINDOWS', CLASS: 'form', BASECLASS: 'form', OBJNAME: 'Form1', PARENT: '', PROPERTIES: props('Width = 100') });
    const { doc } = importFormTable(table(HEADER_ROW, unnamed), 'testdate');
    expect(doc.form.name).toBe('Form1');
    expect(doc.form.props).toEqual({ Width: 100 });
  });

  it('imports a command button child with sparse props and normalised method names', () => {
    const { doc, warnings } = importFormTable(table(HEADER_ROW, TESTDATE_FORM, commandButton('Command1')), 'testdate');
    expect(warnings).toEqual([]);
    expect(doc.form.children).toHaveLength(1);
    const button = doc.form.children[0]!;
    expect(button.type).toBe('CommandButton');
    expect(button.name).toBe('Command1');
    // Enabled (.T.) equals the class default so it is not stored; Height does not - a button
    // made by the product itself is 17 high, and the designer that wrote this one made it 27.
    expect(button.props).toEqual({ Top: 40, Left: 8, Height: 27, Caption: 'OK' });
    expect(button.methods).toEqual({ Click: 'THISFORM.Release()' });
    expect(button.id).toMatch(/^\S{10}$/);
    expect(button.children).toBeUndefined();
  });

  it('imports a subclassed control as its base class with exactly one warning', () => {
    const { doc, warnings } = importFormTable(testdateTable(), 'testdate');
    expect(doc.form.children).toHaveLength(1);
    const spinner = doc.form.children[0]!;
    expect(spinner.type).toBe('Container');
    expect(spinner.name).toBe('Datespin1');
    expect(spinner.props).toEqual({ Top: 17, Left: 24 });
    expect(spinner.children).toEqual([]);
    expect(warnings).toHaveLength(1);
    expect(warnings[0]!.object).toBe('Form1.Datespin1');
    expect(warnings[0]!.message).toContain('"datespin"');
    expect(warnings[0]!.message).toContain('datespin.vcx');
  });

  it('keeps an OLE control as a placeholder rather than dropping it', () => {
    const ole = row({
      PLATFORM: 'WINDOWS',
      CLASS: 'olecontrol',
      BASECLASS: 'olecontrol',
      OBJNAME: 'Olecontrol1',
      PARENT: 'Form1',
      PROPERTIES: props('Name = "Olecontrol1"', 'Left = 12', 'OleClass = "MSComctlLib.TreeCtrl"'),
      METHODS: 'PROCEDURE Init\r\n  * set up the tree\r\nENDPROC\r\n',
    });
    const { doc, warnings } = importFormTable(table(HEADER_ROW, TESTDATE_FORM, ole, commandButton('Command1')), 'testdate');

    expect(doc.form.children.map((c) => c.name)).toEqual(['Olecontrol1', 'Command1']);
    const kept = doc.form.children[0]!;
    expect(kept.type).toBe('OleControl');
    expect(kept.props['Left']).toBe(12);
    // the ActiveX class and the code behind it survive, which is the point of keeping it
    expect(kept.props['OleClass']).toBe('MSComctlLib.TreeCtrl');
    expect(kept.methods['Init']).toContain('set up the tree');
    expect(warnings).toEqual([]);
  });

  it('drops a base class that has no equivalent at all, with a warning', () => {
    const formset = row({
      PLATFORM: 'WINDOWS',
      CLASS: 'projecthook',
      BASECLASS: 'projecthook',
      OBJNAME: 'Projecthook1',
      PARENT: 'Form1',
      PROPERTIES: props('Name = "Projecthook1"'),
    });
    const { doc, warnings } = importFormTable(table(HEADER_ROW, TESTDATE_FORM, formset, commandButton('Command1')), 'testdate');
    expect(doc.form.children.map((c) => c.name)).toEqual(['Command1']);
    expect(warnings).toHaveLength(1);
    expect(warnings[0]!.kind).toBe('unsupportedBaseClass');
    expect(warnings[0]!.message).toContain('projecthook');
  });

  it('takes the tables of the data environment and the links between them', () => {
    // the cursors are what a form's code assumes is open, and the relations are what make a
    // grid of order lines follow the order the user is on
    const de = row({ PLATFORM: 'WINDOWS', CLASS: 'dataenvironment', BASECLASS: 'dataenvironment', OBJNAME: 'Dataenvironment', PARENT: '' });
    const cursor1 = row({
      PLATFORM: 'WINDOWS',
      CLASS: 'cursor',
      BASECLASS: 'cursor',
      OBJNAME: 'Cursor1',
      PARENT: 'Dataenvironment',
      PROPERTIES: ['Alias = "customer"', 'CursorSource = data\\customer.dbf', 'Name = "Cursor1"', ''].join('\n'),
    });
    const cursor2 = row({
      PLATFORM: 'WINDOWS',
      CLASS: 'cursor',
      BASECLASS: 'cursor',
      OBJNAME: 'Cursor2',
      PARENT: 'Dataenvironment',
      PROPERTIES: ['CursorSource = orders.dbf', 'Order = "custno"', 'Name = "Cursor2"', ''].join('\n'),
    });
    const relation = row({
      PLATFORM: 'WINDOWS',
      CLASS: 'relation',
      BASECLASS: 'relation',
      OBJNAME: 'Relation1',
      PARENT: 'Dataenvironment.Cursor1',
      PROPERTIES: props('ParentAlias = "customer"', 'RelationalExpr = "cust_id"', 'ChildAlias = "orders"', 'ChildOrder = "cust_id"', 'OneToMany = .T.'),
    });
    // one with no expression links nothing, and VFP's own designer cannot make one
    const halfMade = row({
      PLATFORM: 'WINDOWS',
      CLASS: 'relation',
      BASECLASS: 'relation',
      OBJNAME: 'Relation2',
      PARENT: 'Dataenvironment.Cursor1',
      PROPERTIES: props('ParentAlias = "customer"', 'ChildAlias = "orders"'),
    });
    const { doc, warnings } = importFormTable(
      table(HEADER_ROW, de, cursor1, cursor2, relation, halfMade, TESTDATE_FORM, commandButton('Command1')),
      'testdate',
    );

    expect(doc.form.children.map((c) => c.name)).toEqual(['Command1']);
    expect(doc.data).toEqual([
      { alias: 'customer', source: 'data\\customer.dbf' },
      // no Alias of its own: the file stem is what VFP calls it
      { alias: 'orders', source: 'orders.dbf', order: 'custno' },
    ]);
    expect(doc.relations).toEqual([
      { parent: 'customer', child: 'orders', expression: 'cust_id', childOrder: 'cust_id', oneToMany: true },
    ]);
    expect(warnings).toEqual([]);
  });

  it('keeps dotted and unknown properties in meta.vfp.reserved', () => {
    const { doc } = importFormTable(testdateTable(), 'testdate');
    expect(doc.meta?.vfp?.reserved).toEqual({
      'Form1.DoCreate': '.T.',
      'Form1.Datespin1.Label1.Name': '"Label1"',
      'Form1.Datespin1.spnMonths.Name': '"spnMonths"',
    });
    // The form itself is a plain form, so no class information is recorded for it.
    expect(doc.meta?.vfp?.baseClass).toBeUndefined();
    expect(doc.meta?.vfp?.classLib).toBeUndefined();
  });

  it('builds a nested PageFrame > Page > Label tree from dotted PARENT paths', () => {
    const pgf = row({
      PLATFORM: 'WINDOWS',
      CLASS: 'pageframe',
      BASECLASS: 'pageframe',
      OBJNAME: 'pgf1',
      PARENT: 'Form1',
      PROPERTIES: props('Top = 4', 'Left = 4', 'PageCount = 2', 'Name = "pgf1"'),
    });
    const page1 = row({
      PLATFORM: 'WINDOWS',
      CLASS: 'page',
      BASECLASS: 'page',
      OBJNAME: 'Page1',
      PARENT: 'Form1.pgf1',
      PROPERTIES: props('Caption = "First"', 'Name = "Page1"'),
    });
    const page2 = row({
      PLATFORM: 'WINDOWS',
      CLASS: 'page',
      BASECLASS: 'page',
      OBJNAME: 'Page2',
      PARENT: 'Form1.pgf1',
      PROPERTIES: props('Caption = "Second"', 'Name = "Page2"'),
    });
    const deep = row({
      PLATFORM: 'WINDOWS',
      CLASS: 'label',
      BASECLASS: 'label',
      OBJNAME: 'Label1',
      PARENT: 'Form1.pgf1.Page1',
      PROPERTIES: props('Top = 8', 'Left = 8', 'Caption = "Inside"', 'Name = "Label1"'),
    });
    // VFP also writes a bare parent name; it must resolve on the last segment.
    const shallow = row({
      PLATFORM: 'WINDOWS',
      CLASS: 'label',
      BASECLASS: 'label',
      OBJNAME: 'Label2',
      PARENT: 'Page2',
      PROPERTIES: props('Caption = "Also inside"', 'Name = "Label2"'),
    });
    const { doc, warnings } = importFormTable(table(HEADER_ROW, TESTDATE_FORM, pgf, page1, page2, deep, shallow), 'testdate');
    expect(warnings).toEqual([]);
    const frame = doc.form.children[0]!;
    expect(frame.type).toBe('PageFrame');
    expect(frame.children?.map((c) => c.name)).toEqual(['Page1', 'Page2']);
    const label = frame.children![0]!.children![0]!;
    expect(label.type).toBe('Label');
    expect(label.props).toEqual({ Top: 8, Left: 8, Caption: 'Inside' });
    expect(frame.children![1]!.children![0]!.name).toBe('Label2');
    expect(doc.meta?.vfp?.reserved).toMatchObject({ 'Form1.DoCreate': '.T.' });
  });

  it('places a row with an unresolvable parent on the form and warns', () => {
    const orphan = row({
      PLATFORM: 'WINDOWS',
      CLASS: 'label',
      BASECLASS: 'label',
      OBJNAME: 'Label1',
      PARENT: 'Nowhere.Missing',
      PROPERTIES: props('Name = "Label1"'),
    });
    const { doc, warnings } = importFormTable(table(HEADER_ROW, TESTDATE_FORM, orphan), 'testdate');
    expect(doc.form.children.map((c) => c.name)).toEqual(['Label1']);
    expect(warnings).toHaveLength(1);
    expect(warnings[0]!.message).toContain('Nowhere.Missing');
  });

  it('resolves children written before their parent', () => {
    const label = row({
      PLATFORM: 'WINDOWS',
      CLASS: 'label',
      BASECLASS: 'label',
      OBJNAME: 'Label1',
      PARENT: 'Form1.Container1',
      PROPERTIES: props('Name = "Label1"'),
    });
    const container = row({
      PLATFORM: 'WINDOWS',
      CLASS: 'container',
      BASECLASS: 'container',
      OBJNAME: 'Container1',
      PARENT: 'Form1',
      PROPERTIES: props('Name = "Container1"'),
    });
    const { doc, warnings } = importFormTable(table(HEADER_ROW, TESTDATE_FORM, label, container), 'testdate');
    expect(warnings).toEqual([]);
    expect(doc.form.children[0]!.children![0]!.name).toBe('Label1');
  });

  it('makes duplicate names unique and warns', () => {
    const { doc, warnings } = importFormTable(
      table(HEADER_ROW, TESTDATE_FORM, commandButton('Command1'), commandButton('Command1')),
      'testdate',
    );
    expect(doc.form.children.map((c) => c.name)).toEqual(['Command1', 'Command2']);
    expect(warnings).toHaveLength(1);
    expect(warnings[0]!.message).toContain('renamed to "Command2"');
    expect(doc.form.children[0]!.id).not.toBe(doc.form.children[1]!.id);
  });

  it('lets a row of the file redefine a member it inherited, rather than adding a second', () => {
    // `systray_sample.scx` is stamped from a class that has a Label1, and adds a Label1 of its
    // own. Two members of one container cannot share a name, so the row is that member being
    // redefined - which is why only a *second row of the file* is a duplicate worth renaming.
    const library: VfpClassLibrary = {
      name: 'lib.vcx',
      table: table(
        HEADER_ROW,
        row({ PLATFORM: 'WINDOWS', CLASS: 'form', BASECLASS: 'form', OBJNAME: 'basefrm', PROPERTIES: props('Name = "basefrm"'), RESERVED1: 'Class', RESERVED2: '2' }),
        row({
          PLATFORM: 'WINDOWS',
          CLASS: 'label',
          BASECLASS: 'label',
          OBJNAME: 'Label1',
          PARENT: 'basefrm',
          PROPERTIES: props('Top = 24', 'Caption = "from the class"', 'Name = "Label1"'),
        }),
      ),
    };
    const form = row({ PLATFORM: 'WINDOWS', CLASS: 'basefrm', CLASSLOC: 'lib.vcx', BASECLASS: 'form', OBJNAME: 'Form1', PROPERTIES: props('Name = "Form1"') });
    const own = row({
      PLATFORM: 'WINDOWS',
      CLASS: 'label',
      BASECLASS: 'label',
      OBJNAME: 'Label1',
      PARENT: 'Form1',
      PROPERTIES: props('Top = 354', 'Caption = "from the file"', 'Name = "Label1"'),
    });
    const { doc, warnings } = importFormTable(table(HEADER_ROW, form, own, TRAILING_ROW), 'systray', [library]);

    expect(warnings).toEqual([]);
    expect(doc.form.children.map((c) => c.name)).toEqual(['Label1']);
    expect(doc.form.children[0]!.props['Caption']).toBe('from the file');
  });

  it('reports a table that is not a form at all', () => {
    const bogus: DbfTableData = {
      ok: true,
      version: 0x30,
      codepage: 1252,
      fields: [{ name: 'NAME', kind: 'C', length: 10, decimals: 0 }],
      records: [{ deleted: false, values: ['x'] }],
    };
    const { doc, warnings } = importFormTable(bogus, 'whatever');
    expect(doc.form.name).toBe('whatever');
    expect(warnings).toHaveLength(1);
  });

  it('produces a document that round-trips through the form serializer', () => {
    const { doc } = importFormTable(table(HEADER_ROW, TESTDATE_FORM, TESTDATE_DATESPIN, commandButton('Command1'), TRAILING_ROW), 'testdate');
    const parsed = parseFormDocument(stringifyFormDocument(doc));
    expect(parsed.ok).toBe(true);
    if (parsed.ok) expect(parsed.doc).toEqual(doc);
  });
});

describe('importClassLibrary', () => {
  it('splits a two-class .vcx into one document per class', () => {
    const btn = row({
      PLATFORM: 'WINDOWS',
      CLASS: 'commandbutton',
      BASECLASS: 'commandbutton',
      OBJNAME: 'okbutton',
      PARENT: '',
      PROPERTIES: props('Caption = "OK"', 'Width = 60', 'Name = "okbutton"'),
      METHODS: ['PROCEDURE Click', 'THISFORM.Release()', 'ENDPROC', ''].join('\r\n'),
    });
    const container = row({
      PLATFORM: 'WINDOWS',
      CLASS: 'container',
      BASECLASS: 'container',
      OBJNAME: 'datespin',
      PARENT: '',
      PROPERTIES: props('Height = 40', 'Name = "datespin"'),
    });
    const inner = row({
      PLATFORM: 'WINDOWS',
      CLASS: 'label',
      BASECLASS: 'label',
      OBJNAME: 'Label1',
      PARENT: 'datespin',
      PROPERTIES: props('Caption = "Month"', 'Name = "Label1"'),
    });
    const classes = importClassLibrary(table(HEADER_ROW, btn, container, inner), 'datespin');
    expect(classes.map((c) => c.className)).toEqual(['okbutton', 'datespin']);

    const first = classes[0]!.imported;
    expect(first.warnings).toEqual([]);
    expect(first.doc.form.name).toBe('okbutton');
    expect(first.doc.form.props).toEqual({ Caption: 'OK', Width: 60 });
    expect(first.doc.form.methods).toEqual({ Click: 'THISFORM.Release()' });
    expect(first.doc.meta?.vfp?.baseClass).toBe('commandbutton');

    const second = classes[1]!.imported;
    expect(second.doc.form.name).toBe('datespin');
    expect(second.doc.form.children.map((c) => [c.type, c.name])).toEqual([['Label', 'Label1']]);
    expect(parseFormDocument(stringifyFormDocument(second.doc)).ok).toBe(true);
  });

  it('records the parent class library of a subclassed class', () => {
    const sub = row({
      PLATFORM: 'WINDOWS',
      CLASS: 'basebutton',
      CLASSLOC: 'base.vcx',
      BASECLASS: 'commandbutton',
      OBJNAME: 'okbutton',
      PARENT: '',
      PROPERTIES: props('Caption = "OK"', 'Name = "okbutton"'),
    });
    const [only] = importClassLibrary(table(HEADER_ROW, sub), 'buttons');
    expect(only!.imported.doc.meta?.vfp).toMatchObject({ classLib: 'base.vcx', baseClass: 'commandbutton' });
    expect(only!.imported.warnings).toHaveLength(1);
    expect(only!.imported.warnings[0]!.message).toContain('basebutton');
  });

  it('returns nothing for a table that is not a class library', () => {
    expect(importClassLibrary({ ok: true, version: 0x30, codepage: null, fields: [], records: [] }, 'x')).toEqual([]);
  });

  it('gives an inherited pageframe the page a subclass adds, under the name the subclass gives it', () => {
    // This is what `_tabhieroptions` does in the Foundation Classes: it inherits a pageframe
    // with three pages and adds a fourth, and every one of its own controls is parented to
    // that fourth page. The page has no row of its own - the pageframe makes it - so the
    // properties that name it are written on the *form* row, dotted through the pageframe.
    const base = row({
      PLATFORM: 'WINDOWS',
      CLASS: 'form',
      BASECLASS: 'form',
      OBJNAME: 'baseoptions',
      PARENT: '',
      PROPERTIES: props('Name = "baseoptions"'),
    });
    const frame = row({
      PLATFORM: 'WINDOWS',
      CLASS: 'pageframe',
      BASECLASS: 'pageframe',
      OBJNAME: 'pf1',
      PARENT: 'baseoptions',
      PROPERTIES: props('PageCount = 1', 'Page1.Name = "pg1"', 'Name = "pf1"'),
    });
    const sub = row({
      PLATFORM: 'WINDOWS',
      CLASS: 'baseoptions',
      BASECLASS: 'form',
      OBJNAME: 'suboptions',
      PARENT: '',
      PROPERTIES: props(
        'Name = "suboptions"',
        'pf1.PageCount = 2',
        'pf1.Page2.Caption = "Relations"',
        'pf1.Page2.Name = "pg2"',
        // an inherited page is addressed by the name it already has, not the one it was born with
        'pf1.pg1.Enabled = .F.',
      ),
    });
    const onNewPage = row({
      PLATFORM: 'WINDOWS',
      CLASS: 'label',
      BASECLASS: 'label',
      OBJNAME: 'Label1',
      PARENT: 'suboptions.pf1.pg2',
      PROPERTIES: props('Caption = "Child field"', 'Name = "Label1"'),
    });

    const classes = importClassLibrary(table(HEADER_ROW, base, frame, sub, onNewPage), 'options');
    const imported = classes.find((c) => c.className === 'suboptions')!.imported;
    expect(imported.warnings).toEqual([]);

    const pages = imported.doc.form.children[0]!.children!;
    expect(pages.map((p) => p.name)).toEqual(['pg1', 'pg2']);
    expect(pages[0]!.props).toMatchObject({ Enabled: false });
    expect(pages[1]!.props).toMatchObject({ Caption: 'Relations' });
    expect(pages[1]!.children!.map((c) => [c.type, c.name])).toEqual([['Label', 'Label1']]);
  });

  it('keeps the pages a middle class names when a third class inherits them', () => {
    // CodeMine's install dialog: frmWizardDialog makes the pageframe, frmInstallDialog names its
    // pages from the form row and puts controls on them, and the application's own dialog
    // inherits all of that with no rows of its own. The middle class's dotted page names have
    // to reach the pageframe, or its controls have no page to go on.
    const base = row({ PLATFORM: 'WINDOWS', CLASS: 'form', BASECLASS: 'form', OBJNAME: 'wizard', PARENT: '', PROPERTIES: props('Name = "wizard"') });
    const frame = row({
      PLATFORM: 'WINDOWS',
      CLASS: 'pageframe',
      BASECLASS: 'pageframe',
      OBJNAME: 'pgfSteps',
      PARENT: 'wizard',
      PROPERTIES: props('PageCount = 1', 'Name = "pgfSteps"'),
    });
    const middle = row({
      PLATFORM: 'WINDOWS',
      CLASS: 'wizard',
      BASECLASS: 'form',
      OBJNAME: 'install',
      PARENT: '',
      PROPERTIES: props('Name = "install"', 'pgfSteps.PageCount = 2', 'pgfSteps.Page1.Name = "pagRegistration"', 'pgfSteps.Page2.Name = "pagPaths"'),
    });
    const onPaths = row({
      PLATFORM: 'WINDOWS',
      CLASS: 'label',
      BASECLASS: 'label',
      OBJNAME: 'lblLocal',
      PARENT: 'install.pgfSteps.pagPaths',
      PROPERTIES: props('Name = "lblLocal"'),
    });
    const leaf = row({
      PLATFORM: 'WINDOWS',
      CLASS: 'install',
      BASECLASS: 'form',
      OBJNAME: 'appinstall',
      PARENT: '',
      PROPERTIES: props('Name = "appinstall"', 'pgfSteps.Height = 200'),
    });

    const classes = importClassLibrary(table(HEADER_ROW, base, frame, middle, onPaths, leaf), 'dialogs');
    const imported = classes.find((c) => c.className === 'appinstall')!.imported;
    expect(imported.warnings).toEqual([]);

    const frameNode = imported.doc.form.children[0]!;
    expect(frameNode.props).toMatchObject({ Height: 200 });
    const pages = frameNode.children!;
    expect(pages.map((p) => p.name)).toEqual(['pagRegistration', 'pagPaths']);
    expect(pages[1]!.children!.map((c) => c.name)).toEqual(['lblLocal']);
    expect(imported.doc.form.children).toHaveLength(1);
  });

  it("works out a class's own property written in parentheses, and keeps an unquoted one as text", () => {
    // CodeMine's cmApplicationManager: `nhkeymachineroot = ((2^31) + 2)` is HKEY_LOCAL_MACHINE to
    // every registry call handed it, while `cappname = ShutterDesign` is a string
    const app = row({
      PLATFORM: 'WINDOWS',
      CLASS: 'custom',
      BASECLASS: 'custom',
      OBJNAME: 'appmanager',
      PARENT: '',
      PROPERTIES: props('nhkeymachineroot = ((2^31) + 2)', 'cappname = ShutterDesign', 'cpair = (a) + (b)', 'Name = "appmanager"'),
    });
    const imported = importClassLibrary(table(HEADER_ROW, app), 'apps').find((c) => c.className === 'appmanager')!.imported;
    expect(imported.doc.meta?.vfp?.expressions).toEqual({ 'appmanager.nhkeymachineroot': '((2^31) + 2)' });
    expect(imported.doc.form.props['cappname']).toBe('ShutterDesign');
  });

  it('imports a Collection member the way it imports a Custom one', () => {
    const holder = row({
      PLATFORM: 'WINDOWS',
      CLASS: 'custom',
      BASECLASS: 'custom',
      OBJNAME: 'operation',
      PARENT: '',
      PROPERTIES: props('Name = "operation"'),
    });
    const list = row({
      PLATFORM: 'WINDOWS',
      CLASS: 'collection',
      BASECLASS: 'collection',
      OBJNAME: 'colParms',
      PARENT: 'operation',
      PROPERTIES: props('Top = 12', 'Left = 34', 'Name = "colParms"'),
    });
    const [only] = importClassLibrary(table(HEADER_ROW, holder, list), 'ws3client');
    expect(only!.imported.warnings).toEqual([]);
    expect(only!.imported.doc.form.children.map((c) => [c.type, c.name])).toEqual([['Collection', 'colParms']]);
    expect(only!.imported.doc.form.children[0]!.props).toEqual({ Top: 12, Left: 34 });
  });
});

// ---------------------------------------------------------------- header files

/**
 * A form and a class library each name a header file whose `#DEFINE`s its methods are compiled
 * with, in the eighth reserved field of a record. Measured in Visual FoxPro 9: a `.scx` carries
 * one for the whole file, on the `COMMENT`/`Screen` record the designer writes first, and a
 * `.vcx` carries one per class on the class's own row; the constants reach every method that
 * file holds, contained controls included, and reach nothing outside it.
 */
describe('the header file a form names', () => {
  const SCREEN = (include: string) => row({ PLATFORM: 'COMMENT', UNIQUEID: 'Screen', RESERVED8: include });

  const form = row({
    PLATFORM: 'WINDOWS',
    CLASS: 'form',
    BASECLASS: 'form',
    OBJNAME: 'Form1',
    PARENT: '',
    PROPERTIES: props('Name = "Form1"'),
    METHODS: 'PROCEDURE Init\r\n? GREETING\r\nENDPROC\r\n',
  });
  const label = row({
    PLATFORM: 'WINDOWS',
    CLASS: 'label',
    BASECLASS: 'label',
    OBJNAME: 'Label1',
    PARENT: 'Form1',
    PROPERTIES: props('Name = "Label1"'),
    METHODS: 'PROCEDURE Init\r\n? GREETING\r\nENDPROC\r\n',
  });

  it('reads it off the COMMENT record and puts it on the document', () => {
    const { doc } = importFormTable(table(SCREEN('sysinfo.h'), form, label), 'sysinfo');
    expect(doc.meta?.vfp?.include).toBe('sysinfo.h');
    // it covers every method in the file, so nothing needs an entry of its own
    expect(doc.meta?.vfp?.includes).toBeUndefined();
  });

  it('says nothing when the file names none', () => {
    const { doc } = importFormTable(table(SCREEN(''), form), 'plain');
    expect(doc.meta?.vfp?.include).toBeUndefined();
  });

  it('gives a method taken from a class library the header of that library, not of the form', () => {
    const library: VfpClassLibrary = {
      name: '../../ffc/_crypt.vcx',
      path: 'C:/vfp/Ffc/_crypt.vcx',
      table: table(
        row({ PLATFORM: 'COMMENT', UNIQUEID: 'Class' }),
        row({
          PLATFORM: 'WINDOWS',
          CLASS: 'custom',
          BASECLASS: 'custom',
          OBJNAME: '_cryptapi',
          PARENT: '',
          RESERVED8: 'wincrypt.h',
          PROPERTIES: props('Name = "_cryptapi"'),
          METHODS: 'PROCEDURE APISetup\r\n? T_CHARACTER\r\nENDPROC\r\n',
        }),
      ),
    };
    const instance = row({
      PLATFORM: 'WINDOWS',
      CLASS: '_cryptapi',
      CLASSLOC: '../../ffc/_crypt.vcx',
      BASECLASS: 'custom',
      OBJNAME: '_cryptapi',
      PARENT: 'Form1',
      PROPERTIES: props('Name = "_cryptapi"'),
    });
    const { doc } = importFormTable(table(SCREEN('crypto.h'), form, instance), 'crypto', [library]);

    expect(doc.meta?.vfp?.include).toBe('crypto.h');
    // the header is carried as a path from the library that brought the method in, because that
    // is where Visual FoxPro looks for it
    expect(doc.meta?.vfp?.includes?.['_cryptapi.apisetup']).toBe('C:/vfp/Ffc/wincrypt.h');
  });

  it('leaves a method the instance itself writes with the header of the form', () => {
    const library: VfpClassLibrary = {
      name: 'buttons.vcx',
      path: 'C:/project/buttons.vcx',
      table: table(
        row({ PLATFORM: 'COMMENT', UNIQUEID: 'Class' }),
        row({
          PLATFORM: 'WINDOWS',
          CLASS: 'commandbutton',
          BASECLASS: 'commandbutton',
          OBJNAME: 'okbutton',
          PARENT: '',
          RESERVED8: 'buttons.h',
          PROPERTIES: props('Name = "okbutton"'),
          METHODS: 'PROCEDURE Init\r\n? FROM_THE_LIBRARY\r\nENDPROC\r\n',
        }),
      ),
    };
    const instance = row({
      PLATFORM: 'WINDOWS',
      CLASS: 'okbutton',
      CLASSLOC: 'buttons.vcx',
      BASECLASS: 'commandbutton',
      OBJNAME: 'cmdOk',
      PARENT: 'Form1',
      PROPERTIES: props('Name = "cmdOk"'),
      METHODS: 'PROCEDURE Click\r\n? FROM_THE_FORM\r\nENDPROC\r\n',
    });
    const { doc } = importFormTable(table(SCREEN('form.h'), form, instance), 'dialog', [library]);

    // the class's Init came out of the library, the instance's Click out of the form
    expect(doc.meta?.vfp?.includes?.['cmdok.init']).toBe('C:/project/buttons.h');
    expect(doc.meta?.vfp?.includes?.['cmdok.click']).toBeUndefined();
  });

  it('gives every class in a .vcx the header its own row names', () => {
    const classes = importClassLibrary(
      table(
        row({ PLATFORM: 'COMMENT', UNIQUEID: 'Class' }),
        row({
          PLATFORM: 'WINDOWS',
          CLASS: 'custom',
          BASECLASS: 'custom',
          OBJNAME: 'registry',
          PARENT: '',
          RESERVED8: 'registry.h',
          PROPERTIES: props('Name = "registry"'),
        }),
        row({
          PLATFORM: 'WINDOWS',
          CLASS: 'custom',
          BASECLASS: 'custom',
          OBJNAME: 'plainclass',
          PARENT: '',
          PROPERTIES: props('Name = "plainclass"'),
        }),
      ),
      'registry',
    );
    expect(classes.map((c) => c.imported.doc.meta?.vfp?.include)).toEqual(['registry.h', undefined]);
  });

  it('reaches the methods of a member the class holds', () => {
    const classes = importClassLibrary(
      table(
        row({ PLATFORM: 'COMMENT', UNIQUEID: 'Class' }),
        row({
          PLATFORM: 'WINDOWS',
          CLASS: 'form',
          BASECLASS: 'form',
          OBJNAME: 'clock',
          PARENT: '',
          RESERVED8: 'clock.h',
          PROPERTIES: props('Name = "clock"'),
        }),
        row({
          PLATFORM: 'WINDOWS',
          CLASS: 'textbox',
          BASECLASS: 'textbox',
          OBJNAME: 'txtDate',
          PARENT: 'clock',
          PROPERTIES: props('Name = "txtDate"'),
          METHODS: 'PROCEDURE Init\r\n? MEMBER_CONSTANT\r\nENDPROC\r\n',
        }),
      ),
      'samples',
    );
    const [only] = classes;
    expect(only!.imported.doc.meta?.vfp?.include).toBe('clock.h');
    // the member's method takes the class's header, so it needs no entry of its own
    expect(only!.imported.doc.meta?.vfp?.includes).toBeUndefined();
  });
});
