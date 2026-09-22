import { readFileSync } from 'node:fs';
import { beforeAll, beforeEach, describe, expect, it } from 'vitest';
import { setApi } from '@renderer/api/foxdev';
import { createMemoryApi, type MemoryApi } from '@renderer/api/memoryApi';
import { useDocumentsStore } from '@renderer/stores/documentsStore';
import { useSessionStore } from '@renderer/runtime/session';
import { openFile } from '@renderer/stores/fileActions';
import { isVfpFile } from '@renderer/vfp/openVfpFile';
import { loadFoxVm } from '../../src/wasm/foxvm/loader';

/** Loose .scx and .vcx files, the way VFP sample code is usually distributed. */
const FIXTURES = 'crates/foxvm/tests/fixtures';
const D = '/demos';
let api: MemoryApi;

beforeAll(async () => {
  await loadFoxVm();
});

beforeEach(() => {
  api = createMemoryApi();
  for (const name of ['testdate.scx', 'testdate.sct', 'forms.vcx', 'forms.vct']) {
    api.binary$.set(`${D}/${name}`, new Uint8Array(readFileSync(`${FIXTURES}/${name}`)));
  }
  setApi(api);
  useDocumentsStore.getState().closeAll();
  useSessionStore.setState({ output: [] });
});

describe('opening a Visual FoxPro file directly', () => {
  it('recognises the DBF-based documents', () => {
    expect(isVfpFile('/x/Form.scx')).toBe(true);
    expect(isVfpFile('/x/LIB.VCX')).toBe(true);
    expect(isVfpFile('/x/main.prg')).toBe(false);
    expect(isVfpFile('/x/Form1.fxf')).toBe(false);
  });

  it('opens a loose .scx as a form tab', async () => {
    const id = await openFile(`${D}/testdate.scx`);
    const doc = useDocumentsStore.getState().docs[id];
    if (doc?.kind !== 'form') throw new Error('expected a form tab');

    expect(doc.store.getState().doc.form.props['Caption']).toBe('Date Spinner Sample');
    // untitled, so saving asks where to put it and the .scx is left alone
    expect(doc.path).toBeNull();
  });

  it('opens a .vcx as one tab per class', async () => {
    await openFile(`${D}/forms.vcx`);
    const forms = Object.values(useDocumentsStore.getState().docs).filter((d) => d.kind === 'form');
    expect(forms).toHaveLength(1);
    const names = useSessionStore.getState().output.map((o) => o.text).join(' ');
    expect(names).toContain('Imported 1 class(es)');
  });

  it('reports what it could not bring across', async () => {
    await openFile(`${D}/testdate.scx`);
    const errors = useSessionStore.getState().output.filter((o) => o.kind === 'error');
    expect(errors.some((e) => e.text.toLowerCase().includes('datespin'))).toBe(true);
  });
});
