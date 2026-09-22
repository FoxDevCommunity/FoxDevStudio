/**
 * Running a real Visual FoxPro formset, headless.
 *
 * `Solution\Forms\objects.scx` is a formset of two forms whose whole point is that they talk to
 * each other: a button on the right form sets the caption of the left one through THISFORMSET,
 * another hides and shows it, and Close does `RELEASE THISFORMSET`. Nothing about it works
 * unless the formset is a real object with both forms inside it, so it is the sample worth
 * running end to end.
 *
 * The forms are imported and named the way the project import names them - the file's own name
 * for the first, `<file>.<form>` for the rest - because that is how the runtime finds the rest of
 * a formset from the one it was asked for. It skips itself when Visual FoxPro is not installed.
 */

import { existsSync, readFileSync, readdirSync, statSync } from 'node:fs';
import { afterEach, beforeAll, describe, expect, it } from 'vitest';
import { importFormFile } from '@shared/vfp/importForm';
import { formsetDocumentName } from '@shared/form/formset';
import { parseFormDocument, stringifyFormDocument } from '@shared/form/serialize';
import type { FormDocument } from '@shared/form/schema';
import { baseName, formMethodSources, requireBytes, type ProgramSource } from '@shared/runtime/programSource';
import { createMemoryApi, type MemoryApi } from '@renderer/api/memoryApi';
import { setApi } from '@renderer/api/foxdev';
import { useProjectStore } from '@renderer/stores/projectStore';
import { useSessionStore } from '@renderer/runtime/session';
import { FormInstance, FormSetInstance, type RuntimeObject } from '@shared/runtime/objectModel';
import { readVfpTable } from '@renderer/vfp/openVfpFile';
import { loadClassLibraries } from '@renderer/vfp/classLibraries';
import { compileForm, compileProgram } from '@renderer/runtime/vmBridge';
import { loadFoxVm } from '../../src/wasm/foxvm/loader';

const SAMPLES = 'C:/Program Files (x86)/Microsoft Visual FoxPro 9/Samples';
const SOLUTION = `${SAMPLES}/Solution`;
const installed = existsSync(`${SOLUTION}/Forms/objects.scx`);

const TEXT = new Set(['prg', 'mpr', 'qpr', 'txt', 'h', 'ini']);

/** Every file directly in `dir`, as the file service would hold it. */
function seed(api: MemoryApi, dir: string): void {
  for (const entry of readdirSync(dir)) {
    const path = `${dir}/${entry}`;
    if (statSync(path).isDirectory()) continue;
    const ext = entry.slice(entry.lastIndexOf('.') + 1).toLowerCase();
    if (TEXT.has(ext)) api.files$.set(path, readFileSync(path, 'latin1'));
    else api.binary$.set(path, new Uint8Array(readFileSync(path)));
  }
}

/**
 * The documents one `.scx` becomes, under the names the project import gives them.
 *
 * A formset is several forms and so several documents; the first keeps the file's name and the
 * rest are told apart by the form's name after it.
 */
async function documentsOf(path: string): Promise<Map<string, FormDocument>> {
  const table = await readVfpTable(path);
  const dir = path.slice(0, path.lastIndexOf('/'));
  const { libraries } = await loadClassLibraries(table, dir, readVfpTable, [dir, `${dir}/..`, SOLUTION]);
  const stem = path.slice(path.lastIndexOf('/') + 1).replace(/\.[^.]+$/, '');
  const out = new Map<string, FormDocument>();
  for (const [i, entry] of importFormFile(table, stem, libraries).entries()) {
    // through the serializer, because the IDE runs what the import wrote rather than the import
    const saved = parseFormDocument(stringifyFormDocument(entry.imported.doc));
    if (!saved.ok) throw new Error(saved.error);
    out.set(formsetDocumentName(stem, entry.formName, i).toLowerCase(), saved.doc);
  }
  return out;
}

function sourceOver(docs: Map<string, FormDocument>): ProgramSource {
  return {
    async getForm(name) {
      const doc = docs.get(baseName(name).toLowerCase());
      if (!doc) return null;
      const bytes = requireBytes(doc.form.name, compileForm(doc.form.name, formMethodSources(doc)));
      return { name: doc.form.name, doc, bytes };
    },
    async getProgram(name) {
      const display = baseName(name);
      return { name: display, bytes: requireBytes(display, compileProgram(`? "${display}"`, display)) };
    },
    async getMenu() {
      return null;
    },
  };
}

const settle = () => new Promise((r) => setTimeout(r, 0));

