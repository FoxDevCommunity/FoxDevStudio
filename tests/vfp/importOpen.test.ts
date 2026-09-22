import { readFileSync } from 'node:fs';
import { beforeAll, beforeEach, describe, expect, it } from 'vitest';
import { setApi } from '@renderer/api/foxdev';
import { createMemoryApi, type MemoryApi } from '@renderer/api/memoryApi';
import { useProjectStore } from '@renderer/stores/projectStore';
import { useDocumentsStore } from '@renderer/stores/documentsStore';
import { useSessionStore } from '@renderer/runtime/session';
import { openProjectItem } from '@renderer/stores/fileActions';
import { importVfpProject } from '@renderer/vfp/importVfpProject';
import { loadFoxVm } from '../../src/wasm/foxvm/loader';

/** End to end: import a real .pjx, then open one of its programs the way the explorer does. */
const FIXTURES = 'crates/foxvm/tests/fixtures';
const D = '/vfp/FormsUI';
let api: MemoryApi;

beforeAll(async () => {
  await loadFoxVm();
});

beforeEach(() => {
  api = createMemoryApi({
    // the programs the project lists, with the names VFP recorded
    [`${D}/1_formsui_gdiplusimaging.prg`]: '* imaging demo\n? "one"',
    [`${D}/2_formsui_pictureposition.prg`]: '? "two"',
    [`${D}/3_formsui_themes.prg`]: '? "three"',
    [`${D}/4_formsui_taborientation.prg`]: '? "four"',
    [`${D}/5_formsui_enablehyperlinks.prg`]: '? "five"',
    [`${D}/formsui_readme.txt`]: 'readme',
  });
  api.binary$.set(`${D}/formsui.pjx`, new Uint8Array(readFileSync(`${FIXTURES}/formsui.pjx`)));
  api.binary$.set(`${D}/formsui.PJT`, new Uint8Array(readFileSync(`${FIXTURES}/formsui.PJT`)));
  setApi(api);
  useProjectStore.getState().close();
  useDocumentsStore.getState().closeAll();
  useSessionStore.setState({ output: [] });
});

describe('an imported project opens its items', () => {
  it('writes a project whose paths resolve, and opens a program from it', async () => {
    const report = await importVfpProject(`${D}/formsui.pjx`);
    expect(report.projectPath).toBe(`${D}/formsui.fxproject`);

    await useProjectStore.getState().openProject(report.projectPath);
    const doc = useProjectStore.getState().doc;
    expect(doc?.name).toBe('formsui');
    expect(doc?.items.filter((i) => i.kind === 'program')).toHaveLength(5);

    // this is exactly what a double-click in the Project Explorer does
    const first = doc!.items.find((i) => i.kind === 'program')!;
    const id = await openProjectItem(first.path);
    const opened = useDocumentsStore.getState().docs[id];
    if (opened?.kind !== 'program') throw new Error('expected a program tab');
    expect(opened.text).toContain('imaging demo');
    expect(opened.path).toBe(`${D}/${first.path}`);
  });

  it('marks the main program the way VFP recorded it', async () => {
    const report = await importVfpProject(`${D}/formsui.pjx`);
    await useProjectStore.getState().openProject(report.projectPath);
    expect(useProjectStore.getState().doc?.main).toBe('1_formsui_gdiplusimaging.prg');
  });

  it('carries on when the path guard refuses one of the items', async () => {
    // solution.pjx refers to ..\..\classes\dragmove.cur, which is outside the folder the
    // session was given; the guard throws for those and the import must not die with them
    const real = api.files.exists;
    api.files.exists = async (path: string) => {
      if (path.endsWith('formsui_readme.txt')) throw new Error(`Access denied: ${path}`);
      return real(path);
    };

    const report = await importVfpProject(`${D}/formsui.pjx`);
    expect(report.skipped).toEqual([{ path: 'formsui_readme.txt', reason: 'file not found or outside the project folder' }]);
    // every other item still came through, and the project still opens
    expect(report.referenced).toHaveLength(5);
    await useProjectStore.getState().openProject(report.projectPath);
    expect(useProjectStore.getState().doc?.items.filter((i) => i.kind === 'program')).toHaveLength(5);
  });

  it('says so when a project lists nothing the IDE can open', async () => {
    // appwiz.pjx is one of the stubs Visual FoxPro ships: a header row and a bitmap
    api.binary$.set(`${D}/appwiz.pjx`, new Uint8Array(readFileSync(`${FIXTURES}/appwiz.pjx`)));
    api.binary$.set(`${D}/appwiz.pjt`, new Uint8Array(readFileSync(`${FIXTURES}/appwiz.pjt`)));
    api.files$.set(`${D}/foxqstrt.bmp`, 'BM');
    const report = await importVfpProject(`${D}/appwiz.pjx`);

    expect(report.skipped).toEqual([]);
    const output = useSessionStore.getState().output.map((o) => o.text).join('\n');
    expect(output).toContain('1 item(s)');
    expect(output).toContain('none of them is a form, menu, class library or program');

    // the bitmap is still listed, so nothing was lost on the way in
    await useProjectStore.getState().openProject(report.projectPath);
    expect(useProjectStore.getState().doc?.items.map((i) => i.path)).toEqual(['foxqstrt.bmp']);
  });

  it('files the readme under other, and reports nothing skipped', async () => {
    const report = await importVfpProject(`${D}/formsui.pjx`);
    expect(report.skipped).toEqual([]);
    await useProjectStore.getState().openProject(report.projectPath);
    expect(useProjectStore.getState().doc?.items.some((i) => i.kind === 'other' && i.path.endsWith('.txt'))).toBe(true);
  });
});
