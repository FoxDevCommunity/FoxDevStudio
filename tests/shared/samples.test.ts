import { readFileSync, writeFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { describe, expect, it } from 'vitest';
import { parseFormDocument, stringifyFormDocument } from '@shared/form/serialize';
import { parseMenuDocument, stringifyMenuDocument } from '@shared/menu/serialize';
import { parseProjectDocument, stringifyProjectDocument } from '@shared/project/serialize';
import { collectNames } from '@shared/form/naming';

const dir = resolve(__dirname, '../../resources/samples');
const read = (f: string) => readFileSync(resolve(dir, f), 'utf8');
/** Run with REGEN_SAMPLES=1 to rewrite the samples in canonical form after editing them by hand. */
const regen = process.env['REGEN_SAMPLES'] === '1';
const check = (f: string, canonical: string) => {
  if (regen) writeFileSync(resolve(dir, f), canonical);
  expect(canonical).toBe(read(f));
};

describe('sample project files', () => {
  it('HelloWorld.fxproject parses and lists its files', () => {
    const r = parseProjectDocument(read('HelloWorld.fxproject'));
    expect(r).toMatchObject({ ok: true });
    if (!r.ok) return;
    expect(r.doc.items.map((i) => i.path)).toEqual(['HelloWorld.fxf', 'Main.fxm', 'main.prg']);
    check('HelloWorld.fxproject', stringifyProjectDocument(r.doc));
  });
  it('HelloWorld.fxf parses, has unique names and is canonically formatted', () => {
    const r = parseFormDocument(read('HelloWorld.fxf'));
    expect(r).toMatchObject({ ok: true });
    if (!r.ok) return;
    expect(collectNames(r.doc.form).size).toBe(10);
    check('HelloWorld.fxf', stringifyFormDocument(r.doc));
  });
  it('Main.fxm parses', () => {
    const r = parseMenuDocument(read('Main.fxm'));
    expect(r).toMatchObject({ ok: true });
    if (!r.ok) return;
    expect(r.doc.items[0]!.children!.length).toBe(3);
    check('Main.fxm', stringifyMenuDocument(r.doc));
  });
});
