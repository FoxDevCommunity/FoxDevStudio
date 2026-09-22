/**
 * A report or a label (`.fxr`, `.fxl`): the bands, what is drawn in each, the tables it opens and
 * the page it prints on.
 *
 * This is the same report a `.frx` describes, said in our own words. `crates/foxvm/src/report.rs`
 * reads a `.frx` at run time, and the two shapes are deliberately the same shape, so `REPORT FORM`
 * over either one ends up with the same thing:
 *
 * - Bands are listed in the order they print, and `REPORT_BANDS` is in the order the file's own
 *   `OBJCODE` numbers them - `REPORT_BANDS.indexOf(band.type)` *is* that number.
 * - Everything is measured in ten-thousandths of an inch, which is what a report file measures in.
 * - An object's place is measured from the top left of the band it is in, not of the layout. A
 *   `.frx` writes absolute positions and the reader works out which band each falls in; here the
 *   band already owns its objects, which is the same information without the arithmetic.
 *
 * A label is this document with one detail band and several columns across the page: the same
 * format, because that is what Visual FoxPro's `.lbx` is - a `.frx` with columns.
 */

import { z } from 'zod';
import { documentMetaSchema, formCursorSchema } from '../form/schema';
import type { DocumentMeta, FormCursor } from '../form/schema';

export const REPORT_SCHEMA_ID = 'foxdev-report' as const;
export const REPORT_VERSION = 1 as const;

/** A ten-thousandth of an inch, which is what a report measures in. */
export const PER_INCH = 10_000;

/**
 * The bands, in the order Visual FoxPro numbers them. The position in this list is the `OBJCODE`
 * a `.frx` band record carries, so nothing has to hold a second table of the same numbers.
 */
export const REPORT_BANDS = [
  'title',
  'pageHeader',
  'columnHeader',
  'groupHeader',
  'detail',
  'groupFooter',
  'columnFooter',
  'pageFooter',
  'summary',
  'detailHeader',
  'detailFooter',
] as const;
export type ReportBandType = (typeof REPORT_BANDS)[number];

/** What a band can hold. A line and a rectangle are drawn; the rest print something. */
export const REPORT_OBJECT_TYPES = ['label', 'field', 'line', 'rectangle', 'picture'] as const;
export type ReportObjectType = (typeof REPORT_OBJECT_TYPES)[number];

/** What a field accumulates down the band it repeats in. */
export const REPORT_CALCULATIONS = ['count', 'sum', 'average', 'lowest', 'highest', 'stdDev', 'variance'] as const;
export type ReportCalculation = (typeof REPORT_CALCULATIONS)[number];

/** Where a picture's file comes from: a path, an expression, or a General field of the record. */
export const PICTURE_SOURCES = ['file', 'expression', 'field'] as const;
export type PictureSource = (typeof PICTURE_SOURCES)[number];

/** How a picture fills the place it was given, in the report designer's own three words. */
export const PICTURE_MODES = ['clip', 'scaleRetainShape', 'scaleFill'] as const;
export type PictureMode = (typeof PICTURE_MODES)[number];

export interface ReportFont {
  name?: string;
  /** In points, as a font is asked for everywhere else. */
  size?: number;
  bold?: boolean;
  italic?: boolean;
  underline?: boolean;
}

export interface ReportObject {
  /** Designer-stable id (nanoid), as a control has. Never printed; identity for the designer. */
  id: string;
  type: ReportObjectType;
  /** Where it sits in its band, and how big it is, in ten-thousandths of an inch. */
  top: number;
  left: number;
  width: number;
  height: number;
  /** A label's text, printed exactly as written. */
  text?: string;
  /** A field's expression, worked out against the record the pointer is on. */
  expr?: string;
  /** The PICTURE clause the value is formatted through; the designer calls this Format. */
  format?: string;
  /** Print When: the object is drawn only when this is true, which is how one report prints three ways. */
  printWhen?: string;
  /** A total rather than a value: what it accumulates, and when it starts again. */
  calculate?: ReportCalculation;
  /**
   * Where a total goes back to zero: the report, the page, the column, or a group by its
   * 1-based level. A total with no reset runs the whole report.
   */
  resetOn?: 'report' | 'page' | 'column' | number;
  /** A picture's file, or the expression or field that names one. */
  source?: string;
  sourceType?: PictureSource;
  mode?: PictureMode;
  /** How thick a line or a rectangle is drawn, in ten-thousandths of an inch. */
  pen?: number;
  /** The radius of a rounded rectangle's corners. Zero, or absent, is a square corner. */
  curve?: number;
  font?: ReportFont;
  align?: 'left' | 'center' | 'right';
  /** The object grows down to fit its value, which is what a memo field needs to print at all. */
  stretch?: boolean;
}

export interface ReportBand {
  type: ReportBandType;
  /** How tall the band is, in ten-thousandths of an inch. */
  height: number;
  /** Which group this band belongs to, 1-based, for the header and footer each group has. */
  group?: number;
  objects: ReportObject[];
}

