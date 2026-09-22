/**
 * Running the Visual FoxPro Solution sample, headless.
 *
 * "Run Main" on the imported Solution project is where the reports of unplaced runtime errors
 * come from - `Error 107` with no object and no line among them. A dialog can only show the
 * error it is given; this runs the same code with nothing to click, ignores every error so the
 * run keeps going, and prints all of them with the method and line they came from.
 *
 * It is the real `main.prg`, the real `.scx` files and the real `.dbf` files off disk, through
 * the real session store - the only thing replaced is the screen. It skips itself when Visual
 * FoxPro is not installed.
 */

import { existsSync, readFileSync, readdirSync, statSync } from 'node:fs';
import { beforeAll, describe, expect, it } from 'vitest';
import { importFormTable } from '@shared/vfp/importForm';
import { parseFormDocument, stringifyFormDocument } from '@shared/form/serialize';
import { importMenuTable } from '@shared/vfp/importMenu';
import { baseName, formMethodSources, requireBytes, type ProgramSource } from '@shared/runtime/programSource';
import type { MenuDocument } from '@shared/menu/schema';
import { createMemoryApi, type MemoryApi } from '@renderer/api/memoryApi';
import { setApi } from '@renderer/api/foxdev';
import { useProjectStore } from '@renderer/stores/projectStore';
import { useSessionStore } from '@renderer/runtime/session';
import { TreeView } from '@shared/runtime/oleObjects';
import { readVfpTable } from '@renderer/vfp/openVfpFile';
import { loadClassLibraries } from '@renderer/vfp/classLibraries';
import { compileForm, compileProgram } from '@renderer/runtime/vmBridge';
import { loadFoxVm } from '../../src/wasm/foxvm/loader';

const SAMPLES = 'C:/Program Files (x86)/Microsoft Visual FoxPro 9/Samples';
const DIR = `${SAMPLES}/Solution`;
const installed = existsSync(`${DIR}/main.prg`);

/** Text formats are read as text; everything else is bytes, as the file service does. */
const TEXT = new Set(['prg', 'mpr', 'qpr', 'txt', 'h', 'ini']);

/** Every file under the sample, keyed by its absolute forward-slash path. */
function seed(api: MemoryApi, dir: string): void {
  for (const entry of readdirSync(dir)) {
    const path = `${dir}/${entry}`;
    if (statSync(path).isDirectory()) {
      seed(api, path);
      continue;
    }
    const ext = entry.slice(entry.lastIndexOf('.') + 1).toLowerCase();
    if (TEXT.has(ext)) api.files$.set(path, readFileSync(path, 'latin1'));
    else api.binary$.set(path, new Uint8Array(readFileSync(path)));
  }
}

/**
 * The same API, reached case-insensitively.
 *
 * FoxPro is case-blind about file names and the sample relies on it: `DO FORM Sedona\DVDs`
 * against a folder that holds `dvds.scx`. Windows answers either; a Map does not.
 */
function caseBlind(api: MemoryApi): MemoryApi {
  const index = new Map<string, string>();
  const reindex = () => {
    index.clear();
    for (const key of [...api.files$.keys(), ...api.binary$.keys()]) index.set(key.toLowerCase(), key);
  };
  reindex();
  const real = (path: string) => index.get(path.replace(/\\/g, '/').toLowerCase()) ?? path;

  const files = api.files;
  api.files = {
    ...files,
    readText: (p) => files.readText(real(p)),
    readBytes: (p) => files.readBytes(real(p)),
    exists: (p) => files.exists(real(p)),
    remove: (p) => files.remove(real(p)),
    listDir: (p) => files.listDir(real(p)),
    writeText: async (p, text) => {
      await files.writeText(real(p), text);
      reindex();
    },
  };
  const data = api.data;
  api.data = { ...data, open: (p, exclusive) => data.open(real(p), exclusive) };
  return api;
}

/** Finds a design or program file by name, wherever in the sample tree it sits. */
function locator(api: MemoryApi) {
  const paths = [...api.files$.keys(), ...api.binary$.keys()];
  return (name: string, extensions: string[]): string | null => {
    const wanted = name.replace(/\\/g, '/').toLowerCase();
    for (const ext of extensions) {
      const match = paths.find((p) => {
        const lower = p.toLowerCase();
        if (!lower.endsWith(`.${ext}`)) return false;
        const stem = lower.slice(0, -(ext.length + 1));
        return stem === wanted || stem.endsWith(`/${wanted}`);
      });
      if (match) return match;
    }
    return null;
  };
}

