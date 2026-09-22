import { beforeEach, describe, expect, it } from 'vitest';
import { setApi } from '@renderer/api/foxdev';
import { createMemoryApi, type MemoryApi } from '@renderer/api/memoryApi';
import { useProjectStore } from '@renderer/stores/projectStore';
import { docTitle, isDocDirty, useDocumentsStore } from '@renderer/stores/documentsStore';
import { closeAllDocuments, closeDocument, closeProject, newForm, newProjectDialog, openFile, openFileDialog, openProjectDialog, openProjectItem, saveAll, saveDocument } from '@renderer/stores/fileActions';
import { stringifyFormDocument } from '@shared/form/serialize';
import { stringifyMenuDocument } from '@shared/menu/serialize';
import { stringifyProjectDocument } from '@shared/project/serialize';
import { sampleForm, sampleMenu, sampleProject } from '../helpers/fixtures';

let api: MemoryApi;
const P = '/proj';

beforeEach(() => {
  api = createMemoryApi({
    [`${P}/Sample.fxproject`]: stringifyProjectDocument(sampleProject()),
    [`${P}/Form1.fxf`]: stringifyFormDocument(sampleForm()),
    [`${P}/Main.fxm`]: stringifyMenuDocument(sampleMenu()),
    [`${P}/main.prg`]: 'DO FORM Form1',
  });
  setApi(api);
  useProjectStore.getState().close();
  useDocumentsStore.getState().closeAll();
});

describe('project store', () => {
  it('opens a project, records it as recent and resolves paths', async () => {
    expect(await useProjectStore.getState().openProject(`${P}/Sample.fxproject`)).toBe(true);
    const p = useProjectStore.getState();
    expect(p.doc?.name).toBe('Sample');
    expect(p.dir()).toBe(P);
    expect(p.resolvePath('Form1.fxf')).toBe(`${P}/Form1.fxf`);
    expect(p.relativePath(`${P}/sub/x.fxf`)).toBe('sub/x.fxf');
    expect(p.recent).toEqual([`${P}/Sample.fxproject`]);
    expect(await useProjectStore.getState().openProject('/nope.fxproject')).toBe(false);
    expect(useProjectStore.getState().error).toContain('File not found');
  });

  it('edits items and saves', async () => {
    await useProjectStore.getState().openProject(`${P}/Sample.fxproject`);
    const p = () => useProjectStore.getState();
    p().addItem(`${P}/forms/New.fxf`);
    p().addItem(`${P}/forms/New.fxf`); // no duplicates
    expect(p().doc!.items.map((i) => i.path)).toContain('forms/New.fxf');
    expect(p().doc!.items.filter((i) => i.path === 'forms/New.fxf')).toHaveLength(1);
    expect(p().dirty).toBe(true);
    p().setMain('forms/New.fxf');
    p().setExcluded('main.prg', true);
    p().removeItem('forms/New.fxf');
    expect(p().doc!.main).toBeUndefined();
    await p().save();
    expect(p().dirty).toBe(false);
    expect(api.files$.get(`${P}/Sample.fxproject`)).toContain('"excluded": true');
  });

  it('creates a new project', async () => {
    expect(await useProjectStore.getState().newProject('/new/App', 'App')).toBe(true);
    expect(useProjectStore.getState().path).toBe('/new/App/App.fxproject');
    expect(api.files$.has('/new/App/App.fxproject')).toBe(true);
  });
});

