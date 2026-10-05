/**
 * The JS half of the VM boundary, mirroring `host.rs`.
 *
 * `HostReads` is called synchronously from inside wasm, so it must never re-enter the VM:
 * property reads and member classification only. Everything with a side effect arrives as a
 * `HostRequest` while the VM is off the stack, which is what lets the scheduler run nested
 * events, await a modal dialog, or park a fiber in READ EVENTS.
 */

import type { VmValue } from './values';
import type { VfpClassDef } from './classDef';
import type { MenuDocument } from '../menu/schema';

/** Synchronous, side-effect-free reads. Implemented by the Desktop (or a test stub). */
export interface HostReads {
  /** Property value, or `undefined` when the object has no such property (VFP error 1734). */
  getProp(obj: number, name: string): VmValue | undefined;
  /** A child object's handle, or what kind of member the name is. */
  getMember(obj: number, name: string): number | 'prop' | 'method' | 'none';
  /**
   * The objects a collection member holds, in the order `FOR EACH obj IN x.Member` visits them,
   * or `undefined` when the member is not a collection. `_SCREEN.Forms` is one: Visual FoxPro
   * walks it but will not read it as a value (1924, measured).
   */
  enumerate?(obj: number, name: string): VmValue | undefined;
  /**
   * Whether the handle was a form, or a member of one, that has been released: a variable still
   * holding it then reads as .NULL. (measured - `ISNULL(oForm)` after `oForm.Release()` is .T.).
   */
  released?(obj: number): boolean;
  /**
   * True when the object carries FoxPro source for a method of that name - its class's, or the
   * form's or library's own. A property read or write asks, for `Prop_Access` and `Prop_Assign`.
   */
  hasCodeMethod?(obj: number, name: string): boolean;
  /** Class name, or `null` when the handle has been released. */
  objectClass(obj: number): string | null;
  /**
   * The file the object was built from, for `SYS(1271, oObject)`: a form's `.scx`. Measured: the
   * product answers .F. for an object that came from no file, so `null` is what says so.
   */
  objectFile(obj: number): string | null;
  /** `?` / `??` output. */
  output(text: string, newline: boolean): void;
  /** Local clock for DATE()/DATETIME(). */
  now(): { days: number; secs: number };
  random(): number;
  /** `name|major|minor|build` for OS(), e.g. `Windows|10|0|26200`. */
  osInfo(): string;
  /** True when the object's class defines this method in source (not merely an event of that name). */
  hasClassMethod(obj: number, name: string): boolean;
  /** Module id of an already loaded program, or -1 to make the VM ask via `LoadProgram`. */
  resolveProgram(name: string): number;
  /**
   * Where the pointer is over the character screen, in rows and columns from its top left,
   * and whether a button is down. A host that does not track it leaves it at the corner.
   */
  mouse?(): { row: number; col: number; down: boolean };
  /** Optional log of every error the VM raises, with the program and line it came from, called
   * before anything decides what to do with it. `handled` means a TRY, an Error method or
   * ON ERROR took it - in which case this is the only record it ever leaves.
   */
  errorRaised?(error: RuntimeError, handled: boolean): void;
  /**
   * `SET LIBRARY TO`: loads a Visual FoxPro library, answering with the number this host will
   * know it by, the file it actually opened and the name of each function it adds - an empty
   * name where the library declared a function it does not want called, so the numbering still
   * lines up. A host with nowhere to put one answers with what went wrong instead.
   */
  loadLibrary?(path: string): { id: number; path: string; functions: string[] } | { error: string };
  /**
   * One of a loaded library's functions. Synchronous like everything else here, and for a
   * sharper reason: a program calls one from the middle of an expression the VM is already
   * evaluating, where there is nothing to suspend.
   */
  callLibrary?(library: number, fn: number, args: VmValue[]): { ok: true; value: VmValue } | { ok: false; code: number; message: string };
  unloadLibrary?(library: number): void;
}

