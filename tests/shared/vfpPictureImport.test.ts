import { describe, expect, it } from 'vitest';
import { importFormTable } from '@shared/vfp/importForm';
import type { DbfTableData } from '@shared/vfp/dbfTypes';
import { Desktop } from '@shared/runtime/objectModel';

// Synthetic decoded SCX records; no application files or images are needed.
function importPicture(value: string, property = 'Picture') {
  const names = ['PLATFORM', 'CLASS', 'BASECLASS', 'OBJNAME', 'PARENT', 'PROPERTIES'];
  const table: DbfTableData = {
    ok: true, version: 0x30, codepage: 1252,
    fields: names.map(name => ({ name, kind: 'M', length: 4, decimals: 0 })),
    records: [
      { deleted: false, values: ['WINDOWS', 'form', 'form', 'Form1', '', 'Name = "Form1"'] },
      { deleted: false, values: ['WINDOWS', 'image', 'image', 'Image1', 'Form1', `${property} = ${value}\r\nName = "Image1"`] },
    ],
  };
  return importFormTable(table, 'sample');
}

describe('SCX image filenames', () => {
  it.each(['sample.bmp', '..\\images\\sample.bmp', 'C:\\Sample Images\\logo.PNG', 'images/logo.jpg'])('imports %s as a literal Picture', (path) => {
    const { doc, warnings } = importPicture(path);
    expect(warnings).toEqual([]);
    expect(doc.form.children[0]!.props.Picture).toBe(path);
    expect(doc.meta?.vfp?.expressions?.['Form1.Image1.Picture']).toBeUndefined();
  });

  it.each(['"sample.bmp"', "'sample.bmp'", '[sample.bmp]'])('preserves quoted literal %s', (value) => {
    expect(importPicture(value).doc.form.children[0]!.props.Picture).toBe('sample.bmp');
  });

  it.each(['(HOME() + "graphics\\sample.bmp")', 'm.imagePath', 'THISFORM.imagePath', 'THISFORM.logo.bmp', '&folder.sample.bmp', 'GETPICT()', '"images/" + m.imageName'])('keeps expression %s', (value) => {
    const { doc } = importPicture(value);
    expect(doc.meta?.vfp?.expressions?.['Form1.Image1.Picture']).toBe(value);
    expect(doc.form.children[0]!.props.Picture).toBeUndefined();
  });

  it('lets a later literal replace a deferred Picture expression', () => {
    const { doc } = importPicture('(GETPICT())\r\nPicture = sample.bmp');
    expect(doc.form.children[0]!.props.Picture).toBe('sample.bmp');
    expect(doc.meta?.vfp?.expressions?.['Form1.Image1.Picture']).toBeUndefined();
  });

  it('does not reinterpret an unrelated property with filename-like text', () => {
    const { doc } = importPicture('sample.bmp', 'ToolTipText');
    expect(doc.meta?.vfp?.expressions?.['Form1.Image1.ToolTipText']).toBe('sample.bmp');
  });

  it('makes the imported filename available at runtime without evaluating it', async () => {
    const { doc } = importPicture('sample.bmp');
    const desktop = new Desktop();
    const instance = desktop.instantiate(doc.form, -1);
    const evaluated: string[] = [];
    desktop.evaluate = async (source) => { evaluated.push(source); throw new Error('Not an expression'); };
    await desktop.runFormLifecycle(instance, { expressions: doc.meta?.vfp?.expressions });
    expect(evaluated).toEqual([]);
    expect(instance.child('Image1')!.get('Picture')).toBe('sample.bmp');
  });
});
