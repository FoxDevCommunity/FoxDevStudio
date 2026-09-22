/**
 * Adapts the wasm `FoxVm` to the `VmLike` interface the scheduler uses.
 *
 * The guard matters: wasm-bindgen exported methods are not re-entrant, and jsdom fires focus
 * events synchronously, so a handler that called back into the VM would throw an opaque
 * "recursive use of an object" panic. Here it fails loudly instead, in the test that caused it.
 */

import type { XmlShape } from '@shared/runtime/xmlAdapter';
import type { CompileOutput, DebugFrame, DebugVariable, StackEntry, StepMode, StepResult, VmLike } from '@shared/runtime/host';
import type { VfpClassDef } from '@shared/runtime/classDef';
import type { VmValue } from '@shared/runtime/values';
import { headerStem } from '@shared/runtime/programSource';
import { loadFoxVm, loadFoxVmSync, type FoxVmModule } from '../../../wasm/foxvm/loader';

export interface MethodSourceInput {
  objectPath: string;
  event: string;
  params: string;
  source: string;
  /** The header file the method is compiled with, keyed as `headers` is; `""` for none. */
  include?: string;
}

export class ReentrancyError extends Error {
  constructor(method: string) {
    super(`FoxVM was called re-entrantly (${method}). A host request handler must not call back into the VM.`);
    this.name = 'ReentrancyError';
  }
}

/** Where a host read leaves an exception it was not allowed to throw. */
interface ReadFault {
  error?: unknown;
}

class WasmVm implements VmLike {
  private inWasm = false;

  constructor(
    private readonly vm: InstanceType<FoxVmModule['FoxVm']>,
    private readonly fault: ReadFault = {},
  ) {}

  private guard<T>(name: string, fn: () => T): T {
    if (this.inWasm) throw new ReentrancyError(name);
    this.inWasm = true;
    try {
      const value = fn();
      // a host read that failed could not say so at the time; this is the first safe moment
      const failure = this.fault.error;
      if (failure !== undefined) {
        this.fault.error = undefined;
        throw failure;
      }
      return value;
    } finally {
      this.inWasm = false;
    }
  }

  loadModule(bytes: Uint8Array): number {
    return this.guard('loadModule', () => this.vm.load_module(bytes));
  }
  start(module: number, funcName: string, thisHandle: number | null, args: VmValue[]): number {
    return this.guard('start', () => this.vm.start(module, funcName, thisHandle ?? undefined, args));
  }
  startMethod(module: number, objPath: string, event: string, thisHandle: number, args: VmValue[]): number | null {
    return this.guard('startMethod', () => this.vm.start_method(module, objPath, event, thisHandle, args) ?? null);
  }
  startClassMethod(module: number, className: string, objPath: string, event: string, thisHandle: number, args: VmValue[]): number | null {
    return this.guard('startClassMethod', () => this.vm.start_class_method(module, className, objPath, event, thisHandle, args) ?? null);
  }
  startFunction(func: number, args: VmValue[]): number | null {
    return this.guard('startFunction', () => this.vm.start_function(func, args) ?? null);
  }
  /** Which lambda a request matches, and what the named parts of its path matched. */
  route(server: number, method: string, path: string): { function: number; params: Record<string, string> } | undefined {
    return this.guard('route', () => this.vm.route(server, method, path) as { function: number; params: Record<string, string> } | undefined);
  }
  classDefinitions(module: number): VfpClassDef[] {
    return this.guard('classDefinitions', () => this.vm.class_definitions(module) as VfpClassDef[]);
  }
  step(fiber: number): StepResult {
    return this.guard('step', () => this.vm.step(fiber) as StepResult);
  }
  resume(fiber: number, value: VmValue): void {
    this.guard('resume', () => this.vm.resume(fiber, value));
  }
  resumeError(fiber: number, code: number, message: string): void {
    this.guard('resumeError', () => this.vm.resume_error(fiber, code, message));
  }
  abort(fiber: number): void {
    this.guard('abort', () => this.vm.abort(fiber));
  }
  abortAll(): void {
    this.guard('abortAll', () => this.vm.abort_all());
  }
  callStack(fiber: number): StackEntry[] {
    return this.guard('callStack', () => this.vm.call_stack(fiber) as StackEntry[]);
  }
  setSetting(name: string, value: VmValue): void {
    this.guard('setSetting', () => this.vm.set_setting(name, value));
  }
  menuChosen(pad: string, bar: number, popup: string, prompt: string): void {
    this.guard('menuChosen', () => this.vm.menu_chosen(pad, bar, popup, prompt));
  }
  setGlobal(name: string, value: VmValue): void {
    this.guard('setGlobal', () => this.vm.set_global(name, value));
  }
  getGlobal(name: string): VmValue {
    return this.guard('getGlobal', () => this.vm.get_global(name) as VmValue);
  }
  /** The text of an XML document from its bytes, decoded as its declaration says. */
  xmlText(bytes: string): string {
    return this.guard('xmlText', () => this.vm.xml_text(bytes));
  }
  /** What a document holds, for the XMLAdapter. */
  xmlShape(text: string): XmlShape {
    return this.guard('xmlShape', () => this.vm.xml_shape(text) as XmlShape);
  }
  getSetting(name: string): VmValue {
    return this.guard('getSetting', () => this.vm.get_setting(name) as VmValue);
  }
  /** Evaluates an expression in the current frame of a suspended fiber (Command Window, watches). */
  evaluate(fiber: number, expr: string): VmValue {
    return this.guard('evaluate', () => this.vm.evaluate(fiber, expr) as VmValue);
  }