export type HostRequest =
  | { kind: 'SetProp'; obj: number; name: string; value: VmValue }
  | { kind: 'SetPropIndex'; obj: number; name: string; index: number[]; value: VmValue }
  | { kind: 'DimProp'; obj: number; name: string; rows: number; cols: number }
  | { kind: 'CreateException'; code: number; message: string; program: string; line: number; user_value: VmValue }
  | { kind: 'CallMethod'; obj: number; name: string; args: VmValue[] }
  | { kind: 'DoForm'; name: string; args: VmValue[]; modal: boolean | null; linked: boolean; noshow: boolean; want_object: boolean; want_result: boolean }
  | { kind: 'ReleaseObject'; obj: number }
  | { kind: 'CreateObject'; class: string; args: VmValue[]; definition?: VfpClassDef | null; module?: string }
  | { kind: 'LoadClassLib'; files: string[]; alias: string; additive: boolean }
  | { kind: 'AddProperty'; obj: number; name: string; value: VmValue }
  | { kind: 'MessageBox'; text: string; flags: number; title: string; timeout: number | null }
  | { kind: 'InputBox'; prompt: string; title: string; default: string; timeout: number | null; timeout_value: string }
  | { kind: 'WaitWindow'; text: string; nowait: boolean; timeout: number | null; clear: boolean }
  | { kind: 'HttpListen'; server: number; port: number }
  | { kind: 'HttpClose'; server: number }
  | { kind: 'ReadEvents' }
  | { kind: 'ClearEvents' }
  | { kind: 'Quit' }
  | { kind: 'Cancel' }
  | { kind: 'GetFile'; extensions: string; title: string }
  | { kind: 'PutFile'; prompt: string; default_name: string; extension: string }
  // `search` on the requests that look for a file is where else to look when it is not at
  // `path`: the folders SET PATH TO named, each with the file's name on the end, in the order
  // the command wrote them. It is empty when nothing is to be searched - no path set, or a name
  // that carries a folder of its own, which the product does not look for on the path.
  | { kind: 'FileRead'; path: string; search: string[] }
  | { kind: 'FileWrite'; path: string; text: string; append: boolean }
  | { kind: 'FileExists'; path: string; search: string[] }
  | { kind: 'FileDelete'; path: string }
  // The data engine. The host owns bytes and the VM owns meaning: record numbers come out of the
  // VM and go back in as byte offsets, and nothing outside the VM decodes a field.
  | { kind: 'DataOpen'; path: string; exclusive: boolean; search: string[] }
  | { kind: 'DataRead'; handle: number; first: number; count: number }
  | { kind: 'DataWriteMemo'; handle: number; bytes: number[] }
  | { kind: 'FileWriteBytes'; path: string; bytes: number[] }
  | { kind: 'FileReadBytes'; path: string }
  // LOADPICTURE()'s answer: a bag holding what a picture answers to. The pixels stay in the VM,
  // which decoded them; `picture` is the handle that finds them again and 0 is the null picture.
  | { kind: 'MakePicture'; picture: number; of_kind: number; width: number; height: number }
  | { kind: 'DataIndex'; handle: number }
  | { kind: 'DataWriteIndex'; handle: number; bytes: number[] }
  | { kind: 'DataReadMemo'; handle: number; block: number }
  | { kind: 'CallDll'; library: string; function: string; returns: string; params: string[]; by_ref: boolean[]; args: VmValue[] }
  | {
      kind: 'FileOp';
      op: string;
      handle: number;
      path: string;
      target: string;
      text: string;
      count: number;
      offset: number;
      whence: number;
      search: string[];
    }
  | { kind: 'DataCreate'; path: string; header: number[]; memo: boolean }
  | { kind: 'DataWrite'; handle: number; recno: number; bytes: number[]; count: number | null }
  | { kind: 'DataWriteHeader'; handle: number; bytes: number[] }
  | { kind: 'DataClose'; handles: number[] }
  | { kind: 'OpenDocument'; path: string }
  | { kind: 'LoadProgram'; name: string }
  | { kind: 'DoMenu'; name: string }
  | { kind: 'SetMenu'; menu: MenuDocument | null }
  | { kind: 'SetScreen'; screen: ScreenDoc }
  | { kind: 'Browse'; browse: BrowseTable; nowait: boolean }
  | {
      kind: 'MousePress';
      clicks: number;
      at: [number, number] | null;
      drag: [number, number][];
      window: string;
      style: string;
    }
  | { kind: 'Enumerate'; what: number; name: string }
  | { kind: 'GetObject'; name: string; class: string }
  | { kind: 'RunLine'; text: string }
  | { kind: 'NewDocument'; what: string; path: string }
  | { kind: 'Build'; what: string; target: string; from: string[]; recompile: boolean }
  | { kind: 'Compile'; what: string; files: string; all: boolean; encrypt: boolean; nodebug: boolean }
  | { kind: 'CallParentMethod'; obj: number; method: string; args: VmValue[]; from?: string }
  | { kind: 'Sql'; what: number; handle: number; text: string; extra: string }
  | { kind: 'Environment'; name: string }
  | { kind: 'EditMemo'; alias: string; field: string; text: string; noedit: boolean; nowait: boolean }
  | { kind: 'CloseMemo'; fields: string[] }
  | { kind: 'RunProgram'; command: string; nowait: boolean }
  | { kind: 'Settle'; events: boolean }
  | { kind: 'ReadGets'; fields: GetField[] }
  | { kind: 'ChooseFrom'; prompts: string[] }
  | { kind: 'Break'; program: string; line: number; reason: BreakReason }
  | { kind: 'DebugResume' }
  | { kind: 'RemoveProperty'; obj: number; name: string }
  | { kind: 'HomeDir'; which: number }
  | { kind: 'GetKey' }
  | { kind: 'GetColor'; default: number }
  | { kind: 'GetFont'; name: string; size: number; style: string }
  | { kind: 'BindEvent'; source: number; event: string; handler: number; delegate: string; flags: number }
  | { kind: 'UnbindEvent'; source: number | null; event: string | null; handler: number | null; delegate: string | null }
  | { kind: 'RaiseEvent'; source: number; event: string; args: VmValue[] };