/** Runs one line of FoxPro against the session and lets everything it started finish. */
async function run(source: ProgramSource, line: string): Promise<void> {
  await useSessionStore.getState().execute(source, line);
  for (let i = 0; i < 200 && useSessionStore.getState().status === 'running'; i++) await settle();
  for (let i = 0; i < 20; i++) await settle();
}

/** Ends the run and waits for everything it was holding to be let go of. */
async function stop(): Promise<void> {
  useSessionStore.getState().cancel();
  for (let i = 0; i < 50; i++) await settle();
}

/** Presses a button the way the window does, so its own method runs. */
async function click(button: RuntimeObject): Promise<void> {
  const outcome = button.desktop.dispatch(button, 'Click', []);
  if (outcome instanceof Promise) await outcome;
  for (let i = 0; i < 50; i++) await settle();
}

beforeAll(async () => {
  if (installed) await loadFoxVm();
}, 60_000);

describe.skipIf(!installed)('DO FORM on a formset', () => {
  let source: ProgramSource;

  beforeAll(async () => {
    const api = createMemoryApi();
    seed(api, `${SOLUTION}/Forms`);
    seed(api, SOLUTION);
    setApi(api);
    useProjectStore.setState({ path: `${SOLUTION}/solution.fxproject`, doc: null });
    source = sourceOver(await documentsOf(`${SOLUTION}/Forms/objects.scx`));
  }, 60_000);

  /** `DO FORM objects`, and the formset it made. */
  async function open(clauses = ''): Promise<FormSetInstance> {
    await run(source, `DO FORM objects${clauses}`);
    const sets = useSessionStore.getState().desktop!.formSets;
    return sets[sets.length - 1]!;
  }

  // released here rather than by cancel(): cancelling stops the scheduler first, and a Destroy
  // that cannot run leaves the forms standing
  afterEach(async () => {
    const desktop = useSessionStore.getState().desktop;
    for (const set of [...(desktop?.formSets ?? [])]) await desktop!.releaseFormSet(set);
    await stop();
  });

  it('opens the formset and every form in it, in the order the file defines them', async () => {
    const set = await open();
    const desktop = useSessionStore.getState().desktop!;

    expect(set.name).toBe('Formset1');
    expect(set.baseClass).toBe('Formset');
    expect(set.forms.map((f) => f.name)).toEqual(['frmleft', 'frmright']);
    // the formset holds the forms; the screen still holds each form as a window of its own
    expect(desktop.forms.map((f) => f.name)).toEqual(['frmleft', 'frmright']);
    expect(set.forms[0]!.parent).toBe(set);
    // DO FORM shows them, which a form built inside a formset is not otherwise
    expect(set.forms.map((f) => f.get('Visible'))).toEqual([true, true]);
  });

  it('reaches the other form through THISFORMSET, and hides and shows them together', async () => {
    const set = await open();
    const desktop = useSessionStore.getState().desktop!;
    const [left, right] = set.forms;

    // cmdCaption1.Click is `ThisFormSet.frmLeft.Caption = ALLTRIM(ThisFormSet.frmLeft.txtInput.Value)`
    left!.child('txtInput')!.set('Value', 'typed here ');
    expect(left!.get('Caption')).toBe('Left Form');
    await click(right!.child('cmdCaption1')!);
    expect(left!.get('Caption')).toBe('typed here');

    desktop.hideFormSet(set);
    expect(set.forms.map((f) => f.get('Visible'))).toEqual([false, false]);
    desktop.showFormSet(set);
    expect(set.forms.map((f) => f.get('Visible'))).toEqual([true, true]);
    expect(right!.alive).toBe(true);
  });

  it('lets go of every form when the formset is released', async () => {
    const set = await open();
    const desktop = useSessionStore.getState().desktop!;

    // cmdQuit.Click is `release thisformset`
    await click(set.forms[1]!.child('cmdQuit')!);
    expect(desktop.formSets).toEqual([]);
    expect(desktop.forms).toEqual([]);
    expect(set.alive).toBe(false);
  });

  it('gives the formset back to DO FORM ... NAME, with its forms on it', async () => {
    await run(source, 'PUBLIC oSet');
    const set = await open(' NAME oSet NOSHOW');
    expect(set).toBeInstanceOf(FormSetInstance);
    // NOSHOW builds the whole formset and puts none of it up
    expect(set.forms.map((f) => f.get('Visible'))).toEqual([false, false]);
    expect(set.forms[0]).toBeInstanceOf(FormInstance);

    await run(source, '? TRANSFORM(oSet.FormCount) + " " + oSet.Forms(2).Name + " " + oSet.frmLeft.Parent.Name');
    const output = useSessionStore.getState().output.filter((l) => l.kind === 'output');
    expect(output[output.length - 1]!.text).toBe('2 frmright Formset1');
  });
});
