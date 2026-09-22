import { describe, expect, it } from 'vitest';
import { PER_INCH, REPORT_BANDS, bandCode, bandTops, isPerRecordBand, type ReportDocument } from '@shared/report/schema';
import { createEmptyReportDocument, parseReportDocument, stringifyReportDocument } from '@shared/report/serialize';

function sales(): ReportDocument {
  return {
    $schema: 'foxdev-report',
    version: 1,
    kind: 'report',
    name: 'sales',
    page: { width: 8.5 * PER_INCH, height: 11 * PER_INCH, margins: { left: 5_000, right: 5_000, top: 5_000, bottom: 5_000 } },
    groups: [{ expr: 'customer.region', newPage: true, reprintHeader: true }],
    bands: [
      {
        type: 'title',
        height: 3_333,
        objects: [{ id: 't1', type: 'label', top: 0, left: 10_000, width: 20_000, height: 1_667, text: 'Sales by region', font: { name: 'Arial', size: 12, bold: true } }],
      },
      { type: 'pageHeader', height: 1_667, objects: [{ id: 'h1', type: 'line', top: 1_600, left: 0, width: 75_000, height: 0, pen: 100 }] },
      { type: 'groupHeader', height: 1_667, group: 1, objects: [{ id: 'g1', type: 'field', top: 0, left: 0, width: 20_000, height: 1_667, expr: 'customer.region' }] },
      {
        type: 'detail',
        height: 1_667,
        objects: [
          { id: 'd1', type: 'field', top: 0, left: 0, width: 20_000, height: 1_667, expr: 'orders.company', stretch: true },
          { id: 'd2', type: 'field', top: 0, left: 22_000, width: 10_000, height: 1_667, expr: 'orders.total', format: '999,999.99', align: 'right', printWhen: 'orders.total > 0' },
          { id: 'd3', type: 'picture', top: 0, left: 34_000, width: 5_000, height: 1_667, source: 'logo.bmp', sourceType: 'file', mode: 'scaleRetainShape' },
        ],
      },
      {
        type: 'groupFooter',
        height: 1_667,
        group: 1,
        objects: [{ id: 'f1', type: 'field', top: 0, left: 22_000, width: 10_000, height: 1_667, expr: 'orders.total', calculate: 'sum', resetOn: 1 }],
      },
      { type: 'summary', height: 1_667, objects: [{ id: 's1', type: 'rectangle', top: 0, left: 0, width: 75_000, height: 1_667, pen: 100, curve: 500 }] },
    ],
    data: [{ alias: 'orders', source: 'data/orders.dbf', order: 'company' }],
    order: 'customer.region',
  };
}

describe('the report document', () => {
  it('round-trips through text unchanged', () => {
    const doc = sales();
    const parsed = parseReportDocument(stringifyReportDocument(doc));
    expect(parsed.ok).toBe(true);
    if (!parsed.ok) return;
    expect(parsed.doc).toEqual(doc);
    expect(stringifyReportDocument(parsed.doc)).toBe(stringifyReportDocument(doc));
  });

  it('leaves the objects of a band in the order the layout has them', () => {
    const text = stringifyReportDocument(sales());
    expect(text.indexOf('"d1"')).toBeLessThan(text.indexOf('"d2"'));
    expect(text.indexOf('"d2"')).toBeLessThan(text.indexOf('"d3"'));
  });

  it('keeps the tables the report opens', () => {
    const parsed = parseReportDocument(stringifyReportDocument(sales()));
    expect(parsed.ok && parsed.doc.data).toEqual([{ alias: 'orders', source: 'data/orders.dbf', order: 'company' }]);
  });

  it('a new report starts with the bands the designer shows', () => {
    expect(createEmptyReportDocument('Report1').bands.map((b) => b.type)).toEqual(['title', 'pageHeader', 'detail', 'pageFooter']);
  });

  it('a label is the same document with one band and columns across the page', () => {
    const label = createEmptyReportDocument('Labels', 'label');
    expect(label.kind).toBe('label');
    expect(label.bands.map((b) => b.type)).toEqual(['detail']);
    expect(label.page.columns).toBe(3);
    expect(parseReportDocument(stringifyReportDocument(label)).ok).toBe(true);
  });
});

describe('the report shape and the one the VM reads a .frx into', () => {
  // crates/foxvm/src/report.rs numbers the bands in its own Band::of; these are those numbers,
  // and they are the OBJCODE a .frx band record carries. If the two ever part, REPORT FORM
  // would print one shape's summary where the other's page footer goes.
  it('numbers the bands the way the run-time reader does', () => {
    expect(REPORT_BANDS.map(bandCode)).toEqual([0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10]);
    expect(bandCode('title')).toBe(0);
    expect(bandCode('detail')).toBe(4);
    expect(bandCode('summary')).toBe(8);
    expect(bandCode('detailFooter')).toBe(10);
  });

  it('agrees on which bands print once for every record', () => {
    expect(REPORT_BANDS.filter(isPerRecordBand)).toEqual(['detail', 'detailHeader', 'detailFooter']);
  });

  it('stacks the bands down the layout with the designer bar between them, as a .frx does', () => {
    const doc = sales();
    const bar = PER_INCH / 4.8;
    expect(bandTops(doc)).toEqual([
      0,
      3_333 + bar,
      3_333 + bar + 1_667 + bar,
      3_333 + bar + 1_667 + bar + 1_667 + bar,
      3_333 + bar + 1_667 + bar + 1_667 + bar + 1_667 + bar,
      3_333 + bar + 1_667 + bar + 1_667 + bar + 1_667 + bar + 1_667 + bar,
    ]);
  });
});

describe('a malformed report', () => {
  it('says what is wrong with the JSON', () => {
    const result = parseReportDocument('{ bands: [] }');
    expect(result.ok).toBe(false);
    if (!result.ok) expect(result.error).toMatch(/^Invalid JSON: /);
  });

  it('names the band it cannot read', () => {
    const doc = sales() as unknown as { bands: { type: string }[] };
    doc.bands[2]!.type = 'groupTotal';
    const result = parseReportDocument(JSON.stringify(doc));
    expect(result.ok).toBe(false);
    if (!result.ok) expect(result.error).toContain('bands.2.type');
  });

  it('names the object it cannot read', () => {
    const doc = sales() as unknown as { bands: { objects: { type: string }[] }[] };
    doc.bands[3]!.objects[1]!.type = 'barcode';
    const result = parseReportDocument(JSON.stringify(doc));
    expect(result.ok).toBe(false);
    if (!result.ok) expect(result.error).toContain('bands.3.objects.1.type');
  });

  it('refuses a page with no size, because a report has to print somewhere', () => {
    const doc = sales() as unknown as { page: { width: number } };
    doc.page.width = 0;
    const result = parseReportDocument(JSON.stringify(doc));
    expect(result.ok).toBe(false);
    if (!result.ok) expect(result.error).toContain('page.width');
  });

  it('refuses a document of another kind', () => {
    const result = parseReportDocument(JSON.stringify({ ...sales(), $schema: 'foxdev-form' }));
    expect(result.ok).toBe(false);
    if (!result.ok) expect(result.error).toContain('$schema');
  });
});