/** The character screen `@ ... SAY` draws on, and the windows over it. */
export interface ScreenDoc {
  rows: number;
  cols: number;
  /** One string per row, each `cols` characters wide. */
  lines: string[];
  windows: WindowDoc[];
}

export interface WindowDoc {
  name: string;
  title: string;
  footer: string;
  /** Where it sits on the screen, in characters. */
  row: number;
  col: number;
  height: number;
  width: number;
  minimized: boolean;
  border: boolean;
  lines: string[];
}

/** One field a `READ` is waiting on. */
export interface GetField {
  name: string;
  row: number;
  col: number;
  width: number;
  /** The PICTURE clause it was given, which says what may be typed into it. */
  picture: string;
  enabled: boolean;
  value: VmValue;
}

/** A work area as a Browse window shows it. */
export interface BrowseTable {
  alias: string;
  title: string;
  columns: { name: string; kind: string; width: number }[];
  rows: { recno: number; deleted: boolean; values: VmValue[] }[];
  /** How many records the work area holds, which is more than `rows` when some were left out. */
  count: number;
  editable: boolean;
}

export interface RuntimeError {
  code: number;
  message: string;
  /** Program or method the error happened in, e.g. `cmdSayHi.Click`. */
  program: string;
  line: number;
}

export interface StackEntry {
  program: string;
  line: number;
}

/** Why a program stopped where it did. Mirrors `host.rs::BreakReason`. */
export type BreakReason = 'breakpoint' | 'step' | 'suspend' | 'setstep';

/** How far a stopped program runs before it stops again. Mirrors `host.rs::StepMode`. */
export type StepMode = 'go' | 'into' | 'over' | 'out';

/** One frame of a stopped program, with the module its source was compiled from. */
export interface DebugFrame {
  program: string;
  module: string;
  line: number;
}

