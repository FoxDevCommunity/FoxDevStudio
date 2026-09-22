/**
 * Opening the forms Visual FoxPro ships with, rather than only compiling them.
 *
 * `samples.test.ts` reads every sample and compiles every method in it, which says the language
 * is understood. It says nothing about what happens when one is opened: the data environment,
 * the object model, every property expression and every Init in the tree. Every live bug
 * reported against this product so far has been in that half - a control's Init refusing, an
 * array property that was never dimensioned, a relation that was dropped - and none of them
 * could have been caught by compiling.
 *
 * So this opens them. What a form says while it opens is listed in `samples-run-known.txt`, one
 * line each, and anything not on that list fails the test; run with `RUN_REPORT=<file>` to write
 * the list out again. It skips itself when Visual FoxPro is not installed.
 */

import { existsSync, readFileSync, readdirSync, statSync, writeFileSync } from 'node:fs';
import { beforeAll, describe, expect, it } from 'vitest';
import { importFormFile } from '@shared/vfp/importForm';
import { importMenuTable } from '@shared/vfp/importMenu';
import { formsetDocumentName } from '@shared/form/formset';
import { parseFormDocument, stringifyFormDocument } from '@shared/form/serialize';
import type { FormDocument } from '@shared/form/schema';
import type { MenuDocument } from '@shared/menu/schema';
import type { FoxDevApi, OleValue } from '@shared/ipc/api';
import { baseName, formHeaderRefs, formMethodSources, requireBytes, type ProgramSource } from '@shared/runtime/programSource';
import { createMemoryApi, type MemoryApi } from '@renderer/api/memoryApi';
import { setApi } from '@renderer/api/foxdev';
import { useProjectStore } from '@renderer/stores/projectStore';
import { useSessionStore } from '@renderer/runtime/session';
import { readVfpTable } from '@renderer/vfp/openVfpFile';
import { loadClassLibraries } from '@renderer/vfp/classLibraries';
import { compileForm, compileProgram } from '@renderer/runtime/vmBridge';
import { readHeaderFiles } from '@renderer/runtime/headerFiles';
import { loadFoxVm } from '../../src/wasm/foxvm/loader';

// where the product is installed, which is what its own HOME() answers
const HOME = 'C:/Program Files (x86)/Microsoft Visual FoxPro 9';
const SAMPLES = `${HOME}/Samples`;
const SOLUTION = `${SAMPLES}/Solution`;
const KNOWN = 'tests/vfp/samples-run-known.txt';
const installed = existsSync(`${SOLUTION}/Forms/objects.scx`);

/**
 * The two main-process services a form reaches out of the runtime with: a native library and COM.
 *
 * The application gives the renderer both, so a run that withheld them would be asking a
 * question the product is never asked - a sample that calls `GetSystemTime` or hosts a Rich Text
 * control would be answered by the harness rather than by Windows, and the answer would be
 * recorded here as though it were the runtime's. They are loaded by a path held in a variable,
 * as `tests/win32/win32api.test.ts` loads the library service, so that this file - which belongs
 * to the renderer's half of the build - does not pull the main process's half in at compile time.
 */
interface DllService {
  available(): boolean;
  call(request: Parameters<FoxDevApi['dll']['call']>[0]): Awaited<ReturnType<FoxDevApi['dll']['call']>>;
}
type OleReply = { ok: true; value: OleValue } | { ok: false; error: string };
interface OleService {
  available(): boolean;
  perform(op: string, args: unknown[]): OleReply;
}
const dllServicePath = '../../src/main/services/dllService';
const oleServicePath = '../../src/main/services/oleService';
const { createDllService } = (await import(/* @vite-ignore */ dllServicePath)) as { createDllService: () => DllService };
const { createOleService } = (await import(/* @vite-ignore */ oleServicePath)) as {
  createOleService: (dirs: string[]) => OleService;
};

/**
 * Extensions the file service holds as text; everything else is bytes.
 *
 * A `.dbc` is a table and belongs with the bytes: held as text it is handed to the data engine
 * re-encoded, and a form that opens the database it names is told its field descriptors are
 * broken rather than being given the container.
 */
const TEXT = new Set(['prg', 'mpr', 'qpr', 'txt', 'h', 'ini', 'log']);

