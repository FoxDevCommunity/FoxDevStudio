/**
 * `.fxa` application bundle: a whole project compiled to bytecode, ready to run without the
 * IDE. This is the artefact the runtime player loads and that a built executable embeds.
 *
 * Plain JSON with base64 bytecode so it travels over the existing text file IPC. A HelloWorld
 * bundle is a few kilobytes; the size that matters is the player, not the program.
 */

import { z } from 'zod';
import { formDocumentSchema, type FormDocument } from '../form/schema';
import { menuDocumentSchema, type MenuDocument } from '../menu/schema';
import type { ProjectDocument } from '../project/schema';
import type { ParseResult } from '../form/schema';
import { baseName, formMethodSources, requireBytes, type ProgramSource } from './programSource';
import type { CompileOutput } from './host';

export const BUNDLE_SCHEMA = 'foxdev-app';
export const BUNDLE_VERSION = 1;

const bundleFormSchema = z.object({ doc: formDocumentSchema, bytecode: z.string() });
const bundleProgramSchema = z.object({ name: z.string(), bytecode: z.string() });

export const appBundleSchema = z.object({
  $schema: z.literal(BUNDLE_SCHEMA),
  version: z.literal(BUNDLE_VERSION),
  name: z.string(),
  /** FoxVM crate version the bytecode was produced by; a mismatch means "rebuild". */
  vmVersion: z.string(),
  builtAt: z.string(),
  main: z.object({ kind: z.enum(['form', 'program']), name: z.string() }),
  /** Lower-cased base name -> entry. */
  programs: z.record(z.string(), bundleProgramSchema),
  forms: z.record(z.string(), bundleFormSchema),
  menus: z.record(z.string(), menuDocumentSchema),
});

export type AppBundle = z.infer<typeof appBundleSchema>;

export function encodeBytes(bytes: Uint8Array): string {
  let binary = '';
  for (const byte of bytes) binary += String.fromCharCode(byte);
  return btoa(binary);
}

export function decodeBytes(text: string): Uint8Array {
  const binary = atob(text);
  const out = new Uint8Array(binary.length);
  for (let i = 0; i < binary.length; i++) out[i] = binary.charCodeAt(i);
  return out;
}

export function stringifyBundle(bundle: AppBundle): string {
  return `${JSON.stringify(bundle, null, 2)}\n`;
}

export function parseBundle(text: string): ParseResult<AppBundle> {
  let raw: unknown;
  try {
    raw = JSON.parse(text);
  } catch {
    return { ok: false, error: 'Not a FoxDev application bundle: invalid JSON' };
  }
  const result = appBundleSchema.safeParse(raw);
  if (!result.success) {
    const issue = result.error.issues[0];
    return { ok: false, error: `Not a FoxDev application bundle: ${issue ? `${issue.path.join('.')} ${issue.message}` : 'unknown field'}` };
  }
  return { ok: true, doc: result.data };
}

/** What the IDE must supply to compile a project into a bundle. */
export interface BundleCompilers {
  compileForm(name: string, methods: ReturnType<typeof formMethodSources>): CompileOutput;
  compileProgram(source: string, name: string): CompileOutput;
  vmVersion: string;
}

export interface BundleInputs {
  project: ProjectDocument;
  /** Reads a project item, by its path relative to the project file. */
  readItem(relativePath: string): Promise<string>;
  parseForm(text: string): ParseResult<FormDocument>;
  parseMenu(text: string): ParseResult<MenuDocument>;
}

export interface PackResult {
  bundle: AppBundle;
  /** One line per item compiled, for the Output window. */
  log: string[];
}

/**
 * Compiles every included project item. Throws on the first compile error so Build App fails
 * loudly rather than shipping a bundle that cannot run.
 */
export async function packBundle(inputs: BundleInputs, compilers: BundleCompilers): Promise<PackResult> {
  const { project } = inputs;
  const bundle: AppBundle = {
    $schema: BUNDLE_SCHEMA,
    version: BUNDLE_VERSION,
    name: project.name,
    vmVersion: compilers.vmVersion,
    builtAt: new Date().toISOString(),
    main: { kind: 'form', name: '' },
    programs: {},
    forms: {},
    menus: {},
  };
  const log: string[] = [];

  for (const item of project.items) {
    if (item.excluded) continue;
    const key = baseName(item.path).toLowerCase();

    if (item.kind === 'form') {
      const parsed = inputs.parseForm(await inputs.readItem(item.path));
      if (!parsed.ok) throw new Error(`${item.path}: ${parsed.error}`);
      const bytes = requireBytes(item.path, compilers.compileForm(parsed.doc.form.name, formMethodSources(parsed.doc)));
      bundle.forms[key] = { doc: parsed.doc, bytecode: encodeBytes(bytes) };
      log.push(`Compiled form ${item.path}`);
    } else if (item.kind === 'program') {
      const text = await inputs.readItem(item.path);
      const name = baseName(item.path);
      const bytes = requireBytes(item.path, compilers.compileProgram(text, name));
      bundle.programs[key] = { name, bytecode: encodeBytes(bytes) };
      log.push(`Compiled program ${item.path}`);
    } else if (item.kind === 'menu') {
      const parsed = inputs.parseMenu(await inputs.readItem(item.path));
      if (!parsed.ok) throw new Error(`${item.path}: ${parsed.error}`);
      bundle.menus[key] = parsed.doc;
      log.push(`Added menu ${item.path}`);
    }
  }

  if (!project.main) throw new Error('The project has no main form or program. Use Set Main in the Project Explorer.');
  bundle.main = { kind: project.main.toLowerCase().endsWith('.prg') ? 'program' : 'form', name: baseName(project.main) };
  const mainKey = bundle.main.name.toLowerCase();
  const present = bundle.main.kind === 'program' ? bundle.programs[mainKey] : bundle.forms[mainKey];
  if (!present) throw new Error(`The main item ${project.main} is not part of the build.`);
  log.push(`Main: ${project.main}`);

  return { bundle, log };
}

/** A `ProgramSource` backed by a packed bundle: what the player runs from. */
export function createBundleSource(bundle: AppBundle): ProgramSource {
  return {
    getForm: (name) => {
      const entry = bundle.forms[baseName(name).toLowerCase()];
      return Promise.resolve(entry ? { name: entry.doc.form.name, doc: entry.doc, bytes: decodeBytes(entry.bytecode) } : null);
    },
    getProgram: (name) => {
      const entry = bundle.programs[baseName(name).toLowerCase()];
      return Promise.resolve(entry ? { name: entry.name, bytes: decodeBytes(entry.bytecode) } : null);
    },
    getMenu: (name) => Promise.resolve(bundle.menus[baseName(name).toLowerCase()] ?? null),
  };
}
