/**
 * Where a running session gets its code. The IDE implementation compiles from the open
 * documents (so Run Form reflects unsaved edits), the player implementation reads a packed
 * `.fxa` bundle. Both hand back compiled bytecode plus, for forms, the document to build the
 * object tree from.
 */

import type { ControlNode, FormDocument, FormNode } from '../form/schema';
import type { MenuDocument } from '../menu/schema';
import { getObjectDescriptor } from '../registry';
import type { CompileOutput } from './host';

export interface CompiledForm {
  name: string;
  doc: FormDocument;
  bytes: Uint8Array;
}

export interface CompiledProgram {
  name: string;
  bytes: Uint8Array;
}

/** Thrown when the source exists but does not compile; carries the diagnostics for Output. */
export class CompileFailure extends Error {
  constructor(
    readonly target: string,
    readonly detail: string,
  ) {
    super(`${target}: ${detail}`);
    this.name = 'CompileFailure';
  }
}

export interface ProgramSource {
  /** `DO x` / Do Program. `null` when there is no such program. */
  getProgram(name: string): Promise<CompiledProgram | null>;
  /** `DO FORM x`. `null` when there is no such form. */
  getForm(name: string): Promise<CompiledForm | null>;
  /** `DO x.fxm`. `null` when there is no such menu. */
  getMenu(name: string): Promise<MenuDocument | null>;
}

/**
 * Strips a directory and a known extension so `DO FORM HelloWorld.scx` matches `HelloWorld`.
 * `.fxp` here is Visual FoxPro's compiled program, not a FoxDev project, which is `.fxproject`.
 *
 * `.mpr` is on the list because a menu is always named by it: generating a menu writes the
 * program `chkmenu.mpr` beside the `chkmenu.mnx` it was drawn in, and `DO chkmenu.mpr` is how
 * every form that puts a menu up says so. What answers is the menu, so the name has to reach it.
 */
export function baseName(name: string): string {
  const file = name.replace(/\\/g, '/').split('/').pop() ?? name;
  return file.replace(/\.(fxf|scx|fxm|mnx|mpr|prg|fxp|app|exe)$/i, '');
}

/** Compiled modules are cached by content so re-running a form does not recompile it. */
export class CompileCache<T> {
  private readonly entries = new Map<string, { key: string; value: T }>();

  get(name: string, key: string): T | undefined {
    const hit = this.entries.get(name.toLowerCase());
    return hit && hit.key === key ? hit.value : undefined;
  }

  set(name: string, key: string, value: T): T {
    this.entries.set(name.toLowerCase(), { key, value });
    return value;
  }

  clear(): void {
    this.entries.clear();
  }
}

/** Turns a compile result into bytes or throws a `CompileFailure` naming the first error. */
export function requireBytes(target: string, out: CompileOutput): Uint8Array {
  if (out.bytes) return out.bytes;
  const error = out.diagnostics.find((d) => d.severity === 'error');
  if (error) throw new CompileFailure(target, `${error.message} (line ${error.line})`);
  const method = out.methodDiagnostics?.find((m) => m.diagnostic.severity === 'error');
  if (method) throw new CompileFailure(`${target}.${method.method}`, `${method.diagnostic.message} (line ${method.diagnostic.line})`);
  throw new CompileFailure(target, 'Compilation failed');
}

export interface MethodSourceInput {
  /** Path from the form (`pgfMain.Page1.lblGreeting`); empty for the form itself. */
  objectPath: string;
  event: string;
  /** The event's VFP parameter list, compiled as implicit LPARAMETERS. */
  params: string;
  source: string;
  /**
   * The header file whose `#DEFINE`s this method is compiled with, by name without folder or
   * extension and upper-cased - the key the compiler's `headers` map uses. `""` when the file
   * the method came from named none.
   */
  include: string;
}

/** A header file reference as a `.scx` or `.vcx` wrote it, by the name the compiler asks for. */
export function headerStem(reference: string): string {
  const name = reference.trim().split(/[\\/]/).pop() ?? '';
  const dot = name.lastIndexOf('.');
  return (dot > 0 ? name.slice(0, dot) : name).toUpperCase();
}

/**
 * Every header file this form's methods are compiled with, as the file wrote the reference.
 *
 * Reading them is the caller's, as it is for a program: the compiler is handed their text. A
 * form usually names one, and a control taken from a class library brings that library's.
 */
export function formHeaderRefs(doc: FormDocument): string[] {
  const vfp = doc.meta?.vfp;
  const out = new Set<string>();
  if (vfp?.include) out.add(vfp.include);
  for (const reference of Object.values(vfp?.includes ?? {})) if (reference !== '') out.add(reference);
  return [...out];
}

/** Every method that has code, flattened for `compile_form`. */
export function formMethodSources(doc: FormDocument): MethodSourceInput[] {
  const out: MethodSourceInput[] = [];
  const vfp = doc.meta?.vfp;

  const add = (objectPath: string, node: ControlNode | FormNode): void => {
    const events = getObjectDescriptor(node).events;
    for (const [event, source] of Object.entries(node.methods)) {
      if (!source.trim()) continue;
      // `Init#1` is an ancestor's copy of Init, which DODEFAULT() reaches, and takes Init's parameters
      const copied = event.replace(/#\d+$/, '').toLowerCase();
      const params = events.find((e) => e.name.toLowerCase() === copied)?.params ?? '';
      const key = (objectPath === '' ? event : `${objectPath}.${event}`).toLowerCase();
      const include = vfp?.includes?.[key] ?? vfp?.include ?? '';
      out.push({ objectPath, event, params, source, include: headerStem(include) });
    }
  };

  add('', doc.form);
  const walk = (nodes: ControlNode[], prefix: string): void => {
    for (const node of nodes) {
      const path = prefix ? `${prefix}.${node.name}` : node.name;
      add(path, node);
      if (node.children?.length) walk(node.children, path);
    }
  };
  walk(doc.form.children, '');
  return out;
}