/** Every file in `dir` and the directories under it, as the file service would hold it. */
function seed(api: MemoryApi, dir: string, depth = 2): void {
  for (const entry of readdirSync(dir)) {
    const path = `${dir}/${entry}`;
    if (statSync(path).isDirectory()) {
      if (depth > 0) seed(api, path, depth - 1);
      continue;
    }
    const ext = entry.slice(entry.lastIndexOf('.') + 1).toLowerCase();
    if (TEXT.has(ext)) api.files$.set(path, readFileSync(path, 'latin1'));
    else api.binary$.set(path, new Uint8Array(readFileSync(path)));
  }
}

/**
 * The class libraries a sample names outside the Solution folder.
 *
 * A control's `CLASSLOC` is a path relative to the form it sits on, and plenty of them leave the
 * samples entirely: `..\..\..\classes\samples.vcx` is the stop watch and the mover pair the
 * Controls samples are built from, and `..\..\..\ffc\_movers.vcx` and its neighbours are the
 * Foundation Classes the Ffc samples exist to show off. The product follows those paths into its
 * own installation. Without them here the control arrives as the bare base class it stands on
 * and the first line of form code asking it for one of its own members raises - a hole in the
 * harness rather than in the runtime, and one that hid eight entries on the list below.
 *
 * Only the libraries themselves are seeded, and not the rest of those folders, because the
 * wizards alone are fourteen megabytes of things no sample ever opens.
 */
function seedLibraries(api: MemoryApi, dir: string): void {
  if (!existsSync(dir)) return;
  for (const entry of readdirSync(dir)) {
    const path = `${dir}/${entry}`;
    if (statSync(path).isDirectory()) continue;
    if (!/\.(vcx|vct|h)$/i.test(entry)) continue;
    if (entry.toLowerCase().endsWith('.h')) api.files$.set(path, readFileSync(path, 'latin1'));
    else api.binary$.set(path, new Uint8Array(readFileSync(path)));
  }
}

/** Every file under `dir` whose name ends in `ext`, in a stable order. */
function filesUnder(dir: string, ext: string): string[] {
  const out: string[] = [];
  const walk = (at: string): void => {
    for (const entry of readdirSync(at).sort()) {
      const path = `${at}/${entry}`;
      if (statSync(path).isDirectory()) walk(path);
      else if (entry.toLowerCase().endsWith(ext)) out.push(path);
    }
  };
  walk(dir);
  return out;
}

/** What a `.scx` already imported became, so a form opened twice is read and imported once. */
const imported = new Map<string, Promise<Map<string, FormDocument>>>();

/** The documents one `.scx` becomes, under the names the project import gives them. */
function documentsOf(path: string): Promise<Map<string, FormDocument>> {
  let docs = imported.get(path);
  if (docs === undefined) imported.set(path, (docs = readDocuments(path)));
  return docs;
}

async function readDocuments(path: string): Promise<Map<string, FormDocument>> {
  const table = await readVfpTable(path);
  const dir = path.slice(0, path.lastIndexOf('/'));
  const { libraries } = await loadClassLibraries(table, dir, readVfpTable, [dir, `${dir}/..`, SOLUTION]);
  const stem = path.slice(path.lastIndexOf('/') + 1).replace(/\.[^.]+$/, '');
  const out = new Map<string, FormDocument>();
  for (const [i, entry] of importFormFile(table, stem, libraries).entries()) {
    // the project importer writes down where a form came from, and a form's data environment
    // names its tables relative to that, so the harness records it the same way
    entry.imported.doc.meta = { ...entry.imported.doc.meta, vfp: { ...entry.imported.doc.meta?.vfp, source: path.slice(SOLUTION.length + 1) } };
    // through the serializer, because the IDE runs what the import wrote rather than the import
    const saved = parseFormDocument(stringifyFormDocument(entry.imported.doc));
    if (!saved.ok) throw new Error(saved.error);
    out.set(formsetDocumentName(stem, entry.formName, i).toLowerCase(), saved.doc);
  }
  return out;
}

/**
 * The form a neighbouring `.scx` holds, by the name the file itself carries, and the folder it
 * came out of, which is where its own header file is looked for.
 *
 * `DO FORM Multi` names a file and gets what is first in it, which for a formset is the formset
 * rather than a form inside it - the same thing the project source hands the runtime.
 */
async function siblingForm(forms: Map<string, string>, stem: string): Promise<{ doc: FormDocument; dir: string } | undefined> {
  const path = forms.get(stem);
  if (path === undefined) return undefined;
  const docs = await documentsOf(path);
  const doc = docs.get(stem) ?? [...docs.values()][0];
  return doc && { doc, dir: path.slice(0, path.lastIndexOf('/')) };
}

