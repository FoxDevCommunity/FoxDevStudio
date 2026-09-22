import { MENU_SCHEMA_ID, MENU_VERSION, menuDocumentSchema } from './schema';
import type { MenuDocument, MenuItem } from './schema';
import type { ParseResult } from '../form/schema';
import { formatIssues } from '../form/serialize';

export function createEmptyMenuDocument(name = 'Menu1'): MenuDocument {
  return { $schema: MENU_SCHEMA_ID, version: MENU_VERSION, name, location: 'Replace', items: [] };
}

export function parseMenuDocument(text: string): ParseResult<MenuDocument> {
  let raw: unknown;
  try {
    raw = JSON.parse(text);
  } catch (e) {
    return { ok: false, error: `Invalid JSON: ${(e as Error).message}` };
  }
  const result = menuDocumentSchema.safeParse(raw);
  if (!result.success) return { ok: false, error: formatIssues(result.error.issues) };
  return { ok: true, doc: result.data };
}

export function stringifyMenuDocument(doc: MenuDocument): string {
  const out: Record<string, unknown> = {
    $schema: doc.$schema,
    version: doc.version,
    name: doc.name,
    location: doc.location,
  };
  if (doc.shortcut) out['shortcut'] = true;
  if (doc.setup !== undefined) out['setup'] = doc.setup;
  if (doc.cleanup !== undefined) out['cleanup'] = doc.cleanup;
  out['items'] = doc.items.map(canonicalItem);
  return JSON.stringify(out, null, 2) + '\n';
}

function canonicalItem(item: MenuItem): Record<string, unknown> {
  const out: Record<string, unknown> = { id: item.id, prompt: item.prompt };
  if (item.name !== undefined) out['name'] = item.name;
  out['result'] = item.result.text === undefined ? { type: item.result.type } : { type: item.result.type, text: item.result.text };
  if (item.hotkey) out['hotkey'] = item.hotkey;
  if (item.skipFor !== undefined) out['skipFor'] = item.skipFor;
  if (item.message !== undefined) out['message'] = item.message;
  if (item.enabled !== undefined) out['enabled'] = item.enabled;
  if (item.children) out['children'] = item.children.map(canonicalItem);
  return out;
}
