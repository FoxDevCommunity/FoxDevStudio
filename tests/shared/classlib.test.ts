import { describe, expect, it } from 'vitest';
import {
  classMemberPath,
  findClass,
  ownsMethod,
  ownsProperty,
  type ClassDefinition,
  type ClassLibraryDocument,
} from '@shared/classlib/schema';
import { createClass, createEmptyClassLibraryDocument, parseClassLibraryDocument, stringifyClassLibraryDocument } from '@shared/classlib/serialize';

function library(): ClassLibraryDocument {
  const base: ClassDefinition = {
    name: 'buttonbase',
    baseClass: 'commandbutton',
    description: 'Every button in the application',
    props: { Height: 27, Width: 84, FontName: 'Tahoma' },
    methods: { Click: '* nothing yet' },
    children: [],
  };
  const derived: ClassDefinition = {
    name: 'okbutton',
    baseClass: 'commandbutton',
    parentClass: 'buttonbase',
    props: { Height: 27, Width: 84, FontName: 'Tahoma', Caption: 'OK', Default: true },
    methods: { Click: 'THISFORM.Release()' },
    children: [
      { id: 'a1', type: 'Label', name: 'Label1', props: { Caption: 'x' }, methods: {} },
    ],
    own: {
      okbutton: { props: ['Caption', 'Default'], methods: ['Click'] },
      'okbutton.Label1': { props: ['Caption'] },
    },
  };
  return { $schema: 'foxdev-classlib', version: 1, name: 'buttons', classes: [base, derived] };
}

describe('the class library document', () => {
  it('round-trips through text unchanged', () => {
    const doc = library();
    const parsed = parseClassLibraryDocument(stringifyClassLibraryDocument(doc));
    expect(parsed.ok).toBe(true);
    if (!parsed.ok) return;
    expect(parsed.doc).toEqual(doc);
    // and writing what was read gives the same bytes, so a save with no edit diffs to nothing
    expect(stringifyClassLibraryDocument(parsed.doc)).toBe(stringifyClassLibraryDocument(doc));
  });

  it('writes properties, methods and ownership in a fixed order', () => {
    const doc = library();
    const scrambled = library();
    scrambled.classes[1]!.props = { Default: true, Width: 84, Caption: 'OK', FontName: 'Tahoma', Height: 27 };
    scrambled.classes[1]!.own = {
      'okbutton.Label1': { props: ['Caption'] },
      okbutton: { props: ['Default', 'Caption'], methods: ['Click'] },
    };
    expect(stringifyClassLibraryDocument(scrambled)).toBe(stringifyClassLibraryDocument(doc));
  });

  it('ends with a newline and indents by two, as every other document does', () => {
    const text = stringifyClassLibraryDocument(library());
    expect(text.endsWith('\n')).toBe(true);
    expect(text).toContain('\n  "name": "buttons"');
  });

  it('an empty library is a library', () => {
    const doc = createEmptyClassLibraryDocument('Controls');
    const parsed = parseClassLibraryDocument(stringifyClassLibraryDocument(doc));
    expect(parsed.ok && parsed.doc.classes).toEqual([]);
  });

  it('a new class starts empty on the base class it was asked for', () => {
    expect(createClass('mybutton', 'commandbutton')).toEqual({ name: 'mybutton', baseClass: 'commandbutton', props: {}, methods: {}, children: [] });
  });

  it('keeps the class library meta, so a re-import knows which importer made it', () => {
    const doc = library();
    doc.meta = { vfp: { source: 'classes/buttons.vcx', importer: 4 } };
    const parsed = parseClassLibraryDocument(stringifyClassLibraryDocument(doc));
    expect(parsed.ok && parsed.doc.meta?.vfp?.source).toBe('classes/buttons.vcx');
  });
});

describe('what a class owns and what it inherits', () => {
  const doc = library();
  const base = findClass(doc, 'BUTTONBASE')!;
  const derived = findClass(doc, 'okbutton')!;

  it('finds a class however its name is spelled, as VFP does', () => {
    expect(base.name).toBe('buttonbase');
    expect(findClass(doc, 'nosuchclass')).toBeUndefined();
  });

  it('a class with no parent owns everything it has', () => {
    expect(ownsProperty(base, 'buttonbase', 'Height')).toBe(true);
    expect(ownsMethod(base, 'buttonbase', 'Click')).toBe(true);
  });

  it('a subclass owns what it declares and inherits the rest', () => {
    expect(ownsProperty(derived, 'okbutton', 'Caption')).toBe(true);
    expect(ownsProperty(derived, 'okbutton', 'DEFAULT')).toBe(true);
    expect(ownsProperty(derived, 'okbutton', 'Height')).toBe(false);
    expect(ownsProperty(derived, 'okbutton', 'FontName')).toBe(false);
    expect(ownsMethod(derived, 'okbutton', 'Click')).toBe(true);
    expect(ownsMethod(derived, 'okbutton', 'Init')).toBe(false);
  });

  it('answers about a member further down the class, not just about the class', () => {
    expect(ownsProperty(derived, classMemberPath('okbutton', 'Label1'), 'Caption')).toBe(true);
    expect(ownsProperty(derived, classMemberPath('okbutton', 'Label1'), 'Top')).toBe(false);
    // an object the class says nothing about came from the parent whole
    expect(ownsProperty(derived, 'okbutton.Label2', 'Caption')).toBe(false);
  });
});

describe('a malformed class library', () => {
  it('says what is wrong with the JSON', () => {
    const result = parseClassLibraryDocument('{ "classes": ');
    expect(result.ok).toBe(false);
    if (!result.ok) expect(result.error).toMatch(/^Invalid JSON: /);
  });

  it('names the field that is wrong', () => {
    const doc = library() as unknown as Record<string, unknown>;
    (doc['classes'] as ClassDefinition[])[0]!.baseClass = '' as string;
    const result = parseClassLibraryDocument(JSON.stringify(doc));
    expect(result.ok).toBe(false);
    if (!result.ok) expect(result.error).toContain('classes.0.baseClass');
  });

  it('refuses a document of another kind', () => {
    const result = parseClassLibraryDocument(JSON.stringify({ $schema: 'foxdev-form', version: 1, name: 'x', classes: [] }));
    expect(result.ok).toBe(false);
    if (!result.ok) expect(result.error).toContain('$schema');
  });

  it('refuses a class whose children are not controls', () => {
    const result = parseClassLibraryDocument(
      JSON.stringify({ $schema: 'foxdev-classlib', version: 1, name: 'x', classes: [{ name: 'c', baseClass: 'custom', props: {}, methods: {}, children: [{ type: 'Nope' }] }] }),
    );
    expect(result.ok).toBe(false);
    if (!result.ok) expect(result.error).toContain('classes.0.children.0');
  });
});
