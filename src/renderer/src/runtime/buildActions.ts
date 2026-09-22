/**
 * Build App and Build Executable: compiling the open project into a `.fxa` bundle, and
 * wrapping that bundle in a copy of the installed runtime so it can be handed to someone
 * who does not have FoxDev Studio.
 */

import { parseFormDocument } from '@shared/form/serialize';
import { parseMenuDocument } from '@shared/menu/serialize';
import { packBundle, stringifyBundle, type AppBundle } from '@shared/runtime/bundle';
import { formMethodSources } from '@shared/runtime/programSource';
import { createEmptyProjectDocument, parseProjectDocument, stringifyProjectDocument } from '@shared/project/serialize';
import { kindForPath, type ProjectDocument } from '@shared/project/schema';
import { basename, dirname, extname, relative, resolveFrom } from '@shared/paths';
import { getApi } from '../api/foxdev';
import { useProjectStore } from '../stores/projectStore';
import { useSessionStore } from './session';
import { compileForm, compileProgram, includedHeaders } from './vmBridge';
import { loadFoxVm } from '../../../wasm/foxvm/loader';

/** The path with a default extension when it was written without one. */
function withExtension(path: string, ext: string): string {
  return extname(path) ? path : path + ext;
}

/** A project to build: the one a command named with FROM, or the one that is open. */
export interface BuildSource {
  doc: ProjectDocument;
  /** Directory the project's own paths are relative to. */
  dir: string;
}

/**
 * The project a build works on. `from` is what `BUILD APP x FROM y` named, which need not be
 * the project the IDE has open; without one it is the open project, which is what the menu
 * command means.
 */
export async function buildSource(from?: string): Promise<BuildSource | null> {
  const print = useSessionStore.getState().print;
  if (from) {
    const path = withExtension(from, '.fxp');
    try {
      const parsed = parseProjectDocument(await getApi().files.readText(path));
      if (!parsed.ok) {
        print({ kind: 'error', text: `${path}: ${parsed.error}` });
        return null;
      }
      return { doc: parsed.doc, dir: dirname(path) };
    } catch (e) {
      print({ kind: 'error', text: `Cannot read project ${path}. ${e instanceof Error ? e.message : String(e)}` });
      return null;
    }
  }
  const project = useProjectStore.getState();
  if (!project.doc || !project.path) {
    print({ kind: 'error', text: 'Open a project first.' });
    return null;
  }
  return { doc: project.doc, dir: dirname(project.path) };
}

/** Compiles a project. Returns null and reports to Output when it cannot. */
export async function buildBundle(from?: string): Promise<AppBundle | null> {
  const print = useSessionStore.getState().print;
  const source = await buildSource(from);
  if (!source) return null;

  const vm = await loadFoxVm();
  try {
    const { bundle, log } = await packBundle(
      {
        project: source.doc,
        readItem: (rel) => getApi().files.readText(resolveFrom(source.dir, rel)),
        parseForm: parseFormDocument,
        parseMenu: parseMenuDocument,
      },
      {
        compileForm: (name, methods) => compileForm(name, methods),
        compileProgram: (source, name) => compileProgram(source, name),
        vmVersion: vm.version(),
      },
    );
    for (const line of log) print({ kind: 'output', text: line });
    return bundle;
  } catch (err) {
    print({ kind: 'error', text: `Build failed. ${err instanceof Error ? err.message : String(err)}` });
    return null;
  }
}

/**
 * Build App...: writes the compiled project as a `.fxa` bundle.
 *
 * `to` is where it goes when something already knows - a program that called the project's
 * Build method named the file. Without one the dialog asks, which is what the menu does.
 */
export async function buildApp(to?: string, from?: string): Promise<boolean> {
  const bundle = await buildBundle(from);
  if (!bundle) return false;
  const print = useSessionStore.getState().print;

  const path =
    to ??
    (await getApi().dialog.saveFile({
      title: 'Build Application',
      defaultPath: `${bundle.name}.fxa`,
      filters: [{ name: 'FoxDev Application', extensions: ['fxa'] }],
    }));
  if (!path) return false;

  await getApi().files.writeText(path, stringifyBundle(bundle));
  print({ kind: 'output', text: `Built ${path}` });
  return true;
}

/** Build Executable...: a folder with <Name>.exe and the bundle inside its resources. */
export async function buildExecutable(to?: string, from?: string): Promise<boolean> {
  const bundle = await buildBundle(from);
  if (!bundle) return false;
  const print = useSessionStore.getState().print;

  const outDir = to ?? (await getApi().dialog.pickFolder({ title: 'Choose where to build the application' }));
  if (!outDir) return false;

  const result = await getApi().build.exe({ name: bundle.name, outDir, bundle: stringifyBundle(bundle) });
  if (result.ok && result.exePath) {
    print({ kind: 'output', text: `Built ${result.exePath}` });
    return true;
  }
  print({ kind: 'error', text: `Build Executable failed. ${result.error ?? 'unknown error'}` });
  return false;
}

/**
 * `BUILD PROJECT file [FROM ...]`: the project itself, made out of the files it names.
 *
 * Without FROM it refreshes a project that is already there, which is what VFP does. Files that
 * are not on disk are reported and kept, because a project is allowed to name something that
 * has not been written yet.
 */