/**
 * @param docs the form under test, by the name `DO FORM` names it under.
 * @param dir the folder that form came out of, which is where its header file is looked for.
 * @param forms every other `.scx` the samples ship, by its own name, for a form that opens one.
 * @param menus the `.mnx` a `DO <name>.mpr` was generated from.
 */
function sourceOver(
  docs: Map<string, FormDocument>,
  dir: string,
  forms: Map<string, string>,
  menus: Map<string, string>,
): ProgramSource {
  return {
    // A form is free to open another one, and several of these do: `DO FORM Multi NAME
    // THISFORM.aForms[nInstance]` in the launcher, and `DO FORM (ADDBS(JUSTPATH(SYS(1271,
    // THISFORM))) + "imageview.scx")` in the docking sample, which works the name out as it
    // runs. In the IDE the project answers for every form in it; here the samples folder is the
    // project, so a name that is not the form under test is looked for among its neighbours.
    async getForm(name) {
      const stem = baseName(name).toLowerCase();
      const here = docs.get(stem);
      const sibling = here ? undefined : await siblingForm(forms, stem);
      const doc = here ?? sibling?.doc;
      if (!doc) return null;
      // the header file the form named, looked for beside it, then in the samples root, then
      // where Visual FoxPro is installed: that is the form's own folder, the default directory,
      // and the product's own search path, which is where `foxpro.h` and the Foundation Classes'
      // headers live. FoxDev looks in the folder it ships those in for the same reason. A form
      // that opened another one is beside its own header, not beside the caller's.
      const headers = await readHeaderFiles(formHeaderRefs(doc), [sibling?.dir ?? dir, SOLUTION, HOME]);
      const bytes = requireBytes(doc.form.name, compileForm(doc.form.name, formMethodSources(doc), headers));
      return { name: doc.form.name, doc, bytes };
    },
    // a sample's own programs are not what is under test here, so one that is asked for says so
    async getProgram(name) {
      const display = baseName(name);
      return { name: display, bytes: requireBytes(display, compileProgram(`? "${display}"`, display)) };
    },
    // `DO chkmenu.mpr` names a menu, and the project source finds the `.mnx` it was generated
    // from in the project; the samples ship theirs beside the form, so they are found there
    async getMenu(name): Promise<MenuDocument | null> {
      const path = menus.get(baseName(name).toLowerCase());
      if (path === undefined) return null;
      return importMenuTable(await readVfpTable(path), baseName(path)).doc;
    },
  };
}

const settle = (): Promise<unknown> => new Promise((r) => setTimeout(r, 0));
const deadline = (ms: number): Promise<unknown> => new Promise((r) => setTimeout(r, ms));

/** How long one form is given to open before the session is cancelled and the next one starts. */
const BUDGET_MS = 4000;

/** Everything one form said while it opened, with the noise a headless run always makes taken out. */
function complaints(): string[] {
  return useSessionStore
    .getState()
    .output.filter((l) => l.kind === 'error')
    .map((l) => l.text.trim())
    .filter((t) => t !== '');
}

beforeAll(async () => {
  if (installed) await loadFoxVm();
}, 120_000);

