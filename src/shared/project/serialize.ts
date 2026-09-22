import { PROJECT_SCHEMA_ID, PROJECT_VERSION, projectDocumentSchema } from './schema';
import type { ProjectDocument } from './schema';
import type { ParseResult } from '../form/schema';
import { formatIssues } from '../form/serialize';

export function createEmptyProjectDocument(name: string): ProjectDocument {
  return { $schema: PROJECT_SCHEMA_ID, version: PROJECT_VERSION, name, items: [] };
}

export function parseProjectDocument(text: string): ParseResult<ProjectDocument> {
  let raw: unknown;
  try {
    raw = JSON.parse(text);
  } catch (e) {
    return { ok: false, error: `Invalid JSON: ${(e as Error).message}` };
  }
  const result = projectDocumentSchema.safeParse(raw);
  if (!result.success) return { ok: false, error: formatIssues(result.error.issues) };
  return { ok: true, doc: result.data };
}

export function stringifyProjectDocument(doc: ProjectDocument): string {
  const out: Record<string, unknown> = { $schema: doc.$schema, version: doc.version, name: doc.name };
  if (doc.main !== undefined) out['main'] = doc.main;
  out['items'] = doc.items.map((i) => (i.excluded ? { kind: i.kind, path: i.path, excluded: true } : { kind: i.kind, path: i.path }));
  if (doc.settings) out['settings'] = doc.settings;
  return JSON.stringify(out, null, 2) + '\n';
}