export interface ReportGroup {
  /** The expression whose change starts a new group. */
  expr: string;
  /** Start the group on a fresh page. */
  newPage?: boolean;
  /** Print the group header again at the top of each page the group runs onto. */
  reprintHeader?: boolean;
}

export interface ReportPage {
  /** The paper, in ten-thousandths of an inch: 85000 by 110000 is US Letter. */
  width: number;
  height: number;
  margins: { left: number; right: number; top: number; bottom: number };
  /** Columns across the page: one for an ordinary report, more for labels. */
  columns?: number;
  /** The gap between two columns. */
  columnSpacing?: number;
  landscape?: boolean;
}

export interface ReportDocument {
  $schema: 'foxdev-report';
  version: 1;
  /** Which of the two commands prints it: `REPORT FORM` or `LABEL FORM`. */
  kind: 'report' | 'label';
  name: string;
  page: ReportPage;
  /** The groups, outermost first. A band's `group` is a 1-based index into this. */
  groups?: ReportGroup[];
  /** The bands, in the order they print. */
  bands: ReportBand[];
  /** The tables the report opens for itself, exactly as a form's data environment does. */
  data?: FormCursor[];
  /** The expression the records are read in - the file's own ORDER. */
  order?: string;
  meta?: DocumentMeta;
}

/** US Letter with half-inch margins: what a new report starts on until it is told otherwise. */
export const LETTER_PAGE: ReportPage = {
  width: 8.5 * PER_INCH,
  height: 11 * PER_INCH,
  margins: { left: PER_INCH / 2, right: PER_INCH / 2, top: PER_INCH / 2, bottom: PER_INCH / 2 },
};

export const reportFontSchema: z.ZodType<ReportFont> = z.object({
  name: z.string().optional(),
  size: z.number().optional(),
  bold: z.boolean().optional(),
  italic: z.boolean().optional(),
  underline: z.boolean().optional(),
});

export const reportObjectSchema: z.ZodType<ReportObject> = z.object({
  id: z.string().min(1),
  type: z.enum(REPORT_OBJECT_TYPES),
  top: z.number(),
  left: z.number(),
  width: z.number(),
  height: z.number(),
  text: z.string().optional(),
  expr: z.string().optional(),
  format: z.string().optional(),
  printWhen: z.string().optional(),
  calculate: z.enum(REPORT_CALCULATIONS).optional(),
  resetOn: z.union([z.literal('report'), z.literal('page'), z.literal('column'), z.number().int().positive()]).optional(),
  source: z.string().optional(),
  sourceType: z.enum(PICTURE_SOURCES).optional(),
  mode: z.enum(PICTURE_MODES).optional(),
  pen: z.number().optional(),
  curve: z.number().optional(),
  font: reportFontSchema.optional(),
  align: z.enum(['left', 'center', 'right']).optional(),
  stretch: z.boolean().optional(),
});

export const reportBandSchema: z.ZodType<ReportBand> = z.object({
  type: z.enum(REPORT_BANDS),
  height: z.number().min(0),
  group: z.number().int().positive().optional(),
  objects: z.array(reportObjectSchema),
});

export const reportGroupSchema: z.ZodType<ReportGroup> = z.object({
  expr: z.string(),
  newPage: z.boolean().optional(),
  reprintHeader: z.boolean().optional(),
});

export const reportPageSchema: z.ZodType<ReportPage> = z.object({
  width: z.number().positive(),
  height: z.number().positive(),
  margins: z.object({ left: z.number(), right: z.number(), top: z.number(), bottom: z.number() }),
  columns: z.number().int().positive().optional(),
  columnSpacing: z.number().optional(),
  landscape: z.boolean().optional(),
});

export const reportDocumentSchema: z.ZodType<ReportDocument> = z.object({
  $schema: z.literal(REPORT_SCHEMA_ID),
  version: z.literal(REPORT_VERSION),
  kind: z.enum(['report', 'label']),
  name: z.string().min(1),
  page: reportPageSchema,
  groups: z.array(reportGroupSchema).optional(),
  bands: z.array(reportBandSchema),
  data: z.array(formCursorSchema).optional(),
  order: z.string().optional(),
  meta: documentMetaSchema.optional(),
});

/** The `OBJCODE` a `.frx` gives this band, which is how the run-time reader names its bands. */
export function bandCode(band: ReportBandType): number {
  return REPORT_BANDS.indexOf(band);
}

/** Whether the band prints once for every record, rather than once for the report or the page. */
export function isPerRecordBand(band: ReportBandType): boolean {
  return band === 'detail' || band === 'detailHeader' || band === 'detailFooter';
}

/**
 * Where each band starts down the layout, in ten-thousandths of an inch.
 *
 * A `.frx` stores this rather than the band an object is in, with the designer's separator bar
 * between one band and the next, so anything that has to write one - or read our document as if
 * it were one - works it out the same way here.
 */
export const BAND_BAR = PER_INCH / 4.8;

export function bandTops(doc: ReportDocument): number[] {
  const tops: number[] = [];
  let top = 0;
  for (const band of doc.bands) {
    tops.push(top);
    top += band.height + BAND_BAR;
  }
  return tops;
}
