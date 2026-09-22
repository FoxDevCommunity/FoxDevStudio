import { FORM_SCHEMA_ID, FORM_VERSION, formDocumentSchema } from './schema';
import type { ControlNode, FormDocument, FormNode, ParseResult } from './schema';

export function createEmptyFormDocument(name = 'Form1'): FormDocument {
  return { $schema: FORM_SCHEMA_ID, version: FORM_VERSION, form: { name, props: {}, methods: {}, children: [] } };
}

export function parseFormDocument(text: string): ParseResult<FormDocument> {
  let raw: unknown;
  try {
    raw = JSON.parse(text);
  } catch (e) {
    return { ok: false, error: `Invalid JSON: ${(e as Error).message}` };
  }
  const migrated = migrateFormDocument(raw);
  const result = formDocumentSchema.safeParse(migrated);
  if (!result.success) return { ok: false, error: formatIssues(result.error.issues) };
  return { ok: true, doc: result.data };
}

/** Upgrades older on-disk versions in place. Only version 1 exists today. */
export function migrateFormDocument(raw: unknown): unknown {
  return raw;
}

/** Stable 2-space JSON: fixed key order for nodes, alphabetical props/methods, so diffs stay clean. */
export function stringifyFormDocument(doc: FormDocument): string {
  const out: Record<string, unknown> = {
    $schema: doc.$schema,
    version: doc.version,
    form: canonicalForm(doc.form),
  };
  // the tables the form opens: without these a saved form loads with no data environment, and
  // every method that reads a field fails on a form that ran perfectly when it was imported
  if (doc.data?.length) out['data'] = doc.data;
  if (doc.meta) out['meta'] = doc.meta;
  return JSON.stringify(out, null, 2) + '\n';
}

/** The node's keys in a fixed order. A class library writes a class through this too. */
export function canonicalForm(form: FormNode) {
  return {
    name: form.name,
    props: sortedRecord(form.props),
    methods: sortedRecord(form.methods),
    children: form.children.map(canonicalControl),
  };
}

export function canonicalControl(node: ControlNode): Record<string, unknown> {
  const out: Record<string, unknown> = {
    id: node.id,
    type: node.type,
    name: node.name,
    props: sortedRecord(node.props),
    methods: sortedRecord(node.methods),
  };
  if (node.children) out['children'] = node.children.map(canonicalControl);
  return out;
}

export function sortedRecord<T>(rec: Record<string, T>): Record<string, T> {
  const out: Record<string, T> = {};
  for (const key of Object.keys(rec).sort((a, b) => a.localeCompare(b))) out[key] = rec[key]!;
  return out;
}

export function formatIssues(issues: { path: PropertyKey[]; message: string }[]): string {
  return issues
    .slice(0, 5)
    .map((i) => `${i.path.map(String).join('.') || '(root)'}: ${i.message}`)
    .join('; ');
}