/** One variable a stopped frame can see of its own. */
export interface DebugVariable {
  /** Upper-cased, as the runtime keeps every variable name. */
  name: string;
  /** PRIVATE rather than a LOCAL slot. */
  private: boolean;
  value: VmValue;
}

/** A program stopped in the debugger, and what it took to stop it. */
export interface BreakStop {
  fiber: number;
  program: string;
  line: number;
  reason: BreakReason;
}

export type StepResult =
  | { state: 'done'; value: VmValue; nodefault: boolean }
  | { state: 'error'; error: RuntimeError; stack: StackEntry[] }
  | { state: 'suspend'; request: HostRequest };

export interface Diagnostic {
  severity: 'error' | 'warning';
  message: string;
  line: number;
  col: number;
  endLine: number;
  endCol: number;
  start: number;
  end: number;
}

export interface CompileOutput {
  bytes: Uint8Array | null;
  diagnostics: Diagnostic[];
  methodDiagnostics?: { method: string; diagnostic: Diagnostic }[];
}

/** The subset of the wasm `FoxVm` the scheduler uses, so it can be driven by a fake in tests. */
export interface VmLike {
  loadModule(bytes: Uint8Array): number;
  start(module: number, funcName: string, thisHandle: number | null, args: VmValue[]): number;
  /** `null` when the form has no code for that event. */
  startMethod(module: number, objPath: string, event: string, thisHandle: number, args: VmValue[]): number | null;
  /** The same, for an object created by `DEFINE CLASS`, whose methods are keyed by class name. */
  startClassMethod(module: number, className: string, objPath: string, event: string, thisHandle: number, args: VmValue[]): number | null;
  /**
   * A fiber whose first frame is a lambda: the host waking the runtime with a payload. `null`
   * when the VM has no function of that id, which is what a handler left over from an earlier
   * run looks like.
   */
  startFunction(func: number, args: VmValue[]): number | null;
  /** Every class a loaded module defines, so a parent class can be resolved before use. */
  classDefinitions(module: number): VfpClassDef[];
  step(fiber: number): StepResult;
  resume(fiber: number, value: VmValue): void;
  resumeError(fiber: number, code: number, message: string): void;
  abort(fiber: number): void;
  abortAll(): void;
  callStack(fiber: number): StackEntry[];
  setSetting(name: string, value: VmValue): void;
  /**
   * The debugger's reads and writes. Optional: a fake VM in a scheduler test need not have
   * them, and a session that never stops a program never calls them.
   */
  setBreakpoint?(program: string, line: number, on: boolean): void;
  clearBreakpoints?(): void;
  /** How far a stopped fiber runs when it is let go. Set before `resume`. */
  setStepMode?(fiber: number, mode: StepMode): void;
  frames?(fiber: number): DebugFrame[];
  frameVariables?(fiber: number, level: number): DebugVariable[];
  /** A watch expression, read in a chosen frame of a stopped fiber. */
  evaluateIn?(fiber: number, level: number, expr: string): VmValue;
  /**
   * Tells the VM what was chosen from the menu that is up, so BAR(), PAD(), POPUP() and
   * PROMPT() answer it while the command that choice stands for runs. Optional: a fake VM in
   * a test need not have one.
   */
  menuChosen?(pad: string, bar: number, popup: string, prompt: string): void;
  getSetting(name: string): VmValue;
}

/** Performs a yielded request. Returning a promise parks the fiber until it settles. */
export interface HostRequestHandler {
  perform(request: HostRequest, ctx: FiberContext): VmValue | Promise<VmValue>;
}

export interface FiberContext {
  fiber: number;
  /** Bumped whenever the session is cancelled; a stale fiber must not be resumed. */
  generation: number;
}

/** Thrown by `perform` to make the VM raise a catchable FoxPro error at the current statement. */
export class HostError extends Error {
  constructor(
    readonly code: number,
    message: string,
  ) {
    super(message);
    this.name = 'HostError';
  }
}
