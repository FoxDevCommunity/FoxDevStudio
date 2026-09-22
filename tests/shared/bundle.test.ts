import { readFileSync } from 'node:fs';
import { beforeAll, describe, expect, it } from 'vitest';
import { parseFormDocument } from '@shared/form/serialize';
import { parseMenuDocument } from '@shared/menu/serialize';
import { parseProjectDocument } from '@shared/project/serialize';
import type { ProjectDocument } from '@shared/project/schema';
import { createBundleSource, decodeBytes, encodeBytes, packBundle, parseBundle, stringifyBundle } from '@shared/runtime/bundle';
import { compileForm, compileProgram } from '@renderer/runtime/vmBridge';
import { loadFoxVm } from '../../src/wasm/foxvm/loader';

const sample = (file: string) => readFileSync(`resources/samples/${file}`, 'utf8');

function project(): ProjectDocument {
  const parsed = parseProjectDocument(sample('HelloWorld.fxproject'));
  if (!parsed.ok) throw new Error(parsed.error);
  return parsed.doc;
}

const inputs = (doc = project()) => ({
  project: doc,
  readItem: (relative: string) => Promise.resolve(sample(relative)),
  parseForm: parseFormDocument,
  parseMenu: parseMenuDocument,
});

let compilers: Parameters<typeof packBundle>[1];

beforeAll(async () => {
  const vm = await loadFoxVm();
  compilers = { compileForm, compileProgram, vmVersion: vm.version() };
});

describe('application bundle', () => {
  it('packs every included project item and records the main entry', async () => {
    const { bundle, log } = await packBundle(inputs(), compilers);

    expect(bundle.name).toBe('HelloWorld');
    expect(Object.keys(bundle.forms)).toEqual(['helloworld']);
    expect(Object.keys(bundle.menus)).toEqual(['main']);
    expect(Object.keys(bundle.programs)).toEqual(['main']);
    expect(bundle.main).toEqual({ kind: 'form', name: 'HelloWorld' });
    expect(bundle.vmVersion).toMatch(/^\d+\.\d+\.\d+$/);
    expect(log).toContain('Compiled form HelloWorld.fxf');
    expect(bundle.forms['helloworld']!.bytecode.length).toBeGreaterThan(0);
  });

  it('round-trips through text', async () => {
    const { bundle } = await packBundle(inputs(), compilers);
    const parsed = parseBundle(stringifyBundle(bundle));
    if (!parsed.ok) throw new Error(parsed.error);

    expect(parsed.doc).toEqual(bundle);
    expect(parsed.doc.forms['helloworld']!.doc.form.name).toBe('frmHello');
  });

  it('serves compiled code to the runtime through a bundle source', async () => {
    const { bundle } = await packBundle(inputs(), compilers);
    const source = createBundleSource(bundle);

    const form = await source.getForm('HelloWorld');
    expect(form?.name).toBe('frmHello');
    expect(form?.bytes.byteLength).toBeGreaterThan(0);
    expect(await source.getProgram('main.prg')).toMatchObject({ name: 'main' });
    expect((await source.getMenu('Main.fxm'))?.name).toBe('Main');
    expect(await source.getForm('NoSuch')).toBeNull();
  });

  it('excludes items marked excluded', async () => {
    const doc = project();
    doc.items = doc.items.map((i) => (i.kind === 'menu' ? { ...i, excluded: true } : i));
    const { bundle } = await packBundle(inputs(doc), compilers);
    expect(bundle.menus).toEqual({});
  });

  it('refuses to build without a main item, or when main is not included', async () => {
    const noMain = project();
    delete noMain.main;
    await expect(packBundle(inputs(noMain), compilers)).rejects.toThrow(/no main form or program/i);

    const missing = project();
    missing.main = 'Absent.fxf';
    await expect(packBundle(inputs(missing), compilers)).rejects.toThrow(/not part of the build/i);
  });

  it('fails the build when an item does not compile', async () => {
    const doc = project();
    const broken = {
      ...inputs(doc),
      readItem: (relative: string) => Promise.resolve(relative.endsWith('.prg') ? 'IF x\n' : sample(relative)),
    };
    await expect(packBundle(broken, compilers)).rejects.toThrow(/ENDIF/);
  });

  it('rejects text that is not a bundle', () => {
    expect(parseBundle('not json')).toMatchObject({ ok: false });
    expect(parseBundle('{"$schema":"foxdev-form"}')).toMatchObject({ ok: false });
  });

  it('encodes bytecode as base64 without loss', () => {
    const bytes = new Uint8Array([0, 1, 70, 88, 86, 77, 255, 128, 13, 10]);
    expect(decodeBytes(encodeBytes(bytes))).toEqual(bytes);
  });
});
