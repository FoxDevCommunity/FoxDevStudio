/**
 * A run of the user's application: one VM, one desktop of live forms, one scheduler.
 *
 * This is where yielded host requests become real behaviour - dialogs, file access, form
 * creation - and where the IDE reads run state for the status bar and Output panel. One
 * session exists at a time, matching VFP's single runtime.
 */

import { create } from 'zustand';
import { basename, dirname, isAbsolute, join, resolveFrom } from '@shared/paths';
import { Desktop, FormInstance, FormSetInstance, type OpenedWindow, type RuntimeObject } from '@shared/runtime/objectModel';
import { Scheduler, type ErrorAction, type SchedulerState } from '@shared/runtime/scheduler';
import { buildApp, buildExecutable, buildProject, compileFiles } from './buildActions';
import type { FiberContext } from '@shared/runtime/host';
import { HostError, type BrowseTable, type CompileOutput, type GetField, type HostRequest, type RuntimeError, type ScreenDoc, type StackEntry } from '@shared/runtime/host';
import { DataObject, baseClassNames, registryObject } from '@shared/runtime/dataEnvironment';
import { XmlAdapter } from '@shared/runtime/xmlAdapter';
import type { HostObject } from '@shared/runtime/oleObjects';
import { baseName, CompileFailure, formHeaderRefs, formMethodSources, requireBytes, type CompiledForm, type ProgramSource } from '@shared/runtime/programSource';
import type { MenuDocument, MenuItem } from '@shared/menu/schema';
import type { FormCursor, FormDocument, FormNode, FormRelation } from '@shared/form/schema';
import { formsetDocumentNames } from '@shared/form/formset';
import { argValue, displayValue, type VmValue } from '@shared/runtime/values';
import { buildClassInstance } from '@shared/runtime/classInstance';
import { isNonVisualBaseClass, resolveInheritance, type VfpClassDef } from '@shared/runtime/classDef';
import { classAsNode } from '@shared/classlib/schema';
import { ClassLibraries } from './classLibraries';
import { readVfpTable } from '../vfp/openVfpFile';
import { bundledClassLibraryDir } from '../vfp/classLibraries';
import { getApi } from '../api/foxdev';
import type { LowLevelValue } from '@shared/ipc/api';
import { useProjectStore } from '../stores/projectStore';
import { compileForm, compileSnippet, createVm, type WasmVm } from './vmBridge';
import { readHeaderFiles } from './headerFiles';
import { reasonLabel, useDebugStore } from './debugSession';
import {
  UnsupportedComObject,
  activeComObject,
  createComObject,
  notSupported,
  releaseComObjects,
  releaseLibraries,
} from './comObject';
import { closeSqlConnections, performSql } from './sqlPassThrough';
import { answerRequest } from './httpServer';
import { ProjectObject } from './projectObject';
import { loadFoxVm } from '../../../wasm/foxvm/loader';

export interface OutputLine {
  kind: 'output' | 'error' | 'trace' | 'echo';
  text: string;
}

/** A modal question the desktop must show; resolved by the dialog component. */
export interface PendingDialog {
  id: number;
  kind: 'message' | 'input' | 'color' | 'font' | 'key';
  title: string;
  text: string;
  /** MESSAGEBOX flags (button set + icon). */
  flags: number;
  /** Starting value: INPUTBOX text, a colour as an RGB integer, or `name,size,style`. */
  defaultText: string;
  resolve(value: VmValue): void;
}

/** A `READ`, or a `MENU TO`, waiting on the user. */
export interface PendingRead {
  id: number;
  /** The fields to let the user into, or the choices to pick one of. */
  fields: GetField[];
  prompts: string[];
  resolve(value: VmValue): void;
}

export interface WaitMessage {
  id: number;
  text: string;
}

export interface RuntimeErrorReport {
  error: RuntimeError;
  stack: StackEntry[];
  resolve(action: ErrorAction): void;
}

export interface SessionState {
  status: SchedulerState | 'error';
  desktop: Desktop | null;
  scheduler: Scheduler | null;
  vm: WasmVm | null;
  /** What is running, for the status bar. */
  target: string | null;
  output: OutputLine[];
  dialog: PendingDialog | null;
  wait: WaitMessage | null;
  errorReport: RuntimeErrorReport | null;
  /** Command Window history, newest last. */
  history: string[];
  traceEvents: boolean;
  /** Bumped whenever the desktop's form list changes, so React re-renders. */
  revision: number;
  /** The menu a running program installed with `DO x.fxm`, if any. */
  menu: MenuDocument | null;
  /** A shortcut menu on screen where the pointer is, waiting to be chosen from or dismissed. */
  shortcutMenu: { id: number; menu: MenuDocument; resolve(item: MenuItem | null): void } | null;
  /** Item id -> disabled, from each item's SKIP FOR expression. */
  menuSkip: Record<string, boolean>;
  /** The character screen, once a program has drawn on it. */
  screen: ScreenDoc | null;
  /** The Browse windows a program has put up, oldest first. */
  browses: BrowseTable[];
  /** Closes one of them; a BROWSE that was waiting on it carries on. */
  closeBrowse(alias: string): void;
  /** The memo editing windows MODIFY MEMO opened. */
  memos: OpenMemo[];
  /** What the window holds now, as it is typed. */
  setMemoText(alias: string, field: string, text: string): void;
  /** Closes one window and gives what it holds back to whatever is waiting for it. */
  closeMemo(alias: string, field: string): void;
  /** A READ or a MENU TO the user has not answered yet. */
  read: PendingRead | null;
  /** Where the pointer last was over the character screen, for MROW()/MCOL()/MDOWN(). */
  pointer: { row: number; col: number; down: boolean };
  /** Records where the pointer is, so the mouse functions can answer. */
  setPointer(at: { row: number; col: number; down: boolean }): void;
  /** Lets go of every table the run opened; set while a session is alive. */
  releaseTables: (() => void) | null;

  runForm(source: ProgramSource, name: string): Promise<void>;
  runProgram(source: ProgramSource, name: string): Promise<void>;
  /** Compiles and runs one line, as the Command Window and menu items do. */
  execute(source: ProgramSource, text: string): Promise<void>;
  cancel(): void;
  /** Runs a menu item's command or procedure text. */
  chooseMenuItem(item: MenuItem): Promise<void>;
  /** Re-evaluates every SKIP FOR expression; called when a menu opens. */
  refreshMenuSkip(): Promise<void>;
  print(line: OutputLine): void;
  clearOutput(): void;
  setTraceEvents(on: boolean): void;
}

let dialogSeq = 0;

/**
 * Everything one run put on the desktop, cleared.
 *
 * A run that fails still ends a run: what the one before it drew has to go, or the screen and
 * the browse windows of a program that is no longer running are left standing in front of the
 * error that stopped this one.
 */
function runCleared(): Pick<SessionState, 'screen' | 'browses' | 'memos' | 'read' | 'pointer'> {
  memoWaiters.clear();
  return { screen: null, browses: [], memos: [], read: null, pointer: { row: 0, col: 0, down: false } };
}

/** What to call when a Browse window a command is waiting on is closed, by its alias. */
const browseWaiters = new Map<string, () => void>();

/** `FoxScript.Http` servers this run has opened, so cancelling the run closes every one. */
const httpServers = new Set<number>();
/** What stops the main process handing requests over; installed with the first server. */
let httpListening: (() => void) | null = null;

/** Closes every server the run opened. A port outliving its program would be a leak. */
function releaseHttpServers(): void {
  httpListening?.();
  httpListening = null;
  for (const server of httpServers) void getApi().http.close(server).catch(() => false);
  httpServers.clear();
}

/** One memo field's editing window. */
export interface OpenMemo {
  alias: string;
  field: string;
  text: string;
  /** The window shows the text and does not take changes to it. */
  noedit: boolean;
}

/** What to call when a MODIFY MEMO window the command is waiting on is closed, by its name. */
const memoWaiters = new Map<string, (text: string) => void>();

/** The key both maps are kept under: a field of a table, as the command wrote it. */
function memoKey(alias: string, field: string): string {
  return `${alias.toUpperCase()}.${field.toUpperCase()}`;
}

/**
 * How the runtime asks the shell to reveal the desktop. The IDE points this at its document
 * store; the runtime player leaves it alone because its desktop is always on screen.
 */
export const runtimeUi = {
  showDesktop: () => {},
  /** Brings the debugger window up when a program stops. The player has no such window. */
  showDebugger: () => {},
  /** Opens a file for editing, for MODIFY and BROWSE. The player has nowhere to open one. */
  openDocument: async (_path: string): Promise<void> => {},
  /**
   * Brings the source a stopped program is in to the front, named as its frames name it: a
   * program's own name, or `objPath.Event` for a method. Stepping into a routine whose file
   * is not open would otherwise mark a line nobody can see.
   */
  showSource: (_program: string): void => {},
  /** Runs a file of the project: a program, or a form. */
  runFile: async (path: string): Promise<void> => {
    throw new Error(`${path} cannot be run from here`);
  },
  /** CREATE FORM and the rest: something new of that kind, in its designer. */
  newDocument: async (kind: string, _path: string): Promise<void> => {
    throw new Error(`CREATE ${kind.toUpperCase()} needs a designer, which the player has not`);
  },
  /**
   * The development environment rather than a built application: `VERSION(2)` answers 2 here
   * and 0 in the player, and programs branch on it to find their source.
   */
  development: false,
};
const showDesktop = () => runtimeUi.showDesktop();

/**
 * `MOUSE`: the pointer put where the command said, and pressed there.
 *
 * Visual FoxPro measures the position in rows and columns of the main window's font unless the
 * command says PIXELS, so the character screen is what a row and a column mean here. Whatever
 * is drawn at that point is sent the events a hand would have sent it, and the position is kept
 * so MROW() and MCOL() answer with it afterwards.
 */
