import type { ObjectDescriptor } from '../types';
import * as c from '../common';

/**
 * The ReportListener: the object `REPORT FORM ... OBJECT` hands the report to, band by band.
 *
 * Visual FoxPro 9 took the report engine apart and put a listener in front of it. The engine
 * still reads the `.frx` and works out where everything goes; the listener is told what is
 * about to be printed and may change it, count it, draw it somewhere else, or stop the report.
 * A program subclasses it, so what matters here is the surface it inherits.
 */
export const reportlistener: ObjectDescriptor = {
  displayName: 'ReportListener',
  baseClass: 'reportlistener',
  defaultEvent: 'BeforeReport',
  properties: c.props(
    c.prop('CommandClauses', 'text', 'Data', '', { readOnly: true, description: 'The clauses the REPORT FORM command was given, as an object.' }),
    c.prop('CurrentPass', 'number', 'Data', 0, { readOnly: true, description: 'Which pass over the records this is, when the report takes two.' }),
    c.prop('TwoPassProcess', 'boolean', 'Data', false, { description: 'Whether the report is run twice, so totals are known before they are printed.' }),
    c.prop('FRXDataSession', 'number', 'Data', 0, { readOnly: true, description: 'The data session the report file itself is open in.' }),
    c.prop('ListenerType', 'number', 'Data', -1, { description: 'What the listener is for: 0 page preview, 1 printer, 2 nothing, 3 XML.' }),
    c.prop('OutputType', 'number', 'Data', -1, { description: 'Where the report goes, as the OBJECT clause set it.' }),
    c.prop('OutputPageCount', 'number', 'Data', 0, { readOnly: true, description: 'How many pages have been produced so far.' }),
    c.prop('PageNo', 'number', 'Data', 0, { readOnly: true, description: 'The page being printed.' }),
    c.prop('PageTotal', 'number', 'Data', 0, { readOnly: true, description: 'How many pages the report has in all, once it is known.' }),
    c.prop('PageHeight', 'number', 'Layout', 0, { readOnly: true, description: 'The height of a page, in the units the report uses.' }),
    c.prop('PageWidth', 'number', 'Layout', 0, { readOnly: true, description: 'And its width.' }),
    c.prop('PrintJobName', 'text', 'Other', '', { description: 'What the job is called where jobs are listed.' }),
    c.prop('PreviewContainer', 'text', 'Other', '', { description: 'The object that shows the preview, when there is one.' }),
    c.prop('QuietMode', 'boolean', 'Other', false, { description: 'Whether the report may put a dialog on the screen while it runs.' }),
    c.prop('AllowModalMessages', 'boolean', 'Other', false, { description: 'Whether a message the report raises may be modal.' }),
    c.prop('GDIPlusGraphics', 'text', 'Other', '', { readOnly: true, description: 'The surface the page is drawn on, where the host draws with GDI+.' }),
    c.prop('SendGDIPlusImage', 'boolean', 'Other', false, { description: 'Whether the page is handed over as an image rather than as instructions.' }),
    c.prop('DynamicLineHeight', 'boolean', 'Layout', true, { description: 'Whether a band grows to fit what is put in it.' }),
  ),
  events: c.events([
    c.ev('LoadReport', '', 'The report file has been read and is about to run.'),
    c.ev('UnloadReport', '', 'The report has finished with the file.'),
    c.ev('BeforeReport', '', 'The report is about to start.'),
    c.ev('AfterReport', '', 'It has finished.'),
    c.ev('BeforeBand', 'nBandObjCode, nFRXRecNo', 'A band is about to be printed.'),
    c.ev('AfterBand', 'nBandObjCode, nFRXRecNo', 'It has been printed.'),
    c.ev('EvaluateContents', 'nFRXRecno, oObjProperties', 'One object of a band has been worked out and may be changed.'),
    c.ev('AdjustObjectSize', 'nFRXRecno, oObjProperties', 'An object that stretches has been measured and may be resized.'),
  ]),
};
