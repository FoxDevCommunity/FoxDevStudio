import { describe, expect, it } from 'vitest';
import { collectNames, dedupeName, isValidName, uniqueName } from '@shared/form/naming';
import { sampleForm } from '../helpers/fixtures';

describe('naming', () => {
  it('generates the next free VFP-style name, case-insensitively', () => {
    const names = collectNames(sampleForm().form);
    expect(names.has('command1')).toBe(true);
    expect(names.has('label1')).toBe(true); // nested inside the page
    expect(uniqueName('Command', names)).toBe('Command3');
    expect(uniqueName('Text', names)).toBe('Text2');
    expect(uniqueName('Edit', names)).toBe('Edit1');
    expect(uniqueName('text', new Set(['TEXT1']))).toBe('text2');
  });
  it('dedupes pasted names by bumping the suffix', () => {
    const taken = new Set(['command1', 'command2']);
    expect(dedupeName('Command1', taken)).toBe('Command3');
    expect(dedupeName('cmdOk', taken)).toBe('cmdOk');
    expect(dedupeName('cmdOk', new Set(['cmdok']))).toBe('cmdOk1');
  });
  it('validates identifiers', () => {
    expect(isValidName('Command1')).toBe(true);
    expect(isValidName('_x')).toBe(true);
    expect(isValidName('1abc')).toBe(false);
    expect(isValidName('a b')).toBe(false);
    expect(isValidName('')).toBe(false);
  });
});
