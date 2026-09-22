import { canonicalControl, formatIssues, sortedRecord } from '../form/serialize';
import type { ParseResult } from '../form/schema';
import { CLASSLIB_SCHEMA_ID, CLASSLIB_VERSION, classLibraryDocumentSchema } from './schema';
import type { ClassDefinition, ClassLibraryDocument } from './schema';

export function createEmptyClassLibraryDocument(name = 'Classes'): ClassLibraryDocument {
  return { $schema: CLASSLIB_SCHEMA_ID, version: CLASSLIB_VERSION, name, classes: [] };
}

/** A class with nothing in it yet: the designer's New Class before anything is dropped on it. */
export function createClass(name: string, baseClass: string): ClassDefinition {
  return { name, baseClass, props: {}, methods: {}, children: [] };
}

export function parseClassLibraryDocument(text: string): ParseResult<ClassLibraryDocument> {
  let raw: unknown;
  try {
    raw = JSON.parse(text);
  } catch (e) {
    return { ok: false, error: `Invalid JSON: ${(e as Error).message}` };
  }
  const result = classLibraryDocumentSchema.safeParse(migrateClassLibraryDocument(raw));
  if (!result.success) return { ok: false, error: formatIssues(result.error.issues) };
  return { ok: true, doc: result.data };
}

/** Upgrades older on-disk versions in place. Only version 1 exists today. */
export function migrateClassLibraryDocument(raw: unknown): unknown {
  return raw;
}

/**
 * Stable 2-space JSON: a fixed key order, classes in the order the library keeps them, and
 * properties, methods and ownership lists sorted, so two saves of the same library diff to
 * nothing and a designer's edit diffs to the one class it touched.
 */
export function stringifyClassLibraryDocument(doc: ClassLibraryDocument): string {
  const out: Record<string, unknown> = {
    $schema: doc.$schema,
    version: doc.version,
    name: doc.name,
    classes: doc.classes.map(canonicalClass),
  };
  if (doc.meta) out['meta'] = doc.meta;
  return JSON.stringify(out, null, 2) + '\n';
}

function canonicalClass(cls: ClassDefinition): Record<string, unknown> {
  const out: Record<string, unknown> = { name: cls.name, baseClass: cls.baseClass };
  if (cls.parentClass) out['parentClass'] = cls.parentClass;
  if (cls.parentLibrary) out['parentLibrary'] = cls.parentLibrary;
  if (cls.description) out['description'] = cls.description;
  out['props'] = sortedRecord(cls.props);
  out['methods'] = sortedRecord(cls.methods);
  out['children'] = cls.children.map(canonicalControl);
  if (cls.own && Object.keys(cls.own).length > 0) {
    const own: Record<string, unknown> = {};
    for (const path of Object.keys(cls.own).sort((a, b) => a.localeCompare(b))) {
      const members = cls.own[path]!;
      const entry: Record<string, unknown> = {};
      if (members.props?.length) entry['props'] = [...members.props].sort((a, b) => a.localeCompare(b));
      if (members.methods?.length) entry['methods'] = [...members.methods].sort((a, b) => a.localeCompare(b));
      if (Object.keys(entry).length > 0) own[path] = entry;
    }
    if (Object.keys(own).length > 0) out['own'] = own;
  }
  if (cls.meta) out['meta'] = cls.meta;
  return out;
}