function mousePress(request: Extract<HostRequest, { kind: 'MousePress' }>): null {
  const words = request.style.toUpperCase().split(/\s+/);
  const screen = document.querySelector('[data-testid="character-screen"]');
  const rect = screen?.getBoundingClientRect();
  const doc = useSessionStore.getState().screen;
  const cell = {
    height: rect && doc ? rect.height / doc.rows : 0,
    width: rect && doc ? rect.width / doc.cols : 0,
  };
  const button = words.includes('RIGHT') ? 2 : words.includes('MIDDLE') ? 1 : 0;
  const modifiers = {
    shiftKey: words.includes('SHIFT'),
    ctrlKey: words.includes('CONTROL'),
    altKey: words.includes('ALT'),
  };
  const pixels = words.includes('PIXELS');

  const at = (row: number, col: number): { x: number; y: number } => {
    if (!rect) return { x: 0, y: 0 };
    if (pixels) return { x: rect.left + col, y: rect.top + row };
    return { x: rect.left + (col + 0.5) * cell.width, y: rect.top + (row + 0.5) * cell.height };
  };
  const send = (type: string, row: number, col: number, buttons: number): void => {
    if (!rect) return;
    const { x, y } = at(row, col);
    const target = document.elementFromPoint(x, y) ?? screen;
    target?.dispatchEvent(
      new MouseEvent(type, { clientX: x, clientY: y, button, buttons, bubbles: true, ...modifiers }),
    );
  };
  const cellOf = (row: number, col: number): { row: number; col: number } =>
    pixels && rect && cell.height > 0 && cell.width > 0
      ? { row: Math.floor(row / cell.height), col: Math.floor(col / cell.width) }
      : { row, col };

  const held = button === 2 ? 2 : button === 1 ? 4 : 1;
  const start = request.at ?? [useSessionStore.getState().pointer.row, useSessionStore.getState().pointer.col];
  if (request.at) send('mousemove', start[0], start[1], 0);
  for (let press = 0; press < request.clicks; press += 1) {
    send('mousedown', start[0], start[1], held);
    send('mouseup', start[0], start[1], 0);
    send('click', start[0], start[1], 0);
  }
  if (request.clicks === 2) send('dblclick', start[0], start[1], 0);
  // a drag holds the button down from where it started until the last position it reaches
  let last = start;
  if (request.drag.length > 0) {
    send('mousedown', start[0], start[1], held);
    for (const point of request.drag) {
      send('mousemove', point[0], point[1], held);
      last = point;
    }
    send('mouseup', last[0], last[1], 0);
  }
  useSessionStore.getState().setPointer({ ...cellOf(last[0], last[1]), down: false });
  return null;
}

/**
 * What the `A...()` functions are asking the host for.
 *
 * Each answer is rows, and a row of one value is a list of one column. What this runtime has
 * nothing of - a designer selection while a program runs, an object under the pointer, a
 * network to enumerate - answers with nothing, which is what those functions answer when there
 * is nothing to find.
 */
function enumerate(desktop: Desktop, what: number, name: string): VmValue {
  const rows = (items: VmValue[][]): VmValue => ({ $arr: items.map((row) => ({ $arr: row, $cols: 0 })), $cols: 0 });
  switch (what) {
    // the objects alive under a class name
    case 0:
      return rows(desktop.instancesOf(name).map((o) => [{ $obj: o.handle }, o.name]));
    // what BINDEVENT has bound
    case 1: {
      const source = Number(name);
      const bound = desktop.boundEvents(Number.isFinite(source) && source > 1 ? source : undefined);
      return rows(bound.map((b) => [{ $obj: b.source }, b.event, { $obj: b.handler }, b.delegate]));
    }
    // the base classes a program can ask CREATEOBJECT() for
    case 8:
      return rows(baseClassNames().map((n) => [n]));
    default:
      return null;
  }
}

/** A low-level answer as a VM value: a list becomes an array, a date stays a date. */
function lowLevelToVm(value: LowLevelValue): VmValue {
  if (Array.isArray(value)) return { $arr: value.map(lowLevelToVm), $cols: 0 };
  return value as VmValue;
}