export async function buildProject(target: string, from: string[]): Promise<boolean> {
  const print = useSessionStore.getState().print;
  const path = withExtension(target, '.fxp');
  const dir = dirname(path);
  const files = getApi().files;

  let doc: ProjectDocument;
  if (await files.exists(path)) {
    const parsed = parseProjectDocument(await files.readText(path));
    if (!parsed.ok) {
      print({ kind: 'error', text: `${path}: ${parsed.error}` });
      return false;
    }
    doc = parsed.doc;
  } else {
    doc = createEmptyProjectDocument(basename(path, true));
  }

  const items = [...doc.items];
  for (const one of from) {
    const rel = relative(dir, resolveFrom(dir, one)).replace(/\\/g, '/');
    if (!items.some((i) => i.path.toLowerCase() === rel.toLowerCase())) {
      items.push({ kind: kindForPath(rel), path: rel });
    }
    if (!(await files.exists(resolveFrom(dir, rel)))) {
      print({ kind: 'error', text: `${rel}: unresolved reference; the project names it anyway.` });
    }
  }
  // "the first executable program or menu file in the FROM clause is the master program file"
  const main = doc.main ?? items.find((i) => i.kind === 'program' || i.kind === 'menu' || i.kind === 'form')?.path;
  const built: ProjectDocument = { ...doc, items, ...(main ? { main } : {}) };

  await files.writeText(path, stringifyProjectDocument(built));
  print({ kind: 'output', text: `Built ${path}` });
  return true;
}

/** What COMPILE reads when it is given a file name or a skeleton like `*.prg`. */
async function filesMatching(skeleton: string, fallbackExtension: string): Promise<string[]> {
  const files = getApi().files;
  if (!/[*?]/.test(skeleton)) return [withExtension(skeleton, fallbackExtension)];
  const dir = dirname(skeleton) || '.';
  const pattern = new RegExp(
    '^' + basename(skeleton).replace(/[.+^${}()|[\]\\]/g, '\\$&').replace(/\*/g, '.*').replace(/\?/g, '.') + '$',
    'i',
  );
  const names = await files.listDir(dir);
  return names.filter((n) => pattern.test(n)).map((n) => resolveFrom(dir, n));
}

/**
 * `COMPILE`: reads each source file, compiles it, and reports what it could not compile.
 *
 * The object code itself is not written beside the source: this runtime compiles what it runs
 * as it loads it, so what COMPILE is for here is the check - the errors a file would raise
 * before anyone runs it - which is also what makes it useful over a whole directory.
 */
export async function compileFiles(what: string, skeleton: string): Promise<boolean> {
  const print = useSessionStore.getState().print;
  const files = getApi().files;
  const extension = what === 'FORM' ? '.fxf' : what === 'DATABASE' ? '.dbc' : what === 'REPORT' ? '.frx' : what === 'LABEL' ? '.lbx' : what === 'CLASSLIB' ? '.fxc' : '.prg';
  const targets = await filesMatching(skeleton, extension);
  if (targets.length === 0) {
    print({ kind: 'error', text: `COMPILE: no file matches ${skeleton}.` });
    return false;
  }

  await loadFoxVm();
  let failed = 0;
  for (const file of targets) {
    let source: string;
    try {
      source = await files.readText(file);
    } catch (e) {
      print({ kind: 'error', text: `${file}: ${e instanceof Error ? e.message : String(e)}` });
      failed++;
      continue;
    }
    const name = basename(file, true);
    // a form, a class library and a menu keep their code in methods; a program is one text
    const output =
      what === 'FORM'
        ? compileFormFile(name, source, print)
        : compileProgram(source, name, await headersNextTo(source, dirname(file)));
    if (!output) {
      failed++;
      continue;
    }
    for (const d of output.diagnostics) {
      if (d.severity === 'error') failed++;
      print({ kind: d.severity === 'error' ? 'error' : 'output', text: `${basename(file)} (${d.line}): ${d.message}` });
    }
    for (const m of output.methodDiagnostics ?? []) {
      failed++;
      print({ kind: 'error', text: `${basename(file)} ${m.method} (${m.diagnostic.line}): ${m.diagnostic.message}` });
    }
  }
  print({ kind: 'output', text: `Compiled ${targets.length} file${targets.length === 1 ? '' : 's'}${failed ? `, ${failed} error${failed === 1 ? '' : 's'}` : ''}.` });
  return failed === 0;
}

/** The text of each header a program includes, looked for beside the program itself. */
async function headersNextTo(source: string, dir: string): Promise<Record<string, string>> {
  const out: Record<string, string> = {};
  for (const stem of includedHeaders(source)) {
    if (stem in out) continue;
    for (const ext of ['.h', '.prg']) {
      const path = resolveFrom(dir, stem + ext);
      if (await getApi().files.exists(path)) {
        out[stem] = await getApi().files.readText(path).catch(() => '');
        break;
      }
    }
  }
  return out;
}

/** One form's methods, compiled the way the runtime compiles them when it opens the form. */
function compileFormFile(name: string, source: string, print: (line: { kind: 'output' | 'error'; text: string }) => void) {
  const parsed = parseFormDocument(source);
  if (!parsed.ok) {
    print({ kind: 'error', text: `${name}: ${parsed.error}` });
    return null;
  }
  return compileForm(name, formMethodSources(parsed.doc));
}
