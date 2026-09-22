import { describe, expect, it } from 'vitest';
import { createEmptyFormDocument, parseFormDocument, stringifyFormDocument } from '@shared/form/serialize';
import { sampleForm } from '../helpers/fixtures';

describe('form document serialization', () => {
  it('round trips and is stable', () => {
    const doc = sampleForm();
    const text = stringifyFormDocument(doc);
    const parsed = parseFormDocument(text);
    expect(parsed.ok).toBe(true);
    if (!parsed.ok) return;
    expect(parsed.doc).toEqual(doc);
    expect(stringifyFormDocument(parsed.doc)).toBe(text);
  });

  it('writes props and methods in alphabetical order for clean diffs', () => {
    const doc = createEmptyFormDocument();
    doc.form.props = { Width: 1, Caption: 'x', Height: 2 };
    const text = stringifyFormDocument(doc);
    expect(text.indexOf('"Caption"')).toBeLessThan(text.indexOf('"Height"'));
    expect(text.indexOf('"Height"')).toBeLessThan(text.indexOf('"Width"'));
    expect(text.endsWith('\n')).toBe(true);
  });

  it('rejects invalid documents with a readable error', () => {
    expect(parseFormDocument('{')).toMatchObject({ ok: false, error: expect.stringContaining('Invalid JSON') });
    expect(parseFormDocument('{"$schema":"foxdev-menu","version":1}')).toMatchObject({ ok: false, error: expect.stringContaining('$schema') });
    const bad = sampleForm() as unknown as { form: { children: unknown[] } };
    bad.form.children.push({ id: 'x', type: 'Bogus', name: 'x', props: {}, methods: {} });
    expect(parseFormDocument(JSON.stringify(bad))).toMatchObject({ ok: false, error: expect.stringContaining('type') });
  });
});

describe('the data environment', () => {
  it('survives a save: a form that opened its tables when imported still does after', () => {
    const doc = createEmptyFormDocument('frmOrders');
    doc.data = [
      { alias: 'solutions', source: 'solution.dbf' },
      { alias: 'customer', source: 'customer', database: '../data/testdata.dbc', order: 'cust_id', exclusive: true },
    ];
    const parsed = parseFormDocument(stringifyFormDocument(doc));
    expect(parsed.ok && parsed.doc.data).toEqual(doc.data);
  });
});