export const useSessionStore = create<SessionState>((set, get) => {
  const print = (line: OutputLine) => set((s) => ({ output: [...s.output.slice(-499), line] }));

  /** Builds a fresh VM + desktop + scheduler, discarding any previous run. */
  const startSession = async (source: ProgramSource, target: string) => {
    get().cancel();
    await loadFoxVm();

    const desktop = new Desktop();
    desktop.mouse = () => get().pointer;
    // _VFP.ActiveProject is the project the IDE has open, which a builder reads and acts on
    desktop.activeProject = () => (useProjectStore.getState().doc ? new ProjectObject() : null);
    // what only the VM can do, for the methods of the application object that need it
    desktop.evaluate = (expression) => runExpression(expression);
    // a property the form works out for itself: what it comes to, or nothing, and either way
    // not an error in the program that opened the form
    desktop.evaluateQuietly = (expression) => runExpression(expression, true);
    desktop.runLine = (text) => runSnippet(text, 'line').then(() => undefined);
    desktop.setVariable = (name, value) => vm.setGlobal(name, value);
    desktop.quitRequested = () => void get().cancel();
    // SET LIBRARY TO reaches the process that can load a 32-bit .fll; there is none in a browser.
    // A library named without a folder is the program's default directory's, which here is the
    // project folder - `SET LIBRARY TO codemine.fll` means the one beside the application. Left
    // alone, that process would resolve it against its own working directory instead.
    const libraryHost = getApi().library;
    desktop.libraries = libraryHost && {
      ...libraryHost,
      load: (path: string) => libraryHost.load(isAbsolute(path) ? path : useProjectStore.getState().resolvePath(path)),
    };
    const vm = createVm(desktop);
    vm.setSetting('RUNTIME', !runtimeUi.development);
    // _SAMPLES names the samples directory, which is HOME(2). A program that opens
    // _samples + "\Data\customer.dbf" reads it on its first line, so it is answered before
    // anything runs rather than while it does.
    const samples = await getApi()
      .project.homeDir(2, useProjectStore.getState().dir() ?? '')
      .catch(() => '');
    vm.setGlobal('_SAMPLES', samples);
    // where the Foundation Classes we ship live, for a program that names one by file alone
    const ffcDir = await bundledClassLibraryDir();
    const modules = new Map<string, number>();
    /** Where records sit in each open table, so a record number can become a byte offset. */
    const layouts = new Map<number, { headerLen: number; recordLen: number }>();
    /**
     * A path as the program wrote it, made absolute.
     *
     * Visual FoxPro resolves a relative path against the default directory; here that is the
     * project folder, which is also the only place the main process will let a program read.
     * Without this `FILE("solution.scx")` is refused rather than answered.
     */
    const at = (path: string) => (path.trim() === '' ? path : useProjectStore.getState().resolvePath(path.trim()));
    /**
     * Asks for access to a file outside the project folder before touching it.
     *
     * The main process only lets the renderer at directories the user chose, and a Visual FoxPro
     * project reaches sideways out of its own: the Solution sample's forms open tables in the
     * `data` folder beside it. Asking is what the importer already does; the
     * running program has the same need, and without it a table two folders away is reported as
     * missing rather than refused.
     */
    const reach = async (path: string): Promise<string> => {
      try {
        await getApi().project.allowNear(path);
      } catch {
        // no host to ask (the in-memory API): the operation itself will say what happened
      }
      return path;
    };
    /**
     * Where a file the program named really is, given the other folders `SET PATH TO` named.
     *
     * Measured in Visual FoxPro 9: the default directory is tried first and each path entry
     * after it, in the order it was written, and the first one that has the file wins. A name
     * nothing answers to comes back as it was asked for, so what is reported missing is the
     * place the program meant rather than the last place that was tried.
     */
    const locate = async (path: string, search: readonly string[]): Promise<string> => {
      const first = await reach(path);
      if (search.length === 0) return first;
      if (await getApi().files.exists(first).catch(() => false)) return first;
      for (const candidate of search) {
        const where = await reach(candidate);
        if (await getApi().files.exists(where).catch(() => false)) return where;
      }
      return first;
    };
    const releaseTables = () => {
      for (const handle of layouts.keys()) void getApi().data.close(handle);
      layouts.clear();
    };

    /**
     * The class libraries the run has loaded, and the modules their classes were compiled into.
     *
     * A class out of a library is a document like a form: its methods are compiled once, under
     * the class's own name, and every object of it runs that module. The key holds the file as
     * well as the class, because two libraries may each have a class of the same name.
     */
    /** The folders the project's class libraries are in, each once, in the order the project lists them. */
    function projectClassFolders(): string[] {
      const project = useProjectStore.getState();
      const folders: string[] = [];
      for (const item of project.doc?.items ?? []) {
        if (item.kind !== 'class') continue;
        const folder = dirname(project.resolvePath(item.path));
        if (!folders.some((f) => f.toLowerCase() === folder.toLowerCase())) folders.push(folder);
      }
      return folders;
    }

    const classLibraries = new ClassLibraries(async (given) => {
      const path = await reach(at(given));
      if (await getApi().files.exists(path).catch(() => false)) return readVfpTable(path);
      // A class library of the project is found by name wherever in the project it sits, as it
      // is once the project is built and every file is inside the application: `SET CLASSLIB TO
      // AppMain` from a program in `source\` names `source\appmain.vcx`. The project lists the
      // library as what the import made of it; the .vcx it was made from is beside that.
      const stemOf = (p: string) => basename(p).replace(/\.[^.]*$/, '').toLowerCase();
      const wanted = stemOf(given);
      const project = useProjectStore.getState();
      const item = project.doc?.items.find((i) => i.kind === 'class' && stemOf(i.path) === wanted);
      if (item) {
        const vcx = project.resolvePath(item.path).replace(/\.[^./\\]+$/, '.vcx');
        if (await getApi().files.exists(vcx).catch(() => false)) return readVfpTable(await reach(vcx));
      }
      // A Foundation Class is named by file alone - `SET CLASSLIB TO _base` - because Visual
      // FoxPro finds those on its own search path. Copies of them ship with FoxDev for exactly
      // that reason, and the folder holding them is the last place looked, as it is when a
      // project is imported.
      const bundled = ffcDir === '' ? '' : join(ffcDir, basename(path));
      if (bundled !== '' && (await getApi().files.exists(bundled).catch(() => false))) return readVfpTable(bundled);
      // the product reports a class library it cannot find as a missing file, not as a class
      // that does not exist - measured, error 1 with the name it was given
      throw new HostError(1, `File '${given}' does not exist.`);
    },
      // A class names the library it stands on by a path from wherever the developer's copy sat,
      // and that seldom leads anywhere now: `..\..\common50\cmapp.vcx` from `source\`. The
      // project's own class libraries say where those files really are, as they did when the
      // project was imported, so their folders are looked in before the Foundation Classes.
      [...projectClassFolders(), ...(ffcDir === '' ? [] : [ffcDir])],
      (library, from) => print({ kind: 'error', text: `${basename(from)}: class library "${library}" was not found; its classes are missing what they inherit from it` }),
    );
    const classModules = new Map<string, number>();

    const scheduler = new Scheduler(vm, { perform: (request, ctx) => perform(request, ctx) }, {
      onError: (error, stack) =>
        // a quiet run is one whose caller wants the answer or nothing: it carries on at the next
        // statement, and nothing is said to anyone
        quietRuns > 0
          ? // a promise, not the bare answer: the scheduler's synchronous path resumes the fiber
            // from inside its own loop, and a quiet run is the first caller that would take it
            Promise.resolve<ErrorAction>('ignore')
          : new Promise<ErrorAction>((resolve) => {
              set({ status: 'error', errorReport: { error, stack, resolve: (action) => { set({ errorReport: null }); resolve(action); } } });
              print({ kind: 'error', text: `Error ${error.code} in ${error.program} line ${error.line}: ${error.message}` });
            }),
      onStateChange: (status) => set({ status }),
      onQuit: () => get().cancel(),
      // a program stopped at a breakpoint is a parked fiber, so the IDE carries on around it
      onBreak: (stop) => {
        useDebugStore.getState().stopped(stop);
        runtimeUi.showDebugger();
        // and the source it stopped in, so the line the debugger marks is one that is on screen
        runtimeUi.showSource(stop.program);
        print({ kind: 'trace', text: `${reasonLabel(stop)} in ${stop.program}, line ${stop.line}` });
      },
      onResumed: () => useDebugStore.getState().resumed(),
    });
    useDebugStore.getState().attach(vm, scheduler);

    /**
     * A method of an object of a program's class: the class's own code, or the nearest class
     * above it that has some. Each class's methods are compiled under the class's own name, so
     * one that a class inherits without overriding is found under its parent's - measured, an
     * object of `cc AS cb` runs cb's Greet, and cb's object runs ca's Who.
     */
    function dispatchUpClasses(module: number, className: string, obj: RuntimeObject, event: string, args: VmValue[]) {
      const classes = definedClasses(module);
      const seen = new Set<string>();
      let name = className;
      while (name !== '' && !seen.has(name.toLowerCase())) {
        seen.add(name.toLowerCase());
        const outcome = scheduler.dispatchClass(module, name, obj.path(), event, obj.handle, args);
        if (outcome !== null) return outcome;
        name = classes.find((c) => c.name.toLowerCase() === name.toLowerCase())?.baseClass ?? '';
      }
      return null;
    }

    desktop.dispatch = (obj, event, args) => {
      // a control's code lives in its form's module; a formset's own lives in the formset's,
      // because each form of a formset is a document, and so is compiled, of its own
      const owner = obj.codeOwner();
      if (!owner || owner.module < 0 || !obj.alive) return null;
      if (get().traceEvents) print({ kind: 'trace', text: `${owner.name}.${obj.path() || owner.name}.${event}()` });

      // A module compiled from `DEFINE CLASS` holds every class of the program, so its methods
      // are keyed by class name; a module compiled from one document - a form, or one class out
      // of a library - keys them by the path from the document's own root. Which of the two a
      // module is, is a question about the module, not about whether the object has a class.
      const own = owner.className && definedClasses(owner.module).length > 0
        ? dispatchUpClasses(owner.module, owner.className, obj, event, args ?? [])
        : scheduler.dispatch(owner.module, obj.path(), event, obj.handle, args ?? []);

      const bound = desktop.boundHandlers(obj.handle, event);
      if (bound.length === 0) return own;

      // BINDEVENT: the delegate runs after the object's own method, as VFP's default does
      return (async () => {
        const outcome = own instanceof Promise ? await own : own;
        for (const binding of bound) {
          const handler = desktop.object(binding.handler);
          if (handler?.alive) await runDelegate(handler, binding.delegate, args ?? []);
        }
        return outcome ?? { value: null, nodefault: false };
      })();
    };

    /** Runs a bound handler's method, whether it came from a class or a designed form. */
    async function runDelegate(handler: RuntimeObject, delegate: string, args: VmValue[]): Promise<void> {
      const outcome = desktop.dispatch(handler, delegate, args);
      if (outcome instanceof Promise) await outcome;
    }

    // `oContainer.NewObject(name, class, file)`: a member of a class out of a class library
    desktop.libraryObject = (className, module, into) => libraryObject(className, module, into);
    // _SCREEN.AddObject makes its object the way CREATEOBJECT does, a program's own classes
    // included, so it goes the same way a CREATEOBJECT from the VM goes
    desktop.createNamedObject = (className) =>
      Promise.resolve(
        perform({ kind: 'CreateObject', class: className, args: [], definition: programClass(className) }, { fiber: 0, generation: 0 }),
      );

    // an ActiveX control this runtime does not draw is still reachable through COM
    desktop.createOleObject = (progId) => {
      if (progId.trim() === '') return null;
      // A class this runtime refuses on purpose says so on every member rather than going
      // quiet: the control is on the form, the form opens, and only what asks for the control
      // hears the reason. A form designed against a server file names its class `(mci32.ocx)`,
      // which is a name like any other as far as the refusal list is concerned.
      const refused = notSupported(progId);
      if (refused !== undefined) return new UnsupportedComObject(progId, refused);
      // anything else named by its file rather than by a ProgID has no name to ask COM for
      if (progId.startsWith('(')) return null;
      try {
        return createComObject(progId, desktop);
      } catch {
        // no COM here, or the control is not registered: the member error already says so
        return null;
      }
    };

    // the object model notifies on form open/close; mirror that into React
    desktop.subscribe(() => set((s) => ({ revision: s.revision + 1 })));

    // An error the program handles never reaches the error dialog, and the samples all handle
    // their own: without this line the only sign of one is whatever its handler prints, which
    // says what went wrong but never where.
    desktop.onErrorRaised = (error, handled) => {
      // a value the form was only trying to work out is not news, however it went
      if (!handled || quietRuns > 0) return;
      print({ kind: 'error', text: `Error ${error.code} in ${error.program} line ${error.line}: ${error.message} (handled by the program)` });
    };

    // `?` writes to the session's Output panel; `??` continues the previous line
    desktop.output = (text, newline) =>
      set((s) => {
        const out = [...s.output];
        const last = out[out.length - 1];
        if (!newline && last?.kind === 'output') out[out.length - 1] = { kind: 'output', text: last.text + text };
        else out.push({ kind: 'output', text });
        return { output: out.slice(-500) };
      });

    /** Loads (compiling if needed) a form and runs its creation lifecycle. */
    const openForm = async (
      name: string,
      args: VmValue[],
      modal: boolean | null,
      noshow: boolean,
      /** The fiber of the program that asked for the form, whose scope its expressions read. */
      caller?: number,
    ): Promise<OpenedWindow | null> => {
      const asked = baseName(name);
      const compiled = await source.getForm(asked);
      if (!compiled) throw new HostError(1, `Form '${name}' does not exist.`);
      // a file that held a formset opens as the formset, whichever of its forms was named
      const formset = compiled.doc.meta?.vfp?.formset;
      if (formset) return openFormSet(formsetDocumentNames(asked, compiled.doc, formset), formset, args, noshow, caller);
      return buildForm(compiled, args, modal, noshow, caller);
    };

    /**
     * Opens a formset: the container, then every form it holds, then the container's own Init.
     *
     * The order is a container's order everywhere in Visual FoxPro - a member is finished before
     * whatever holds it is - and it matters here because a formset's Init is the first place that
     * can reach all of its forms at once. Nothing is shown while they are being built; Show at the
     * end puts up the ones the file did not mark invisible, which is what `DO FORM` does.
     */
    async function openFormSet(
      documents: string[],
      formset: NonNullable<NonNullable<FormDocument['meta']>['vfp']>['formset'] & object,
      args: VmValue[],
      noshow: boolean,
      caller?: number,
    ): Promise<FormSetInstance | null> {
      showDesktop();
      const node: FormNode = { name: formset.name, props: { ...formset.props }, methods: { ...formset.methods }, children: [] };
      const set = desktop.instantiateFormSet(node, moduleFor(formset.name, compileForm(formset.name, formMethodSources({ $schema: 'foxdev-form', version: 1, form: node }))));
      for (const document of documents) {
        const member = await source.getForm(document);
        if (!member) {
          print({ kind: 'error', text: `${formset.name}: form '${document}' was not found` });
          continue;
        }
        // the member is built and initialised inside the formset, and shown only by the formset
        const form = await buildForm(member, [], false, true, caller, set);
        if (!form) continue;
      }
      const outcome = desktop.dispatch(set, 'Init', args);
      const init = (outcome instanceof Promise ? await outcome : outcome)?.value;
      if (init === false) {
        await desktop.releaseFormSet(set);
        return null;
      }
      if (!noshow) desktop.showFormSet(set);
      return set;
    }

    /** Loads a compiled form document into the desktop and runs its creation lifecycle. */
    async function buildForm(
      compiled: CompiledForm,
      args: VmValue[],
      modal: boolean | null,
      noshow: boolean,
      caller?: number,
      formset?: FormSetInstance,
    ): Promise<FormInstance | null> {
      // a form has to be somewhere: reveal the desktop, however the DO FORM was reached
      showDesktop();
      let module = modules.get(compiled.name.toLowerCase());
      if (module === undefined) {
        module = vm.loadModule(compiled.bytes);
        modules.set(compiled.name.toLowerCase(), module);
      }
      const instance = desktop.instantiate(compiled.doc.form, module, {
        modal: modal ?? undefined,
        noshow,
        args,
        cursors: compiled.doc.data,
        dataEnvironment: true,
        // ^oWindows[1,0] among the form's own members: an array property, dimensioned before
        // anything can read it
        arrays: compiled.doc.meta?.vfp?.arrays,
      });
      // Where the form was read from, which is what `SYS(1271, THISFORM)` answers. Every sample
      // Visual FoxPro ships asks that question in its first Init and does `SET DEFAULT TO` the
      // folder it names, which is how a form written in one folder finds the class library and
      // the tables named beside it while the program runs somewhere else.
      const from = compiled.doc.meta?.vfp?.source;
      if (from) instance.file = at(from);
      // a member's Parent is the formset from the start, so its Init can already say THISFORMSET
      if (formset) desktop.addToFormSet(formset, instance);
      // a form's tables are open before Load runs, which is why so much form code assumes one is
      await openDataEnvironment(compiled.doc.data, compiled.doc.relations, compiled.name, compiled.doc.meta?.vfp?.source);
      const created = await desktop.runFormLifecycle(instance, {
        modal: modal ?? undefined,
        noshow,
        args,
        // Picture = (HOME() + "graphics\x.bmp") and its like are worked out now, as VFP does
        expressions: compiled.doc.meta?.vfp?.expressions,
        // and worked out where the program that opened the form stands, because that is the
        // only place its own variables are; a fresh program of ours would not see them
        inFrame: caller === undefined ? undefined : (expression) => vm.evaluateIn(caller, -1, expression),
      });
      return created ? instance : null;
    }

    /**
     * An object of a class that lives in a class library.
     *
     * The class is looked for in the file the call named, or in the libraries `SET CLASSLIB`
     * has loaded. What comes back is a class definition shaped exactly like a form - the
     * importer already reads a `.vcx` that way, parent classes folded in - so it is compiled
     * and brought to life the way a form is, and the object that results is an object of this
     * runtime like any other: its properties, its methods, its members, its Init.
     *
     * `undefined` says no library has that class; `null` says one has and its Init refused.
     */
    async function libraryObject(
      className: string,
      module: string,
      into?: { parent: RuntimeObject; name: string },
    ): Promise<RuntimeObject | null | undefined> {
      const found = await classLibraries.find(className, module);
      if (!found) return undefined;
      const { library, definition } = found;
      const key = `${library.path.toLowerCase()}:${definition.name.toLowerCase()}`;
      let compiled = classModules.get(key);
      if (compiled === undefined) {
        // the class's own meta comes with it: the header file its library named, whose constants
        // its methods are compiled with, and which is looked for beside that library first
        const doc: FormDocument = { $schema: 'foxdev-form', version: 1, form: classAsNode(definition), meta: definition.meta };
        const headers = await readHeaderFiles(formHeaderRefs(doc), [dirname(library.path), ...(ffcDir === '' ? [] : [ffcDir])]);
        compiled = vm.loadModule(requireBytes(`${library.alias}.${definition.name}`, compileForm(definition.name, formMethodSources(doc), headers)));
        classModules.set(key, compiled);
      }
      // a visual class is a window or a control, so the desktop has to be there to put it on
      if (!isNonVisualBaseClass(definition.baseClass)) showDesktop();
      return desktop.createClassObject(
        classAsNode(definition),
        compiled,
        {
          className: definition.name,
          baseClass: definition.baseClass,
          library: library.path,
          parentClass: definition.parentClass ?? '',
          arrays: definition.meta?.vfp?.arrays,
          expressions: definition.meta?.vfp?.expressions,
        },
        into,
      );
    }

    /** Loads a compiled module once and keeps it, as a form's is kept, by the name it was for. */
    function moduleFor(name: string, out: CompileOutput): number {
      const key = `formset:${name.toLowerCase()}`;
      const already = modules.get(key);
      if (already !== undefined) return already;
      const module = vm.loadModule(requireBytes(name, out));
      modules.set(key, module);
      return module;
    }

    /**
     * Opens the tables a form's data environment names, each in a work area of its own.
     *
     * This is FoxPro, so it is run as FoxPro: the same USE that a program would write, through
     * the same data engine. A table that will not open is reported and the rest still open, which
     * is what VFP does with a data environment it cannot fully satisfy.
     */
    async function openDataEnvironment(
      cursors: FormCursor[] | undefined,
      relations: FormRelation[] | undefined,
      formName: string,
      formFile?: string,
    ): Promise<void> {
      const home = formFile === undefined ? '' : dirname(formFile);
      for (const cursor of cursors ?? []) {
        const exclusive = cursor.exclusive ? ' EXCLUSIVE' : '';
        // the order the cursor asks for is the tag of the index beside the table
        const order = cursor.order ? `\nSET ORDER TO ${cursor.order}` : '';
        const line = `SELECT 0\nUSE ("${tableOf(cursor, home).replace(/"/g, '')}") ALIAS ${cursor.alias}${exclusive}${order}`;
        await setUpData(line, formName);
      }
      // the links come after every table is open, because a relation names two of them, and
      // the child has to be in the order the relation matches on before it can be sought in
      for (const relation of relations ?? []) {
        const order = relation.childOrder ? `SELECT ${relation.child}\nSET ORDER TO ${relation.childOrder}\n` : '';
        // ADDITIVE because a parent may lead more than one child, as the Grid sample's does
        const skip = relation.oneToMany ? `\nSET SKIP TO ${relation.child}` : '';
        const line = `${order}SELECT ${relation.parent}\nSET RELATION TO ${relation.expression} INTO ${relation.child} ADDITIVE${skip}`;
        await setUpData(line, formName);
      }
      // VFP leaves the first cursor selected, and form code counts on it: the Solution sample's
      // own tree is filled by a bare SCAN, which needs the right work area to be the current one
      const first = cursors?.[0];
      if (first) await setUpData(`SELECT ${first.alias}`, formName);
    }

    /**
     * Runs one line of a form's data environment.
     *
     * A table that will not open or a link that cannot be made is a fault in the form, not in the
     * program that opened it, so it is written into Output rather than stopping anything: the
     * form still gets its other tables. It must not raise the error dialog either - that waits
     * for an answer nobody is there to give while a form is being built.
     */
    async function setUpData(line: string, formName: string): Promise<void> {
      const guarded = `PUBLIC _fdvdataerr\n_fdvdataerr = ""\nTRY\n${line}\nCATCH TO loFdvErr\n_fdvdataerr = loFdvErr.Message\nENDTRY`;
      try {
        await runSnippet(guarded, `${formName} data environment`);
      } catch (e) {
        print({ kind: 'error', text: `${formName} data environment: ${e instanceof Error ? e.message : String(e)}` });
        return;
      }
      const problem = vm.getGlobal('_fdvdataerr');
      if (typeof problem === 'string' && problem !== '') {
        print({ kind: 'error', text: `${formName} data environment: ${problem}` });
      }
    }

    /**
     * Where a cursor's table is.
     *
     * A table that belongs to a database is named on its own - `CursorSource = "customer"` - and
     * the container is the only record of where it sits. Its own folder is where VFP would find
     * it through the `.dbc`, which this runtime does not read yet.
     */
    function tableOf(cursor: FormCursor, home: string): string {
      const source = cursor.source.trim();
      const database = cursor.database?.trim() ?? '';
      const named = database !== '' && !/[\\/]/.test(source) ? `${database.replace(/[^\\/]*$/, '')}${source}` : source;
      // A data environment names its tables the way the designer wrote them down: relative to the
      // form file itself. `..\..\data\employee.dbf` on a form in Samples\Solution\Forms is
      // Samples\Data\employee.dbf, and means nothing read against anything else - which is how so
      // many samples came to open with no table at all.
      if (named === '' || isAbsolute(named) || home === '') return named;
      return resolveFrom(home, named);
    }

    /**
     * Opens a port, and subscribes to what arrives on it the first time one is opened.
     *
     * A request is answered by `answerRequest`, which matches the route in the VM - where the
     * routes were registered - and dispatches the lambda as a fiber of its own. Nothing here
     * calls into the VM from inside a host request: `onRequest` arrives from the main process,
     * with nothing of ours on the stack.
     */
    async function listenHttp(server: number, port: number): Promise<number> {
      httpListening ??= getApi().http.onRequest((id, arrived) => {
        void answerRequest(arrived, vm, scheduler, desktop)
          .then((out) => getApi().http.respond(id, out))
          .catch(() => getApi().http.respond(id, null));
      });
      const bound = await getApi()
        .http.listen(server, port)
        .catch((e: unknown) => {
          throw new HostError(3001, e instanceof Error ? e.message : String(e));
        });
      httpServers.add(server);
      return bound;
    }

    function perform(request: HostRequest, ctx: FiberContext): VmValue | Promise<VmValue> {
      switch (request.kind) {
        case 'SetProp':
          desktop.setProp(request.obj, request.name, request.value);
          return null;

        case 'SetPropIndex':
          desktop.setPropIndex(request.obj, request.name, request.index, request.value);
          return null;

        case 'DimProp':
          desktop.dimProp(request.obj, request.name, request.rows, request.cols);
          return null;

        case 'CreateException':
          return { $obj: desktop.createException(request) };

        case 'CallMethod': {
          const target = desktop.object(request.obj);
          // FoxPro code of the object's own - a program's class, or a class out of a library -
          // is given the arguments as they came, so a variable passed with @ is the variable
          if (target && (target.hasClassMethod(request.name) || target.hasOwnMethod(request.name))) {
            const outcome = desktop.dispatch(target, request.name, request.args);
            if (outcome === null) return null;
            return outcome instanceof Promise ? outcome.then((o) => o.value) : outcome.value;
          }
          // everything else is answered here, and wants values
          const result = desktop.callMethod(request.obj, request.name, request.args.map(argValue));
          if (result === undefined) throw new HostError(1925, `Unknown member ${request.name.toUpperCase()}.`);
          return result;
        }

        // DODEFAULT(): the next class up that has code for the method being run, called with the
        // same THIS. Measured in Visual FoxPro 9: the arguments go with it, its answer is the
        // answer, a class with no code for the method is passed over, and past the last one the
        // answer is .T. Where to start looking is the class whose code is running, not the
        // object's: an inherited method that calls up must not find itself again.
        case 'CallParentMethod': {
          const target = desktop.object(request.obj);
          const form = target?.form();
          if (!target || !form || form.module < 0) return true;
          const settle = (outcome: ReturnType<typeof scheduler.dispatch>) =>
            outcome instanceof Promise ? outcome.then((o) => o.value) : outcome!.value;

          // An object of a class library carries its ancestors' code in its own module: the
          // import kept an overridden method's earlier versions as `Init#1`, `Init#2`, nearest
          // first. The next one up from the version running is the parent's.
          const running = /^(.*?)(?:#(\d+))?$/.exec(request.method)!;
          const event = running[1]!;
          const depth = Number(running[2] ?? 0);
          const copy = scheduler.dispatch(form.module, target.path(), `${event}#${depth + 1}`, target.handle, request.args);
          if (copy !== null) return settle(copy);

          // A class of a program: each class's methods are compiled under the class's own name,
          // so the chain is walked from the class that wrote the running code.
          if (!form.className) return true;
          const classes = classesFor(form.module, {
            name: form.className,
            baseClass: '',
            properties: [],
            members: [],
            methods: [],
            module: form.module,
          });
          const writer = (request.from ?? '').split('.')[0]!.toLowerCase();
          let name = classes.some((c) => c.name.toLowerCase() === writer) ? writer : form.className.toLowerCase();
          const seen = new Set<string>();
          while (name !== '' && !seen.has(name)) {
            seen.add(name);
            const above = classes.find((c) => c.name.toLowerCase() === name)?.baseClass ?? '';
            const parent = classes.find((c) => c.name.toLowerCase() === above.toLowerCase());
            if (!parent) return true;
            name = parent.name.toLowerCase();
            const outcome = scheduler.dispatchClass(form.module, parent.name, target.path(), event, target.handle, request.args);
            if (outcome !== null) return settle(outcome);
          }
          return true;
        }

        case 'AddProperty':
          return desktop.addProperty(request.obj, request.name, request.value);

        case 'RemoveProperty':
          return desktop.removeProperty(request.obj, request.name);

        case 'BindEvent':
          return desktop.bindEvent(request.source, request.event, request.handler, request.delegate, request.flags);

        case 'UnbindEvent':
          return desktop.unbindEvent(request.source ?? undefined, request.event ?? undefined, request.handler ?? undefined, request.delegate ?? undefined);

        case 'RaiseEvent': {
          const target = desktop.object(request.source);
          if (!target) throw new HostError(1943, 'Member  does not evaluate to an object.');
          const outcome = desktop.dispatch(target, request.event, request.args);
          // RAISEEVENT answers with a logical, not with nothing - measured
          return outcome instanceof Promise ? outcome.then(() => true) : true;
        }

        // MOUSE puts the pointer where the command said and presses there, so whatever is
        // drawn under it reacts as it would to a hand
        case 'MousePress':
          return mousePress(request);

        // RUN hands a command line to the operating system, as the command is for
        case 'RunProgram':
          return getApi()
            .project.run(request.command, useProjectStore.getState().dir() ?? '', request.nowait)
            .catch(() => -1);

        // `oServer.Listen(nPort)` and `oServer.Close()`. The sockets are in the main process;
        // everything the runtime knows about a request comes back through `http:request`, which
        // is the host speaking first.
        case 'HttpListen':
          return listenHttp(request.server, request.port);

        case 'HttpClose':
          httpServers.delete(request.server);
          return getApi().http.close(request.server);

        // FLUSH and DOEVENTS ask only that what the program has done be caught up with; a turn
        // of the event loop is what that means where the host draws between statements
        case 'Settle':
          return new Promise<VmValue>((resolve) => setTimeout(() => resolve(null), 0));

        // HOME() answers about the installation: the product directory, the samples, the
        // wizards. FoxDev has none of those of its own, so the main process answers from the
        // Visual FoxPro installation when there is one, and from the project when there is not.
        // the A...() functions that ask about what the host holds rather than what the VM
        // does: the objects alive, what is bound to them, and the classes it can make
        case 'Enumerate':
          return enumerate(desktop, request.what, request.name);

        // GETENV() asks the operating system, which only the main process can do
        case 'Environment':
          return getApi()
            .project.getEnv(request.name)
            .catch(() => '');

        case 'HomeDir':
          return getApi()
            .project.homeDir(request.which, useProjectStore.getState().dir() ?? '')
            .catch(() => '');

        case 'GetKey':
          return new Promise<VmValue>((resolve) => {
            set({ dialog: { id: ++dialogSeq, kind: 'key', title: 'Press a key', text: 'Press any key to continue.', flags: 0, defaultText: '', resolve } });
          });

        case 'GetColor':
          return new Promise<VmValue>((resolve) => {
            set({ dialog: { id: ++dialogSeq, kind: 'color', title: 'Colour', text: 'Choose a colour.', flags: 0, defaultText: String(request.default ?? 0), resolve } });
          });

        case 'GetFont':
          return new Promise<VmValue>((resolve) => {
            const start = [request.name || 'Segoe UI', String(request.size || 9), request.style || ''].join(',');
            set({ dialog: { id: ++dialogSeq, kind: 'font', title: 'Font', text: 'Choose a font.', flags: 0, defaultText: start, resolve } });
          });

        case 'ReleaseObject': {
          const target = desktop.object(request.obj);
          // `RELEASE THISFORMSET` takes every form of the formset with it, which is how a
          // formset's Close button is written
          if (target instanceof FormSetInstance) return desktop.releaseFormSet(target).then(() => null);
          const form = target instanceof FormInstance ? target : target?.form();
          return form ? desktop.releaseForm(form, { queryUnload: false }).then(() => null) : null;
        }

        case 'DoForm':
          return (async () => {
            const instance = await openForm(request.name, request.args, request.modal, request.noshow, ctx.fiber);
            if (!instance) return null;
            // TO waits for the form to close, which is right for a modal form that was shown.
            // Measured, and not what happens with NOSHOW: on a form whose WindowType is Modal,
            // `DO FORM paramask WITH "Q", 2 TO xRet NOSHOW` comes straight back with xRet still
            // holding what it held before, and it is the form's Release, later, that puts the
            // Unload return there. Writing a variable after the statement that named it has
            // finished is not something this runtime can do yet.
            const wantsResult = request.want_result;
            const result = instance.modal || wantsResult ? await instance.whenClosed() : null;
            if (request.want_object && wantsResult) return { $arr: [{ $obj: instance.handle }, result], $cols: 0 };
            if (request.want_object) return { $obj: instance.handle };
            return wantsResult ? result : null;
          })();

        // SQLCONNECT() and its family reach a data source, which is ADO on the other side of
        // COM; what comes back is the connection number, or the rows a statement answered
        case 'Sql':
          return performSql(request.what, request.handle, request.text);

        // BUILD APP / EXE: the project turned into the file that ships. BUILD PROJECT makes
        // the project itself out of the files it names.
        case 'Build':
          return (async () => {
            const from = request.from[0];
            if (request.what === 'PROJECT') {
              await buildProject(request.target, request.from);
              return null;
            }
            if (request.what === 'APP') {
              await buildApp(request.target, from);
              return null;
            }
            if (request.what === 'EXE') {
              await buildExecutable(request.target, from);
              return null;
            }
            // DLL and MTDLL build an in-process COM server, and what an Electron application
            // can register is an out-of-process one; the addon this runtime talks to COM
            // through is a client of servers, not one itself
            throw new HostError(
              1001,
              `BUILD ${request.what}: an in-process COM server is a .dll Windows loads into the ` +
                'calling program, which this runtime cannot be; BUILD EXE makes the same project ' +
                'into an application that runs on its own',
            );
          })();

        // COMPILE: source read and checked, one file or a directory of them
        case 'Compile':
          return compileFiles(request.what, request.files).then(() => null);

        // CREATE FORM, CREATE MENU and the rest: the designer for something new of that kind
        case 'NewDocument':
          return runtimeUi
            .newDocument(request.what, request.path)
            .then(() => null)
            .catch((e: unknown) => {
              print({ kind: 'error', text: e instanceof Error ? e.message : String(e) });
              return null;
            });

        // a function that has to run a whole statement - REQUERY() runs a view's SELECT again
        // - hands the line over rather than compiling it itself
        case 'RunLine':
          return runSnippet(request.text, 'line').then(() => null);

        // GETOBJECT() reaches something that is already running, or a document
        case 'GetObject':
          return { $obj: desktop.hostHandle(activeComObject(request.name, request.class, desktop)) };

        // `SET CLASSLIB TO`: the libraries whose classes the program may name. The answer is
        // what SET("CLASSLIB") is to report, which the VM keeps; the libraries themselves are
        // held here, because reading a file is the host's to do.
        case 'LoadClassLib':
          return (async () => {
            if (request.files.length === 0 && !request.additive) {
              classLibraries.clear();
              return '';
            }
            return classLibraries.set(request.files, request.alias, request.additive);
          })();

        case 'CreateObject':
          return (async () => {
            const definition = request.definition;
            if (!definition) {
              // `NEWOBJECT("x", "lib.vcx")` names the file to read the class out of, and reads
              // it whether or not SET CLASSLIB has it; nothing else is looked at
              const named = request.module ?? '';
              if (named !== '') {
                const object = await libraryObject(request.class, named);
                if (object === undefined) throw new HostError(1733, `Class definition ${request.class.toUpperCase()} is not found.`);
                return object === null ? false : { $obj: object.handle };
              }
              // the classes Visual FoxPro itself provides come before COM does
              const built = baseClassObject(request.class);
              if (built) return { $obj: desktop.hostHandle(built) };
              // the visual base classes are classes too: CREATEOBJECT("Form") makes a form
              // standing on its own, which is how a program builds windows as it goes
              const bare = desktop.createBaseObject(request.class);
              if (bare) return { $obj: bare.handle };
              // then the class libraries the program has loaded, which is where a class the
              // program never wrote down comes from
              const fromLibrary = await libraryObject(request.class, '');
              if (fromLibrary !== undefined) return fromLibrary === null ? false : { $obj: fromLibrary.handle };
              // no class of that name anywhere: in VFP the next thing tried is COM, which is
              // how a program reaches Word, Excel, ADO or anything else registered
              return { $obj: desktop.hostHandle(createComObject(request.class, desktop)) };
            }

            // the parent class may be defined in the same program, so resolve against them all
            const known = classesFor(definition.module ?? -1, definition);
            const built = buildClassInstance(known, definition.name);
            if (!built) throw new HostError(1733, `Class definition ${request.class.toUpperCase()} is not found.`);
            for (const w of built.warnings) print({ kind: 'error', text: `${w.object}: ${w.message}` });

            if (!built.nonVisual) showDesktop();
            const instance = desktop.instantiate(built.form, definition.module ?? -1, {
              // measured: a program's class answers for its Class with the first letter capital
              // and the rest small, however DEFINE CLASS wrote it - `mYthingHere` is Mythinghere
              className: definition.name.charAt(0).toUpperCase() + definition.name.slice(1).toLowerCase(),
              nonVisual: built.nonVisual,
              args: request.args,
            });
            // `DIMENSION` in the class body: an array property the node tree could not carry
            for (const array of built.arrays) {
              let owner: RuntimeObject | undefined = instance;
              for (const step of array.path) owner = owner?.child(step);
              owner?.declareArray(array.name, array.values.length);
            }
            // "Greet" belongs to the object; "image1.Click" belongs to that member
            const resolved = resolveInheritance(known, definition.name);
            for (const method of resolved?.methods ?? []) {
              const dot = method.name.lastIndexOf('.');
              if (dot < 0) instance.classMethods.add(method.name.toLowerCase());
              else instance.child(method.name.slice(0, dot))?.classMethods.add(method.name.slice(dot + 1).toLowerCase());
            }
            const created = await desktop.runFormLifecycle(instance, {
              noshow: true,
              args: request.args,
              // a property written as an expression is worked out now, where CREATEOBJECT() was
              // called - measured: a variable set just before it is seen
              expressions: built.expressions,
              inFrame: (expression) => vm.evaluateIn(ctx.fiber, -1, expression),
            });
            if (!created) return null;
            return { $obj: instance.handle };
          })();

        case 'MessageBox':
          return new Promise<VmValue>((resolve) => {
            set({ dialog: { id: ++dialogSeq, kind: 'message', title: request.title, text: request.text, flags: request.flags, defaultText: '', resolve } });
          });

        case 'InputBox':
          return new Promise<VmValue>((resolve) => {
            set({ dialog: { id: ++dialogSeq, kind: 'input', title: request.title, text: request.prompt, flags: 0, defaultText: request.default, resolve } });
          });

        case 'WaitWindow': {
          if (request.clear && !request.text) {
            set({ wait: null });
            return null;
          }
          const id = ++dialogSeq;
          set({ wait: { id, text: request.text } });
          if (request.nowait) return null;
          // no keyboard dismissal yet: honour the timeout, else show briefly
          const ms = (request.timeout ?? 1.5) * 1000;
          return new Promise<VmValue>((resolve) =>
            setTimeout(() => {
              set((s) => (s.wait?.id === id ? { wait: null } : {}));
              resolve('');
            }, ms),
          );
        }

        case 'FileRead':
          return locate(at(request.path), request.search.map(at))
            .then((path) => getApi().files.readText(path))
            .catch(() => {
              throw new HostError(1, `File '${request.path}' does not exist.`);
            });

        case 'FileWrite':
          return reach(at(request.path))
            .then((path) => getApi().files.writeText(path, request.text))
            .then(() => request.text.length)
            .catch(() => {
              throw new HostError(1102, `Cannot create file ${request.path}`);
            });

        case 'FileExists':
          // a path the main process will not open is a file the program cannot see, not a crash
          return locate(at(request.path), request.search.map(at))
            .then((path) => getApi().files.exists(path))
            .catch(() => false);

        case 'FileDelete':
          return reach(at(request.path))
            .then((path) => getApi().files.remove(path))
            .catch(() => {
              throw new HostError(1, `File '${request.path}' does not exist.`);
            });

        case 'GetFile':
          return getApi()
            .dialog.openFile({ filters: request.extensions ? [{ name: request.extensions, extensions: request.extensions.split(/[,;]/) }] : undefined })
            .then((p) => p ?? '');

        case 'PutFile':
          return getApi()
            .dialog.saveFile({ defaultPath: request.default_name })
            .then((p) => p ?? '');

        // ---- the data engine ----------------------------------------------------------
        //
        // The VM asks in records; the file answers in bytes. The two are joined here by the
        // header the table opened with, which is the only thing outside the VM that reads a
        // DBF at all - and it reads two 16-bit numbers, not a field.
        case 'DataOpen':
          return (async () => {
            const opened = await getApi()
              .data.open(await locate(at(request.path), request.search.map(at)), request.exclusive)
              .catch(() => {
                throw new HostError(1, `File '${request.path}' does not exist.`);
              });
            layouts.set(opened.handle, {
              headerLen: (opened.header.charCodeAt(8) | (opened.header.charCodeAt(9) << 8)) || 0,
              recordLen: (opened.header.charCodeAt(10) | (opened.header.charCodeAt(11) << 8)) || 0,
            });
            return { $arr: [opened.handle, opened.header], $cols: 0 };
          })();

        // FOPEN() and its family, ADIR(), COPY FILE and the rest: one operation, answered with
        // the value and the FERROR() number, which the VM keeps
        case 'FileOp':
          return (async () => {
            const path = request.path ? await locate(request.path, request.search) : '';
            const target = request.target && request.op !== 'dir' ? await reach(request.target) : request.target;
            const result = await getApi().files.lowlevel({ ...request, path, target });
            const value = lowLevelToVm(result.value);
            return { $arr: [value, result.error], $cols: 0 };
          })();

        // `DECLARE ... DLL` made the name callable; this is the call
        case 'CallDll':
          return (async () => {
            const result = await getApi()
              .dll.call({
                library: request.library,
                function: request.function,
                returns: request.returns,
                params: request.params,
                byRef: request.by_ref,
                // a VFP value crossing to C is a string, a number or a logical; anything else
                // (an object, an array) is not something a library parameter can be
                args: request.args.map((a) => (typeof a === 'object' ? null : (a ?? null))),
              })
              .catch((error: unknown) => {
                // The library service puts the VFP error number at the front of what it throws -
                // `1754|Cannot find entry point X in the DLL.` - because an error crossing from
                // the main process arrives as text and nothing else. Measured: a library Windows
                // will not load is 1753 and a function none of them exported is 1754.
                const message = error instanceof Error ? error.message : String(error);
                const numbered = /^(\d{1,5})\|(.*)$/s.exec(message);
                if (numbered) throw new HostError(Number(numbered[1]), numbered[2] ?? '');
                throw new HostError(1, message);
              });
            // a parameter passed by reference comes back beside the return value, in the order
            // the arguments were given, and the VM writes each one back
            if (!request.by_ref.some(Boolean)) return result.value;
            return { $arr: [result.value, { $arr: result.written, $cols: 0 }], $cols: 0 };
          })();

        case 'DataCreate':
          return (async () => {
            const path = await reach(at(request.path));
            const header = request.header.map((b) => String.fromCharCode(b)).join('');
            await getApi()
              .data.create(path, header, request.memo)
              .catch(() => {
                throw new HostError(1102, `Cannot create file ${request.path}`);
              });
            return null;
          })();

        case 'DataRead': {
          const layout = layouts.get(request.handle);
          if (!layout) return '';
          const offset = layout.headerLen + (request.first - 1) * layout.recordLen;
          return getApi().data.read(request.handle, offset, request.count * layout.recordLen);
        }

        // the compound index beside the table, whole, so a tag can be walked from its root
        case 'DataIndex':
          return getApi()
            .data.readIndex(request.handle)
            .catch(() => '');

        case 'DataWriteIndex':
          return getApi()
            .data.writeIndex(request.handle, request.bytes.map((b) => String.fromCharCode(b)).join(''))
            .then(() => null)
            .catch(() => {
              throw new HostError(1102, 'Cannot write the index');
            });

        // the database container and the memo file beside it are written whole
        case 'FileWriteBytes':
          return (async () => {
            const path = await reach(at(request.path));
            await getApi()
              .files.writeBytes(path, request.bytes.map((b) => String.fromCharCode(b)).join(''))
              .catch(() => {
                throw new HostError(1102, `Cannot write ${request.path}`);
              });
            return null;
          })();

        // LOADPICTURE(): the VM has read the file and knows what it is a picture of; what it
        // wants back is an object holding the four things a picture answers to.
        case 'MakePicture':
          return {
            $obj: desktop.hostHandle(
              new DataObject('Picture', {
                Handle: request.picture,
                Type: request.of_kind,
                Width: request.width,
                Height: request.height,
              }),
            ),
          };

        case 'FileReadBytes':
          return (async () => {
            const path = at(request.path);
            const bytes = await getApi()
              .files.readBytes(await reach(path))
              .catch(() => new Uint8Array());
            let text = '';
            for (const b of bytes) text += String.fromCharCode(b);
            return text;
          })();

        // a memo's text goes beside the table, and the block it lands at goes in the record
        case 'DataWriteMemo':
          return getApi().data.writeMemo(request.handle, request.bytes.map((b) => String.fromCharCode(b)).join(''));

        case 'DataReadMemo':
          return getApi().data.readMemo(request.handle, request.block);

        // what an autoincrementing field takes next lives in the header, so it goes back too
        case 'DataWriteHeader':
          return (async () => {
            await getApi().data.write(
              request.handle,
              0,
              request.bytes.map((b) => String.fromCharCode(b)).join(''),
            );
            return null;
          })();

        case 'DataWrite': {
          const layout = layouts.get(request.handle);
          if (!layout) return null;
          const offset = layout.headerLen + (request.recno - 1) * layout.recordLen;
          const bytes = request.bytes.map((b) => String.fromCharCode(b)).join('');
          return (async () => {
            await getApi().data.write(request.handle, offset, bytes);
            // a record was added: the header's count goes back with it, so nothing ever reads a
            // table whose last record the header does not admit to
            if (request.count !== null) {
              const n = request.count;
              const le = [n & 0xff, (n >> 8) & 0xff, (n >> 16) & 0xff, (n >>> 24) & 0xff];
              await getApi().data.write(request.handle, 4, le.map((b) => String.fromCharCode(b)).join(''));
            }
            return null;
          })();
        }

        case 'DataClose':
          return Promise.all(
            request.handles.map((handle) => {
              layouts.delete(handle);
              return getApi().data.close(handle).catch(() => undefined);
            }),
          ).then(() => null);

        // MODIFY COMMAND and BROWSE ask the development environment to open a file, and this is
        // one: the file opens in a tab, in whatever editor its kind calls for.
        case 'OpenDocument':
          return runtimeUi
            .openDocument(at(request.path))
            .then(() => null)
            .catch((e: unknown) => {
              print({ kind: 'error', text: e instanceof Error ? e.message : String(e) });
              return null;
            });

        case 'LoadProgram':
          return (async () => {
            const compiled = await source.getProgram(baseName(request.name));
            if (!compiled) {
              // the product's words: the name in lower case, with a program's extension when it
              // came without one - `File 'hello.prg' does not exist.`
              const file = request.name.split(/[\\/]/).pop() ?? request.name;
              const shown = file === file.toUpperCase() ? file.toLowerCase() : file;
              throw new HostError(1, `File '${shown.includes('.') ? shown : `${shown}.prg`}' does not exist.`);
            }
            const cached = modules.get(compiled.name.toLowerCase());
            if (cached !== undefined) return cached;
            const id = vm.loadModule(compiled.bytes);
            modules.set(compiled.name.toLowerCase(), id);
            desktop.programs.set(compiled.name.toLowerCase(), id);
            return id;
          })();

        // ACTIVATE MENU: the program built the menu itself, so it arrives whole
        case 'SetMenu':
          if (request.menu) showDesktop();
          set({ menu: request.menu, menuSkip: {} });
          return null;

        // BROWSE: the records of a work area, in a window of their own
        case 'Browse': {
          const browse = request.browse;
          showDesktop();
          set((s) => ({ browses: [...s.browses.filter((b) => b.alias !== browse.alias), browse] }));
          if (request.nowait) return null;
          // without NOWAIT the command waits for the window to be closed, as VFP does
          return new Promise<VmValue>((resolve) => {
            browseWaiters.set(browse.alias, () => resolve(null));
          });
        }

        // MODIFY MEMO puts a window on one memo field; without NOWAIT the command waits for
        // it, and what the window is left with goes back into the field
        case 'EditMemo': {
          const { alias, field, text, noedit, nowait } = request;
          showDesktop();
          set((s) => ({
            memos: [...s.memos.filter((m) => memoKey(m.alias, m.field) !== memoKey(alias, field)), { alias, field, text, noedit }],
          }));
          if (nowait) return null;
          return new Promise<VmValue>((resolve) => {
            memoWaiters.set(memoKey(alias, field), (left) => resolve(left));
          });
        }

        // CLOSE MEMO closes the windows and hands back what each was left holding, so the
        // fields keep it
        case 'CloseMemo': {
          const wanted = request.fields.map((f) => f.toUpperCase());
          const open = get().memos.filter(
            (m) => wanted.length === 0 || wanted.includes(m.field.toUpperCase()) || wanted.includes(memoKey(m.alias, m.field)),
          );
          for (const memo of open) memoWaiters.delete(memoKey(memo.alias, memo.field));
          set((s) => ({ memos: s.memos.filter((m) => !open.includes(m)) }));
          return { $arr: open.map((m) => ({ $arr: [m.alias, m.field, m.text], $cols: 0 })), $cols: 0 };
        }

        // the character screen: @ ... SAY and the window commands hand over the whole thing
        case 'SetScreen':
          if (request.screen.windows.length > 0 || request.screen.lines.some((l) => l.trim())) showDesktop();
          set({ screen: request.screen });
          return null;

        case 'ReadGets':
          return new Promise<VmValue>((resolve) => {
            set({ read: { id: ++dialogSeq, fields: request.fields, prompts: [], resolve } });
          });

        case 'ChooseFrom':
          return new Promise<VmValue>((resolve) => {
            set({ read: { id: ++dialogSeq, fields: [], prompts: request.prompts, resolve } });
          });

        case 'DoMenu':
          return (async () => {
            const menu = await source.getMenu(baseName(request.name));
            if (!menu) throw new HostError(1, `Menu '${request.name}' does not exist.`);
            showDesktop();
            // VFP runs a menu's Setup code as the menu is installed, before it is on screen
            if (menu.setup?.trim()) await runSnippet(menu.setup, `${menu.name} Setup`);
            if (!menu.shortcut) {
              set({ menu, menuSkip: {} });
              return null;
            }
            // a shortcut menu is defined as a popup and activated where the pointer is, and the
            // program waits there until something is chosen or the menu is dismissed
            const chosen = await new Promise<MenuItem | null>((resolve) => {
              set({ shortcutMenu: { id: ++dialogSeq, menu, resolve }, menuSkip: {} });
            });
            set({ shortcutMenu: null });
            if (chosen) await get().chooseMenuItem(chosen);
            if (menu.cleanup?.trim()) await runSnippet(menu.cleanup, `${menu.name} Cleanup`);
            return null;
          })();

        default:
          return null;
      }
    }

    /** Every class a module defines, so inheritance resolves without asking the VM again. */
    const classCache = new Map<number, VfpClassDef[]>();
    function classesFor(module: number, fallback: VfpClassDef): VfpClassDef[] {
      if (module < 0) return [fallback];
      const known = definedClasses(module);
      return known.length > 0 ? known : [fallback];
    }

    /** A `DEFINE CLASS` of that name in any program loaded so far, with the module holding it. */
    function programClass(className: string): VfpClassDef | null {
      const wanted = className.toLowerCase();
      for (const id of new Set(modules.values())) {
        const found = definedClasses(id).find((c) => c.name.toLowerCase() === wanted);
        if (found) return { ...found, module: found.module ?? id };
      }
      return null;
    }

    /** The `DEFINE CLASS` definitions a module holds, read once and kept. */
    function definedClasses(module: number): VfpClassDef[] {
      if (module < 0) return [];
      let known = classCache.get(module);
      if (!known) {
        known = vm.classDefinitions(module);
        classCache.set(module, known);
      }
      return known;
    }

    /**
     * One of the classes Visual FoxPro provides itself - XMLAdapter, Exception, Project and
     * the rest - or nothing when the name is not one of them.
     */
    function baseClassObject(className: string): HostObject | undefined {
      if (className.toLowerCase() === 'xmladapter') {
        return new XmlAdapter({
          // the adapter names the public variable the document is in rather than writing it
          // into a line of source, so nothing in the XML can be read as FoxPro
          run: (expression: string) => runExpression(expression),
          setGlobal: (name: string, value: VmValue) => vm.setGlobal(name, value),
          getGlobal: (name: string) => vm.getGlobal(name),
          // a document is bytes: the VM decodes it as its own declaration says it is written
          readFile: async (path: string) => {
            const bytes = await getApi()
              .files.readBytes(await reach(at(path)))
              .catch(() => {
                throw new HostError(1, `File '${path}' does not exist.`);
              });
            let raw = '';
            for (const b of bytes) raw += String.fromCharCode(b);
            return vm.xmlText(raw);
          },
          writeFile: async (path: string, text: string) => {
            await getApi().files.writeText(await reach(at(path)), text);
          },
          shapeOf: (text: string) => vm.xmlShape(text),
        });
      }
      return registryObject(className);
    }

    /** Compiles and runs a fragment of FoxPro on this session (menu items, Setup/Cleanup). */
    /** How many quiet runs are under way; an error inside one is the caller's to make of. */
    let quietRuns = 0;
    const silenceErrors = (): (() => void) => {
      quietRuns += 1;
      return () => {
        quietRuns -= 1;
      };
    };

    async function runSnippet(text: string, label: string, quiet = false): Promise<VmValue> {
      const out = compileSnippet(text, label);
      if (!out.bytes) {
        const diag = out.diagnostics.find((d) => d.severity === 'error');
        throw new CompileFailure(label, diag ? `${diag.message} (line ${diag.line})` : 'Compilation failed');
      }
      // a quiet run answers or does not; what went wrong belongs to whoever asked, not to the
      // program that happens to be running
      const before = quiet ? silenceErrors() : null;
      try {
        const outcome = await scheduler.runProgram(vm.loadModule(out.bytes));
        return outcome.value;
      } finally {
        before?.();
      }
    }

    /**
     * Works out one expression and answers what it came to.
     *
     * A fragment is a program of its own, so what it works out has to be left somewhere the
     * caller can reach: a public variable, which is what PUBLIC is for.
     */
    async function runExpression(expression: string, quiet = false): Promise<VmValue> {
      await runSnippet(`PUBLIC _xmlanswer\n_xmlanswer = ${expression}`, 'XMLAdapter', quiet);
      return vm.getGlobal('_xmlanswer');
    }

    set({
      desktop,
      vm,
      scheduler,
      releaseTables,
      target,
      status: 'running',
      output: [],
      errorReport: null,
      dialog: null,
      wait: null,
      menu: null,
      shortcutMenu: null,
      menuSkip: {},
      ...runCleared(),
    });
    return { desktop, vm, scheduler, openForm, modules, runSnippet };
  };

  /** Runs work, turning a compile failure or a cancel into an Output line rather than a crash. */
  const guard = async (work: () => Promise<unknown>) => {
    try {
      await work();
    } catch (err) {
      if (err instanceof CompileFailure) print({ kind: 'error', text: err.message });
      else if (err instanceof Error && err.name !== 'Cancelled') print({ kind: 'error', text: err.message });
    }
  };

  return {
    status: 'idle',
    desktop: null,
    scheduler: null,
    vm: null,
    target: null,
    output: [],
    dialog: null,
    wait: null,
    errorReport: null,
    history: [],
    traceEvents: false,
    revision: 0,
    menu: null,
    shortcutMenu: null,
    menuSkip: {},
    screen: null,
    browses: [],
    memos: [],
    setMemoText(alias, field, text) {
      set((s) => ({
        memos: s.memos.map((m) => (memoKey(m.alias, m.field) === memoKey(alias, field) ? { ...m, text } : m)),
      }));
    },
    closeMemo(alias, field) {
      const key = memoKey(alias, field);
      const memo = get().memos.find((m) => memoKey(m.alias, m.field) === key);
      set((s) => ({ memos: s.memos.filter((m) => memoKey(m.alias, m.field) !== key) }));
      const waiting = memoWaiters.get(key);
      if (waiting) {
        memoWaiters.delete(key);
        waiting(memo?.text ?? '');
      }
    },
    closeBrowse(alias) {
      set((s) => ({ browses: s.browses.filter((b) => b.alias !== alias) }));
      const waiting = browseWaiters.get(alias);
      if (waiting) {
        browseWaiters.delete(alias);
        waiting();
      }
    },
    read: null,
    pointer: { row: 0, col: 0, down: false },
    setPointer(at) {
      set({ pointer: at });
    },
    releaseTables: null,

    async runForm(source, name) {
      const session = await startSession(source, name);
      await guard(async () => {
        const instance = await session.openForm(name, [], null, false);
        if (!instance) print({ kind: 'error', text: `${name}: Init returned .F., the form was not created` });
      });
    },

    async runProgram(source, name) {
      const session = await startSession(source, name);
      await guard(async () => {
        const compiled = await source.getProgram(baseName(name));
        if (!compiled) throw new CompileFailure(name, 'program not found');
        const module = session.vm.loadModule(compiled.bytes);
        session.modules.set(compiled.name.toLowerCase(), module);
        session.desktop.programs.set(compiled.name.toLowerCase(), module);
        await session.scheduler.runProgram(module);
      });
    },

    async execute(source, text) {
      const line = text.trim();
      if (!line) return;
      set((s) => ({ history: [...s.history.filter((h) => h !== line), line] }));

      let scheduler = get().scheduler;
      let vm = get().vm;
      if (!scheduler || !vm) {
        // starting a session clears Output, so echo the line only once one exists
        const session = await startSession(source, 'Command Window');
        scheduler = session.scheduler;
        vm = session.vm;
        set({ status: 'idle' });
      }
      print({ kind: 'echo', text: line });
      await guard(async () => {
        const out = compileSnippet(line, 'Command');
        if (!out.bytes) {
          const diag = out.diagnostics.find((d) => d.severity === 'error');
          throw new CompileFailure('Command', diag ? diag.message : 'Compilation failed');
        }
        await scheduler.runProgram(vm.loadModule(out.bytes));
      });
    },

    async chooseMenuItem(item) {
      const text = item.result.text?.trim();
      if (!text) return;
      const { scheduler, vm } = get();
      if (!scheduler || !vm) return;
      // BAR(), PAD(), POPUP() and PROMPT() answer what was chosen while the command runs
      const chosen = /^(pad|bar|popup):(.*)$/.exec(item.id);
      if (chosen) {
        const kind = chosen[1];
        const rest = chosen[2] ?? '';
        const cut = rest.lastIndexOf(':');
        const popup = kind === 'bar' ? rest.slice(0, cut) : kind === 'popup' ? rest : '';
        const bar = kind === 'bar' ? Number(rest.slice(cut + 1)) : 0;
        vm.menuChosen?.(kind === 'pad' ? rest : '', Number.isFinite(bar) ? bar : 0, popup, item.prompt);
      }
      await guard(async () => {
        const out = compileSnippet(text, item.name ?? item.prompt);
        if (!out.bytes) {
          const diag = out.diagnostics.find((d) => d.severity === 'error');
          throw new CompileFailure(item.prompt, diag ? `${diag.message} (line ${diag.line})` : 'Compilation failed');
        }
        await scheduler.runProgram(vm.loadModule(out.bytes));
      });
    },

    async refreshMenuSkip() {
      const { menu, scheduler, vm } = get();
      if (!menu || !scheduler || !vm) return;
      const skip: Record<string, boolean> = {};
      const walk = async (items: MenuItem[]): Promise<void> => {
        for (const item of items) {
          if (item.skipFor?.trim()) {
            // SKIP FOR is a logical expression: true disables the item
            const out = compileSnippet(`? IIF(${item.skipFor}, "1", "0")`, 'SkipFor');
            skip[item.id] = false;
            if (out.bytes) {
              try {
                const before = get().output.length;
                await scheduler.runProgram(vm.loadModule(out.bytes));
                const line = get().output[get().output.length - 1];
                if (get().output.length > before && line?.kind === 'output') {
                  skip[item.id] = line.text.trim() === '1';
                  set((st) => ({ output: st.output.slice(0, before) }));
                }
              } catch {
                skip[item.id] = false;
              }
            }
          }
          if (item.children?.length) await walk(item.children);
        }
      };
      await walk(menu.items);
      set({ menuSkip: skip });
    },

    cancel() {
      const { scheduler, desktop } = get();
      get().releaseTables?.();
      releaseComObjects();
      releaseLibraries();
      releaseHttpServers();
      closeSqlConnections();
      scheduler?.cancelAll();
      useDebugStore.getState().detach();
      if (desktop) {
        for (const form of [...desktop.forms]) void desktop.releaseForm(form, { queryUnload: false });
        // a formset whose forms all declined to go still has to: the run is over
        for (const set of [...desktop.formSets]) void desktop.releaseFormSet(set);
      }
      get().dialog?.resolve(null);
      get().read?.resolve(null);
      for (const done of browseWaiters.values()) done();
      browseWaiters.clear();
      set({
        status: 'idle',
        desktop: null,
        scheduler: null,
        vm: null,
        releaseTables: null,
        target: null,
        dialog: null,
        wait: null,
        errorReport: null,
        menu: null,
      shortcutMenu: null,
        menuSkip: {},
        revision: get().revision + 1,
        ...runCleared(),
      });
    },

    print,
    clearOutput: () => set({ output: [] }),
    setTraceEvents: (on) => set({ traceEvents: on }),
  };
});

/** Text for an Output line, used by the panel and by tests. */
export function formatOutput(line: OutputLine): string {
  return line.kind === 'echo' ? `. ${line.text}` : line.text;
}

/** MESSAGEBOX button sets, from the low 4 bits of the flags. */
export function messageBoxButtons(flags: number): { label: string; value: number }[] {
  switch (flags & 0x0f) {
    case 1:
      return [
        { label: 'OK', value: 1 },
        { label: 'Cancel', value: 2 },
      ];
    case 2:
      return [
        { label: 'Abort', value: 3 },
        { label: 'Retry', value: 4 },
        { label: 'Ignore', value: 5 },
      ];
    case 3:
      return [
        { label: 'Yes', value: 6 },
        { label: 'No', value: 7 },
        { label: 'Cancel', value: 2 },
      ];
    case 4:
      return [
        { label: 'Yes', value: 6 },
        { label: 'No', value: 7 },
      ];
    case 5:
      return [
        { label: 'Retry', value: 4 },
        { label: 'Cancel', value: 2 },
      ];
    default:
      return [{ label: 'OK', value: 1 }];
  }
}

export type { RuntimeObject };
export { displayValue };
