import { formatIssues } from '../form/serialize';
import type { ParseResult } from '../form/schema';
import { LETTER_PAGE, REPORT_SCHEMA_ID, REPORT_VERSION, reportDocumentSchema } from './schema';
import type { ReportBand, ReportDocument, ReportObject } from './schema';

/**
 * A new report: a title, a page header, a detail band and a page footer, which is what the
 * designer puts on the layout before anything is dropped on it. A label starts with the one band
 * it prints, because that is the whole of a label.
 */
export function createEmptyReportDocument(name = 'Report1', kind: 'report' | 'label' = 'report'): ReportDocument {
  const bands: ReportBand[] =
    kind === 'label'
      ? [{ type: 'detail', height: 10_000, objects: [] }]
      : [
          { type: 'title', height: 3_333, objects: [] },
          { type: 'pageHeader', height: 3_333, objects: [] },
          { type: 'detail', height: 1_667, objects: [] },
          { type: 'pageFooter', height: 1_667, objects: [] },
        ];
  return {
    $schema: REPORT_SCHEMA_ID,
    version: REPORT_VERSION,
    kind,
    name,
    page: { ...LETTER_PAGE, margins: { ...LETTER_PAGE.margins }, ...(kind === 'label' ? { columns: 3 } : {}) },
    bands,
  };
}

export function parseReportDocument(text: string): ParseResult<ReportDocument> {
  let raw: unknown;
  try {
    raw = JSON.parse(text);
  } catch (e) {
    return { ok: false, error: `Invalid JSON: ${(e as Error).message}` };
  }
  const result = reportDocumentSchema.safeParse(migrateReportDocument(raw));
  if (!result.success) return { ok: false, error: formatIssues(result.error.issues) };
  return { ok: true, doc: result.data };
}

/** Upgrades older on-disk versions in place. Only version 1 exists today. */
export function migrateReportDocument(raw: unknown): unknown {
  return raw;
}

/**
 * Stable 2-space JSON: a fixed key order and objects left in the order the band holds them, which
 * is the order they were laid out. Nothing is sorted, because in a report the order is the layout.
 */
export function stringifyReportDocument(doc: ReportDocument): string {
  const out: Record<string, unknown> = {
    $schema: doc.$schema,
    version: doc.version,
    kind: doc.kind,
    name: doc.name,
    page: canonicalPage(doc),
  };
  if (doc.groups?.length) out['groups'] = doc.groups;
  out['bands'] = doc.bands.map(canonicalBand);
  // the tables the report opens: a report whose data environment is lost prints an empty page
  if (doc.data?.length) out['data'] = doc.data;
  if (doc.order) out['order'] = doc.order;
  if (doc.meta) out['meta'] = doc.meta;
  return JSON.stringify(out, null, 2) + '\n';
}

function canonicalPage(doc: ReportDocument): Record<string, unknown> {
  const { width, height, margins, columns, columnSpacing, landscape } = doc.page;
  const out: Record<string, unknown> = {
    width,
    height,
    margins: { left: margins.left, right: margins.right, top: margins.top, bottom: margins.bottom },
  };
  if (columns !== undefined) out['columns'] = columns;
  if (columnSpacing !== undefined) out['columnSpacing'] = columnSpacing;
  if (landscape) out['landscape'] = landscape;
  return out;
}

function canonicalBand(band: ReportBand): Record<string, unknown> {
  const out: Record<string, unknown> = { type: band.type, height: band.height };
  if (band.group !== undefined) out['group'] = band.group;
  out['objects'] = band.objects.map(canonicalObject);
  return out;
}

/** The keys of an object, in a fixed order: what it is, where it is, then what it prints. */
const OBJECT_KEYS: (keyof ReportObject)[] = [
  'id',
  'type',
  'top',
  'left',
  'width',
  'height',
  'text',
  'expr',
  'format',
  'printWhen',
  'calculate',
  'resetOn',
  'source',
  'sourceType',
  'mode',
  'pen',
  'curve',
  'font',
  'align',
  'stretch',
];

function canonicalObject(object: ReportObject): Record<string, unknown> {
  const out: Record<string, unknown> = {};
  for (const key of OBJECT_KEYS) {
    const value = object[key];
    if (value !== undefined) out[key] = value;
  }
  return out;
}
