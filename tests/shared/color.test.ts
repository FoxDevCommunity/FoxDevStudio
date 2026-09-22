import { describe, expect, it } from 'vitest';
import { formatColor, fromHex, parseColor, rgb, toHex, toRgb } from '@shared/form/color';

describe('VFP colors', () => {
  it('packs like VFP RGB()', () => {
    expect(rgb(255, 0, 0)).toBe(255);
    expect(rgb(0, 255, 0)).toBe(65280);
    expect(rgb(0, 0, 255)).toBe(16711680);
    expect(rgb(236, 233, 216)).toBe(14215660);
  });
  it('round trips through components and hex', () => {
    expect(toRgb(14215660)).toEqual({ r: 236, g: 233, b: 216 });
    expect(toHex(16711680)).toBe('#0000ff');
    expect(fromHex('#0000FF')).toBe(16711680);
    expect(fromHex('nope')).toBeNull();
  });
  it('parses the forms users type in the property sheet', () => {
    expect(parseColor('RGB(255,0,0)')).toBe(255);
    expect(parseColor(' rgb( 0 , 0 , 255 ) ')).toBe(16711680);
    expect(parseColor('0,128,0')).toBe(32768);
    expect(parseColor('#ff0000')).toBe(255);
    expect(parseColor('65280')).toBe(65280);
    expect(parseColor('')).toBeNull();
    expect(parseColor('red')).toBeNull();
    expect(parseColor('99999999')).toBeNull();
  });
  it('formats as VFP source', () => {
    expect(formatColor(rgb(1, 2, 3))).toBe('RGB(1,2,3)');
  });
});
