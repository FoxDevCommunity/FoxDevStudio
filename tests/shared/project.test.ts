import { describe, expect, it } from 'vitest';
import { createEmptyProjectDocument, parseProjectDocument, stringifyProjectDocument } from '@shared/project/serialize';
import { kindForPath } from '@shared/project/schema';
import { sampleProject } from '../helpers/fixtures';

describe('project document', () => {
  it('round trips and drops falsy excluded flags', () => {
    const doc = sampleProject();
    doc.items[0]!.excluded = true;
    const text = stringifyProjectDocument(doc);
    const parsed = parseProjectDocument(text);
    expect(parsed).toMatchObject({ ok: true });
    if (parsed.ok) expect(parsed.doc).toEqual(doc);
    expect(text).toContain('"excluded": true');
    expect(stringifyProjectDocument(createEmptyProjectDocument('P'))).toContain('"items": []');
    expect(parseProjectDocument('[]').ok).toBe(false);
  });
  it('maps extensions to item kinds', () => {
    expect(kindForPath('forms/Main.FXF')).toBe('form');
    expect(kindForPath('Main.fxm')).toBe('menu');
    expect(kindForPath('main.prg')).toBe('program');
    expect(kindForPath('readme.txt')).toBe('other');
    expect(kindForPath('noext')).toBe('other');
  });
});
