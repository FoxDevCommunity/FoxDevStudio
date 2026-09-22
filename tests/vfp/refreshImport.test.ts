/**
 * Converting a document again when the importer has moved on.
 *
 * The case that matters is the one already on disk: a `.fxf` produced by an older importer, which
 * records nothing about where it came from because that importer did not write it down. Those are
 * the documents every existing project is made of, and they are the ones that keep failing in the
 * way the importer was just fixed. If the refresh only helps documents imported after the fix, it
 * helps nobody who already has a project.
 *
 * It skips itself when Visual FoxPro is not installed.
 */

import { existsSync, readFileSync, readdirSync } from 'node:fs';
import { beforeAll, describe, expect, it } from 'vitest';
import { setApi } from '@renderer/api/foxdev';
import { createMemoryApi, type MemoryApi } from '@renderer/api/memoryApi';
import { refreshed } from '@renderer/vfp/refreshImport';
import { parseFormDocument, stringifyFormDocument } from '@shared/form/serialize';
import type { ControlNode, FormNode } from '@shared/form/schema';
import { loadFoxVm } from '../../src/wasm/foxvm/loader';

const DIR = 'C:/Program Files (x86)/Microsoft Visual FoxPro 9/Samples/Solution';
const installed = existsSync(`${DIR}/solution.scx`);
const PROJECT = '/proj';

let api: MemoryApi;

beforeAll(async () => {
  if (installed) await loadFoxVm();
});

/** A project folder holding the Visual FoxPro files, as an imported project does. */
function seed(): void {
  api = createMemoryApi();
  for (const name of readdirSync(DIR)) {
    const lower = name.toLowerCase();
    if (!/\.(scx|sct|vcx|vct)$/.test(lower)) continue;
    api.binary$.set(`${PROJECT}/${name}`, new Uint8Array(readFileSync(`${DIR}/${name}`)));
  }
  setApi(api);
}

function nodes(form: FormNode): (FormNode | ControlNode)[] {
  const out: (FormNode | ControlNode)[] = [form];
  const walk = (children: ControlNode[] | undefined) => {
    for (const c of children ?? []) {
      out.push(c);
      walk(c.children);
    }
  };
  walk(form.children);
  return out;
}

const hasProp = (form: FormNode, name: string) =>
  nodes(form).some((o) => Object.keys(o.props).some((k) => k.toLowerCase() === name.toLowerCase()));

describe.skipIf(!installed)('refreshing an imported document', () => {
  it('converts a document an older importer left beside its .scx, with nothing recorded', async () => {
    seed();
    // what an old import produced: the form, and no note of where it came from
    const stale: FormNode = { name: 'solutions', props: { Caption: 'old' }, methods: {}, children: [] };
    const path = `${PROJECT}/solution.fxf`;
    api.files$.set(path, stringifyFormDocument({ $schema: 'foxdev-form', version: 1, form: stale }));

    const fresh = await refreshed({ $schema: 'foxdev-form', version: 1, form: stale }, path);

    expect(hasProp(fresh.form, 'lCalledBySolution'), 'lCalledBySolution').toBe(true);
    expect(hasProp(fresh.form, 'cOldPath'), 'cOldPath').toBe(true);
    expect(fresh.meta?.vfp?.importer).toBeGreaterThan(0);

    // and it is written back, so the next open costs nothing
    const written = parseFormDocument(api.files$.get(path) ?? '');
    expect(written.ok).toBe(true);
    if (written.ok) expect(hasProp(written.doc.form, 'lCalledBySolution')).toBe(true);
  });

  it('leaves a document alone when the importer that made it is current', async () => {
    seed();
    const form: FormNode = { name: 'solutions', props: {}, methods: {}, children: [] };
    const doc = { $schema: 'foxdev-form' as const, version: 1 as const, form, meta: { vfp: { importer: 99 } } };
    const path = `${PROJECT}/solution.fxf`;
    expect(await refreshed(doc, path)).toBe(doc);
  });
});