describe('file flows', () => {
  it('opens project items into typed tabs and focuses duplicates', async () => {
    await useProjectStore.getState().openProject(`${P}/Sample.fxproject`);
    const formId = await openProjectItem('Form1.fxf');
    const menuId = await openProjectItem('Main.fxm');
    const prgId = await openProjectItem('main.prg');
    const d = useDocumentsStore.getState();
    expect(d.order).toEqual([formId, menuId, prgId]);
    expect(d.docs[formId]!.kind).toBe('form');
    expect(d.docs[menuId]!.kind).toBe('menu');
    expect(d.docs[prgId]).toMatchObject({ kind: 'program', text: 'DO FORM Form1' });
    expect(docTitle(d.docs[formId]!)).toBe('Form1.fxf');
    expect(await openProjectItem('Form1.fxf')).toBe(formId);
    expect(useDocumentsStore.getState().activeId).toBe(formId);
    await expect(openFile(`${P}/missing.fxf`)).rejects.toThrow('File not found');
    api.files$.set(`${P}/bad.fxf`, '{"$schema":"foxdev-form"}');
    await expect(openFile(`${P}/bad.fxf`)).rejects.toThrow('version');
  });

  it('opens a CRLF program clean and saves it back with the line endings it came with', async () => {
    const crlf = 'LOCAL x\r\n? "hi"\r\n';
    api.files$.set(`${P}/crlf.prg`, crlf);
    const id = await openFile(`${P}/crlf.prg`);
    const doc = useDocumentsStore.getState().docs[id]!;

    expect(isDocDirty(doc)).toBe(false);
    expect(doc.kind === 'program' && doc.text).toBe('LOCAL x\n? "hi"\n');

    useDocumentsStore.getState().setProgramText(id, 'LOCAL x\n? "bye"\n');
    expect(isDocDirty(useDocumentsStore.getState().docs[id]!)).toBe(true);
    expect(await saveDocument(id)).toBe(true);
    expect(api.files$.get(`${P}/crlf.prg`)).toBe('LOCAL x\r\n? "bye"\r\n');
  });

  it('saves a new form via Save As, writes canonical JSON and adds it to the project', async () => {
    await useProjectStore.getState().openProject(`${P}/Sample.fxproject`);
    const id = newForm();
    const doc = useDocumentsStore.getState().docs[id]!;
    expect(docTitle(doc)).toBe('Form1 (new)');
    expect(doc.kind === 'form' && doc.store.getState().addControl('CommandButton', { left: 8, top: 8 })).toBeTruthy();
    expect(isDocDirty(useDocumentsStore.getState().docs[id]!)).toBe(true);

    expect(await saveDocument(id)).toBe(false); // dialog cancelled
    api.queueDialog('saveFile', `${P}/forms/Form2.fxf`);
    expect(await saveDocument(id)).toBe(true);
    expect(api.dialogCalls$.at(-1)).toMatchObject({ kind: 'saveFile', opts: { defaultPath: `${P}/Form1.fxf` } });
    const saved = useDocumentsStore.getState().docs[id]!;
    expect(saved.kind === 'form' && saved.path).toBe(`${P}/forms/Form2.fxf`);
    expect(isDocDirty(saved)).toBe(false);
    if (saved.kind === 'form') expect(api.files$.get(`${P}/forms/Form2.fxf`)).toBe(stringifyFormDocument(saved.store.getState().doc));
    const p = useProjectStore.getState();
    expect(p.doc!.items.map((i) => i.path)).toContain('forms/Form2.fxf');
    expect(p.dirty).toBe(false); // project saved along with the new item
  });

  it('prompts before closing dirty tabs and honours each answer', async () => {
    await useProjectStore.getState().openProject(`${P}/Sample.fxproject`);
    const id = await openProjectItem('main.prg');
    useDocumentsStore.getState().setProgramText(id, 'RETURN');
    api.queueDialog('message', 2); // Cancel
    expect(await closeDocument(id)).toBe(false);
    expect(useDocumentsStore.getState().docs[id]).toBeDefined();
    api.queueDialog('message', 1); // Don't save
    expect(await closeDocument(id)).toBe(true);
    expect(api.files$.get(`${P}/main.prg`)).toBe('DO FORM Form1');

    const id2 = await openProjectItem('main.prg');
    useDocumentsStore.getState().setProgramText(id2, 'RETURN');
    api.queueDialog('message', 0); // Save
    expect(await closeDocument(id2)).toBe(true);
    expect(api.files$.get(`${P}/main.prg`)).toBe('RETURN');
    expect(useDocumentsStore.getState().order).toEqual([]);
  });

  it('save all, close all and close project', async () => {
    await useProjectStore.getState().openProject(`${P}/Sample.fxproject`);
    const f = await openProjectItem('Form1.fxf');
    const m = await openProjectItem('Main.fxm');
    const fd = useDocumentsStore.getState().docs[f]!;
    const md = useDocumentsStore.getState().docs[m]!;
    if (fd.kind === 'form') fd.store.getState().setProp(['Command1'], 'Caption', 'Go');
    if (md.kind === 'menu') md.store.getState().setPrompt('new', 'N');
    useProjectStore.getState().setMain('Main.fxm');
    expect(await saveAll()).toBe(true);
    expect(api.files$.get(`${P}/Form1.fxf`)).toContain('"Caption": "Go"');
    expect(api.files$.get(`${P}/Main.fxm`)).toContain('"prompt": "N"');
    expect(api.files$.get(`${P}/Sample.fxproject`)).toContain('"main": "Main.fxm"');
    expect(await closeAllDocuments()).toBe(true);
    expect(useDocumentsStore.getState().order).toEqual([]);
    expect(await closeProject()).toBe(true);
    expect(useProjectStore.getState().doc).toBeNull();
  });

  it('open/new project dialogs', async () => {
    expect(await openProjectDialog()).toBe(false);
    api.queueDialog('openFile', `${P}/Sample.fxproject`);
    expect(await openProjectDialog()).toBe(true);
    expect(useProjectStore.getState().doc?.name).toBe('Sample');
    api.queueDialog('openFile', `${P}/main.prg`);
    expect(await openFileDialog()).toBeTruthy();
    api.queueDialog('pickFolder', '/apps/Inventory');
    expect(await newProjectDialog()).toBe(true);
    expect(useProjectStore.getState().doc?.name).toBe('Inventory');
    expect(useDocumentsStore.getState().order).toEqual([]);
  });
});