/** Compiles from the sample folder the way the IDE compiles from open documents. */
function diskSource(api: MemoryApi): ProgramSource {
  const find = locator(api);
  return {
    async getForm(name) {
      const path = find(baseName(name), ['scx']);
      if (!path) return null;
      const table = await readVfpTable(path);
      const dir = path.slice(0, path.lastIndexOf('/'));
      const { libraries } = await loadClassLibraries(table, dir, readVfpTable);
      const stem = path.slice(path.lastIndexOf('/') + 1).replace(/\.[^.]+$/, '');
      const imported = importFormTable(table, stem, libraries).doc;
      // the IDE runs a form from the .fxf the import wrote, not from the import itself: a save
      // that loses part of the document loses it for every run after
      const saved = parseFormDocument(stringifyFormDocument(imported));
      if (!saved.ok) throw new Error(saved.error);
      const doc = saved.doc;
      const bytes = requireBytes(doc.form.name, compileForm(doc.form.name, formMethodSources(doc)));
      return { name: doc.form.name, doc, bytes };
    },
    async getProgram(name) {
      const path = find(baseName(name), ['prg', 'mpr', 'qpr']);
      if (!path) return null;
      const text = await api.files.readText(path);
      const display = path.slice(path.lastIndexOf('/') + 1).replace(/\.[^.]+$/, '');
      return { name: display, bytes: requireBytes(display, compileProgram(text, display)) };
    },
    async getMenu(name): Promise<MenuDocument | null> {
      const path = find(baseName(name), ['mnx']);
      if (!path) return null;
      const stem = path.slice(path.lastIndexOf('/') + 1).replace(/\.[^.]+$/, '');
      return importMenuTable(await readVfpTable(path), stem).doc;
    },
  };
}

interface Reported {
  code: number;
  program: string;
  line: number;
  message: string;
}

/** Answers every dialog and ignores every error, recording what was asked. */
function autopilot(errors: Reported[], dialogs: string[]): () => void {
  return useSessionStore.subscribe((state) => {
    if (state.dialog) {
      const { kind, text, resolve } = state.dialog;
      useSessionStore.setState({ dialog: null });
      dialogs.push(`${kind}: ${text}`);
      resolve(kind === 'message' ? 6 : '');
    }
    if (state.errorReport) {
      const { error, resolve } = state.errorReport;
      errors.push({ code: error.code, program: error.program, line: error.line, message: error.message });
      resolve(errors.length > 200 ? 'cancel' : 'ignore');
    }
  });
}

const settle = () => new Promise((r) => setTimeout(r, 0));

beforeAll(async () => {
  if (installed) await loadFoxVm();
}, 60_000);

describe.skipIf(!installed)('Run Main on the Solution sample', () => {
  it('reaches READ EVENTS without a runtime error', async () => {
    // the sample reaches out of its own folder: `..\data\country.dbf` and the Foundation Classes
    const loaded = createMemoryApi();
    seed(loaded, `${SAMPLES}/Data`);
    seed(loaded, `${SAMPLES}/Classes`);
    seed(loaded, DIR);
    const api = caseBlind(loaded);
    setApi(api);
    useProjectStore.setState({ path: `${DIR}/solution.fxproject`, doc: null });

    const errors: Reported[] = [];
    const dialogs: string[] = [];
    const stop = autopilot(errors, dialogs);
    const source = diskSource(api);

    void useSessionStore.getState().runProgram(source, 'main');
    // the run parks in READ EVENTS and stays there; everything under test happens before that
    for (let i = 0; i < 4000 && useSessionStore.getState().status !== 'waiting'; i++) await settle();

    const status = useSessionStore.getState().status;
    const forms = useSessionStore.getState().desktop?.forms.map((f) => f.name) ?? [];
    // the form's Init fills its TreeView from the solutions table; the nodes prove it ran
    const form = useSessionStore.getState().desktop?.forms[0];
    const tree = form?.child('pgf1')?.child('pagTree')?.child('oleTree')?.ole;
    const nodeCount = tree instanceof TreeView ? tree.nodeList.length : -1;
    const reported = useSessionStore
      .getState()
      .output.filter((l) => l.kind === 'error')
      .map((l) => l.text.replace(' (handled by the program)', ''));
    stop();
    useSessionStore.getState().cancel();
    setApi(undefined);

    expect({ status, forms }).toEqual({ status: 'waiting', forms: ['solutions'] });
    // The sample installs its own ON ERROR handler, so an error never reaches the error dialog.
    // These are the errors the VM raised, however the program went on to treat them.
    expect(reported).toEqual([]);
    expect(dialogs).toEqual([]);
    expect(errors).toEqual([]);
    // its Init walked the solutions table and built the tree
    expect(nodeCount).toBeGreaterThan(20);
  }, 120_000);
});
