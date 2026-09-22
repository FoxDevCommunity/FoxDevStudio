/**
 * Property values a form works out rather than carries.
 *
 * Every line of a `.scx` property memo is a Visual FoxPro expression: `Caption = "Find"` is the
 * expression `"Find"`, and `Caption = (STR(RECNO()))` is one that has to be run. VFP works the
 * second kind out as it builds the form. Kept as text they are neither a caption nor a path -
 * `Picture = (HOME() + "graphics\edit.bmp")` never finds its bitmap, and a class library named
 * that way reaches the file layer as its own source, which is what "Access denied:
 * (IIF(VERSION(2)=0,..." was.
 */

import { describe, expect, it } from 'vitest';
import { Desktop } from '@shared/runtime/objectModel';
import { readVfpValue } from '@shared/vfp/importForm';
import type { FormNode } from '@shared/form/schema';

function form(desktop: Desktop) {
  const node: FormNode = {
    name: 'Form1',
    props: {},
    methods: {},
    children: [
      { id: 'a', type: 'Label', name: 'lblRecord', props: { Caption: '' }, methods: {}, children: [] },
      { id: 'b', type: 'Image', name: 'imgLogo', props: { Picture: '' }, methods: {}, children: [] },
    ],
  };
  return desktop.instantiate(node, -1);
}

describe('a value written as an expression', () => {
  it('is told apart from one written down', () => {
    expect(readVfpValue('"Current Record:"')).toEqual({ value: 'Current Record:', expression: false });
    expect(readVfpValue('.T.')).toEqual({ value: true, expression: false });
    expect(readVfpValue('15')).toEqual({ value: 15, expression: false });
    expect(readVfpValue('')).toEqual({ value: '', expression: false });
    expect(readVfpValue('(STR(RECNO()))')).toEqual({ value: '(STR(RECNO()))', expression: true });
    expect(readVfpValue('(HOME() + "graphics\\edit.bmp")').expression).toBe(true);
  });

  it('is worked out as the form is built', async () => {
    const desktop = new Desktop();
    const instance = form(desktop);
    const asked: string[] = [];
    desktop.evaluate = async (source) => {
      asked.push(source);
      return source.includes('RECNO') ? '   7' : 'C:/vfp/graphics/edit.bmp';
    };

    await desktop.runFormLifecycle(instance, {
      expressions: {
        'Form1.lblRecord.Caption': '(STR(RECNO()))',
        'Form1.imgLogo.Picture': '(HOME() + "graphics\\edit.bmp")',
      },
    });

    expect(asked).toHaveLength(2);
    expect(instance.child('lblRecord')!.get('Caption')).toBe('   7');
    expect(instance.child('imgLogo')!.get('Picture')).toBe('C:/vfp/graphics/edit.bmp');
  });

  it('leaves the property alone when it cannot be worked out', async () => {
    const desktop = new Desktop();
    const instance = form(desktop);
    desktop.evaluate = async () => {
      throw new Error('Variable RECNO is not found');
    };

    await expect(
      desktop.runFormLifecycle(instance, { expressions: { 'Form1.lblRecord.Caption': '(STR(RECNO()))' } }),
    ).resolves.toBe(true);
    expect(instance.child('lblRecord')!.get('Caption')).toBe('');
  });

  it('ignores one that names something the form does not have', async () => {
    const desktop = new Desktop();
    const instance = form(desktop);
    desktop.evaluate = async () => 'x';

    await expect(
      desktop.runFormLifecycle(instance, { expressions: { 'Form1.lblGone.Caption': '(1)' } }),
    ).resolves.toBe(true);
  });
});