describe.skipIf(!installed)('the forms Visual FoxPro ships with', () => {
  it('opens every one of them', async () => {
    const api = createMemoryApi();
    seed(api, SOLUTION);
    // The tables, the class libraries and the second database the samples share sit beside the
    // Solution folder rather than in it, which is where they are named from: `..\..\data\`,
    // `..\..\classes\samples.vcx`, `..\..\northwind\`. A sample started from the launcher has
    // the whole Samples folder around it, so a run that seeded only Solution would be asking a
    // question the product is never asked.
    for (const beside of ['Data', 'Classes', 'Northwind']) seed(api, `${SAMPLES}/${beside}`, 0);
    // HOME() is where Visual FoxPro is installed and HOME(2) its samples folder, which several
    // of these forms join a file name onto - `USE ADDBS(HOME()) + 'Samples\Northwind\Orders'`.
    // The real file service works that out from the registry; the in-memory one has no registry
    // to read, so the harness answers as the product does. Without it those forms are asked to
    // find their data under a folder that was never the product's.
    api.project.homeDir = async (which: number, appDir: string) =>
      which === 1 ? `${appDir}\\` : which === 0 ? `${HOME}/` : which === 2 ? `${SAMPLES}/` : '';
    // and the libraries a control CLASSLOC reaches outside the samples entirely, plus the
    // headers in the root of the installation: `foxpro.h` is where the Foundation Classes' own
    // headers get the field types and the message box flags from, and the product finds it there
    for (const dir of [`${SAMPLES}/classes`, `${HOME}/ffc`, `${HOME}/gallery`, `${HOME}/wizards`, HOME]) seedLibraries(api, dir);
    // The native library and COM the application gives the renderer, which is where a sample
    // that calls GetSystemTime or hosts an ActiveX control gets its answer. Both report
    // themselves unavailable where they cannot work - off Windows, or in a build with no COM
    // addon - and the runtime turns that into the error a program can see, so this is the real
    // service everywhere and a stand-in nowhere.
    const dll = createDllService();
    api.dll = {
      async available() {
        return dll.available();
      },
      async call(request) {
        return dll.call(request);
      },
    };
    const ole = createOleService(['resources/native']);
    const call = (op: string, ...args: unknown[]): OleValue => {
      const reply = ole.perform(op, args);
      if (!reply.ok) throw new Error(reply.error);
      return reply.value;
    };
    api.ole = {
      available: () => call('available').flag === true,
      create: (progId) => call('create', progId).handle ?? 0,
      active: (name, className) => call('active', name, className).handle ?? 0,
      get: (handle, name, args) => call('get', handle, name, args),
      set: (handle, name, value) => void call('set', handle, name, value),
      call: (handle, name, args) => call('call', handle, name, args),
      release: (handle) => void call('release', handle),
      releaseAll: () => void call('releaseAll'),
    };
    setApi(api);
    useProjectStore.setState({ path: `${SOLUTION}/solution.fxproject`, doc: null });

    // the menus the samples ship, by the name a form names one under
    const menus = new Map(filesUnder(SOLUTION, '.mnx').map((path) => [baseName(path).toLowerCase(), path]));

    // and the forms themselves, for the ones that open each other
    const scx = filesUnder(SOLUTION, '.scx');
    const forms = new Map(scx.map((path) => [baseName(path).toLowerCase(), path]));

    const said: string[] = [];
    const only = process.env['RUN_ONLY'];
    for (const path of scx) {
      const where = path.slice(SAMPLES.length + 1);
      if (only !== undefined && !where.toLowerCase().includes(only.toLowerCase())) continue;
      const started = Date.now();
      useSessionStore.getState().cancel();
      useSessionStore.setState({ output: [] });
      try {
        const docs = await documentsOf(path);
        const first = [...docs.keys()][0];
        if (first === undefined) continue;
        const source = sourceOver(docs, path.slice(0, path.lastIndexOf('/')), forms, menus);
        // A form that parks in READ EVENTS or puts up a MESSAGEBOX is not hung: it is waiting,
        // and what it said before it started waiting is what this measures. Nothing here answers
        // a dialog, so the wait would never end - hence the deadline, after which the session is
        // cancelled and the next form gets a clean one.
        //
        // The name goes in as it is written, which is how anyone opens a form: `DO FORM 1_many`
        // is what the product itself answers to - measured - even though no identifier may
        // start with a digit, because what follows DO FORM is a file name and not a name.
        await Promise.race([useSessionStore.getState().execute(source, `DO FORM ${first} NOSHOW`), deadline(BUDGET_MS)]);
        for (let i = 0; i < 150 && useSessionStore.getState().status === 'running' && Date.now() - started < BUDGET_MS; i++) await settle();
      } catch (e) {
        said.push(`${where}: ${e instanceof Error ? e.message : String(e)}`);
        continue;
      }
      for (const line of complaints()) said.push(`${where}: ${line}`);
      if (process.env['RUN_TIMES']) process.stderr.write(`${Date.now() - started}ms ${where}\n`);
      // written as it goes, so a run that is cut short still says how far it got
      if (process.env['RUN_REPORT']) writeFileSync(process.env['RUN_REPORT'], said.join('\n'));
    }
    useSessionStore.getState().cancel();

    if (process.env['RUN_REPORT']) writeFileSync(process.env['RUN_REPORT'], said.join('\n'));
    const allowed = new Set(
      readFileSync(KNOWN, 'utf8')
        .split('\n')
        .map((l) => l.trim())
        .filter((l) => l !== '' && !l.startsWith('#')),
    );
    expect(said.filter((s) => !allowed.has(s))).toEqual([]);
  }, 900_000);
});