  // ---- the debugger ----

  setBreakpoint(program: string, line: number, on: boolean): void {
    this.guard('setBreakpoint', () => this.vm.set_breakpoint(program, line, on));
  }
  clearBreakpoints(): void {
    this.guard('clearBreakpoints', () => this.vm.clear_breakpoints());
  }
  setStepMode(fiber: number, mode: StepMode): void {
    this.guard('setStepMode', () => this.vm.set_step_mode(fiber, mode));
  }
  frames(fiber: number): DebugFrame[] {
    return this.guard('frames', () => this.vm.frames(fiber) as DebugFrame[]);
  }
  frameVariables(fiber: number, level: number): DebugVariable[] {
    return this.guard('frameVariables', () => this.vm.frame_variables(fiber, level) as DebugVariable[]);
  }
  /** A watch expression, read in a chosen frame of a stopped fiber. */
  evaluateIn(fiber: number, level: number, expr: string): VmValue {
    return this.guard('evaluateIn', () => this.vm.evaluate_in(fiber, level, expr) as VmValue);
  }
}

export type { WasmVm };

/**
 * The host reads, made incapable of throwing.
 *
 * These are called from inside the VM. An exception thrown out of one unwinds the wasm frame
 * without unwinding the Rust one, which leaves the VM's own borrow of itself in place: every call
 * after it fails with "recursive use of an object detected", and the session is dead. One
 * mistake in a host read should cost that read, not the runtime, so each returns undefined
 * instead - which every one of them already treats as "no answer".
 */
function safeReads(reads: unknown, fault: ReadFault): unknown {
  const source = reads as Record<string, unknown>;
  return new Proxy(source, {
    get(target, key) {
      const value = Reflect.get(target, key) as unknown;
      if (typeof value !== 'function') return value;
      return (...args: unknown[]) => {
        try {
          return (value as (...a: unknown[]) => unknown).apply(target, args);
        } catch (error) {
          // the read answers "nothing" and the reason is raised the moment wasm is off the stack
          fault.error ??= error;
          return undefined;
        }
      };
    },
  });
}

/** Creates a VM bound to a set of synchronous host reads. */
export function createVm(reads: unknown, module?: FoxVmModule): WasmVm {
  const mod = module ?? loadFoxVmSync();
  const fault: ReadFault = {};
  return new WasmVm(new mod.FoxVm(safeReads(reads, fault)), fault);
}

export async function createVmAsync(reads: unknown): Promise<WasmVm> {
  return createVm(reads, await loadFoxVm());
}

// ---- compilation (static, no VM instance needed) ----

export function compileProgram(
  source: string,
  name: string,
  headers?: Record<string, string>,
  module?: FoxVmModule,
): CompileOutput {
  return (module ?? loadFoxVmSync()).FoxVm.compile_program(source, name, headers ?? {}) as CompileOutput;
}

/**
 * The header files a program asks for: every `#INCLUDE` line's name, as the line wrote it - the
 * folder included, because a header is looked for beside the file that named it. The compiler
 * needs their text, and reading files is not its to do.
 */
export function includedHeaderNames(source: string): string[] {
  const out: string[] = [];
  for (const line of source.split(/\r?\n/)) {
    const found = /^\s*#\s*(?:INCLUDE|INSERT)\s+(.+?)\s*$/i.exec(line);
    if (!found) continue;
    const name = (found[1] ?? '').replace(/^["']|["']$/g, '').trim();
    if (name !== '') out.push(name);
  }
  return out;
}

/** The same, reduced to the name the compiler's `headers` map is keyed by. */
export function includedHeaders(source: string): string[] {
  return includedHeaderNames(source).map(headerStem);
}

export function compileForm(name: string, methods: MethodSourceInput[], headers?: Record<string, string>, module?: FoxVmModule): CompileOutput {
  return (module ?? loadFoxVmSync()).FoxVm.compile_form(name, methods, headers ?? {}) as CompileOutput;
}

export function compileSnippet(source: string, name: string, module?: FoxVmModule): CompileOutput {
  return (module ?? loadFoxVmSync()).FoxVm.compile_snippet(source, name) as CompileOutput;
}

export function compileExpression(source: string, module?: FoxVmModule): CompileOutput {
  return (module ?? loadFoxVmSync()).FoxVm.compile_expression(source) as CompileOutput;
}

/** First error message from a compile result, or null when it produced a module. */
export function compileError(out: CompileOutput): string | null {
  const err = out.diagnostics.find((d) => d.severity === 'error');
  if (err) return `${err.message} (line ${err.line})`;
  const method = out.methodDiagnostics?.find((m) => m.diagnostic.severity === 'error');
  if (method) return `${method.method}: ${method.diagnostic.message} (line ${method.diagnostic.line})`;
  return out.bytes ? null : 'Compilation failed';
}
