import { describe, expect, it } from 'vitest';
import { basename, dirname, extname, isAbsolute, isInside, join, normalize, relative, resolveFrom } from '@shared/paths';

describe('paths', () => {
  it('handles posix and windows styles', () => {
    expect(dirname('/home/j/app/Form1.fxf')).toBe('/home/j/app');
    expect(dirname('C:\\Apps\\x\\Form1.fxf')).toBe('C:\\Apps\\x');
    expect(dirname('/x')).toBe('/');
    expect(dirname('file.txt')).toBe('.');
    expect(basename('/a/b/c.fxf')).toBe('c.fxf');
    expect(basename('C:\\a\\b\\c.fxf', true)).toBe('c');
    expect(extname('x/y.FXF')).toBe('.FXF');
    expect(extname('noext')).toBe('');
    expect(isAbsolute('/a')).toBe(true);
    expect(isAbsolute('C:\\a')).toBe(true);
    expect(isAbsolute('a/b')).toBe(false);
  });
  it('normalizes and joins', () => {
    expect(normalize('/a/./b/../c/')).toBe('/a/c');
    expect(normalize('a/../../b')).toBe('../b');
    expect(join('/a/b', 'c', 'd.fxf')).toBe('/a/b/c/d.fxf');
    expect(join('C:\\a', 'b.fxf')).toBe('C:\\a\\b.fxf');
    expect(resolveFrom('/proj', 'forms/x.fxf')).toBe('/proj/forms/x.fxf');
    expect(resolveFrom('/proj', '/abs/x.fxf')).toBe('/abs/x.fxf');
  });
  it('computes project-relative paths with forward slashes', () => {
    expect(relative('/proj', '/proj/forms/x.fxf')).toBe('forms/x.fxf');
    expect(relative('/proj/sub', '/proj/x.fxf')).toBe('../x.fxf');
    expect(relative('C:\\proj', 'c:\\proj\\Forms\\x.fxf')).toBe('Forms/x.fxf');
    expect(relative('/proj', '/proj')).toBe('.');
    expect(relative('C:\\proj', 'D:\\other\\x.fxf')).toBe('D:/other/x.fxf');
  });
  it('checks containment', () => {
    expect(isInside('/proj', '/proj/a/b')).toBe(true);
    expect(isInside('/proj', '/proj')).toBe(true);
    expect(isInside('/proj', '/project/a')).toBe(false);
    expect(isInside('/proj', '/proj/../etc/passwd')).toBe(false);
    expect(isInside('C:\\proj', 'c:/proj/x')).toBe(true);
  });
});
