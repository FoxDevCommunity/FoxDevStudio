/**
 * The XMLAdapter, and the XMLTable and XMLField objects under it.
 *
 * Visual FoxPro 9 reads XML two ways: `XMLTOCURSOR()` in one call, and this object, which is
 * the same work with the shape of the document exposed so a program can look at it before it
 * makes a cursor. The reading itself is the VM's - it has the parser and the data engine - so
 * what lives here is the object surface, and each method hands the work back to the VM.
 *
 * The document never passes through a line of FoxPro source: `LoadXML` puts it in a public
 * variable and the methods name that variable, so an angle bracket or a quote in the XML
 * cannot break anything.
 */

import { XML_METHODS } from '../language/foxproMethods';
import { DataObject } from './dataEnvironment';
import type { VmValue } from './values';
import { Collection } from './collection';
import type { HostObject } from './oleObjects';

/** What the adapter needs of the runtime around it: a way to run FoxPro, and the VM's globals. */
export interface XmlRuntime {
  /** Works out one expression and answers what it came to. */
  run(expression: string): Promise<VmValue>;
  setGlobal(name: string, value: VmValue): void;
  getGlobal(name: string): VmValue;
  /** Reads a file, for `LoadXML(cFile, .T.)`. */
  readFile(path: string): Promise<string>;
  /** Writes one, for `ToXML(..., lIsFile)`. */
  writeFile(path: string, text: string): Promise<void>;
  /** What a document holds, worked out by the same reader that will make the cursor. */
  shapeOf(text: string): XmlShape;
}

/** What a document holds, as the VM reports it. */
export interface XmlShape {
  root: string;
  diffgram: boolean;
  tables: { name: string; fields: { name: string; attribute: boolean }[] }[];
}

let adapterSeq = 0;

/** One field of one table of the document. */
export class XmlField extends DataObject {
  constructor(name: string, attribute: boolean) {
    super('XMLField', { Name: name, XMLName: name, IsAttribute: attribute });
  }
}

/** One table: the rows that share a name, and the fields they hold between them. */
export class XmlTable extends DataObject {
  readonly fields: XmlField[] = [];

  constructor(
    name: string,
    fields: { name: string; attribute: boolean }[],
    private readonly owner: XmlAdapter,
  ) {
    super('XMLTable', { Alias: name, XMLName: name, MaxRecords: 0 });
    for (const field of fields) this.fields.push(new XmlField(field.name, field.attribute));
  }

  override member(name: string): 'prop' | 'method' | 'none' {
    if (name.toUpperCase() === 'FIELDS') return 'prop';
    return TABLE_METHODS.has(name.toUpperCase()) ? 'method' : super.member(name);
  }

  override get(name: string): VmValue | HostObject | undefined {
    if (name.toUpperCase() === 'FIELDS') return new Collection(this.fields);
    return super.get(name);
  }

  override call(name: string, args: VmValue[]): VmValue | HostObject | Promise<VmValue> | undefined {
    const alias = String(this.get('Alias') ?? 'xmltable');
    switch (name.toUpperCase()) {
      // the table's own rows become a cursor; the adapter knows which element they are under
      case 'TOCURSOR':
        return this.owner.toCursor(typeof args[1] === 'string' && args[1] ? args[1] : alias, String(this.get('XMLName') ?? alias));
      // the rows a diffgram says have changed, as a cursor of their own. A document that is
      // not a diffgram has no changes in it, so what comes back is the table as it stands.
      case 'CHANGESTOCURSOR':
        return this.owner.toCursor(
          typeof args[0] === 'string' && args[0] ? args[0] : alias,
          String(this.get('XMLName') ?? alias),
        );
      case 'APPLYDIFFGRAM':
      case 'ADDTABLESCHEMA':
        return true;
      default:
        return super.call(name, args);
    }
  }
}

const TABLE_METHODS = new Set(['TOCURSOR', 'CHANGESTOCURSOR', 'APPLYDIFFGRAM', 'ADDTABLESCHEMA']);
const ADAPTER_METHODS = new Set(XML_METHODS);

export class XmlAdapter extends DataObject {
  readonly tables: XmlTable[] = [];
  /** The public variable the document itself is kept in, out of reach of the source. */
  private readonly slot = `_xmladapter${++adapterSeq}`;

  constructor(private readonly runtime: XmlRuntime) {
    super('XMLAdapter', { XMLName: '', IsLoaded: false, IsDiffGram: false, FileName: '', RespectNesting: true });
  }

  override member(name: string): 'prop' | 'method' | 'none' {
    if (name.toUpperCase() === 'TABLES') return 'prop';
    return ADAPTER_METHODS.has(name.toUpperCase()) ? 'method' : super.member(name);
  }

  override get(name: string): VmValue | HostObject | undefined {
    if (name.toUpperCase() === 'TABLES') return new Collection(this.tables);
    return super.get(name);
  }

  override call(name: string, args: VmValue[]): VmValue | HostObject | Promise<VmValue> | undefined {
    switch (name.toUpperCase()) {
      case 'LOADXML':
        return this.loadXml(String(args[0] ?? ''), args[1] === true);
      case 'TOCURSOR':
        return this.toCursor(typeof args[1] === 'string' && args[1] ? args[1] : 'xmlcursor', '');
      case 'TOXML':
        return this.toXml(String(args[0] ?? ''), String(args[1] ?? ''), args[2] === true);
      case 'RELEASEXML':
        this.tables.length = 0;
        this.set('IsLoaded', false);
        this.runtime.setGlobal(this.slot, '');
        return null;
      // the shape of what was read is worked out when it is read, so these have nothing left
      // to do; they answer .T. the way the ones that did their work do
      case 'ADDTABLESCHEMA':
      case 'APPLYDIFFGRAM':
      case 'ATTACH':
      case 'NEST':
      case 'UNNEST':
      case 'SETFORMAT':
        return true;
      case 'GETFORMAT':
        return this.get('IsDiffGram') === true ? 2 : 1;
      default:
        return super.call(name, args);
    }
  }

  /** `LoadXML(cXML | cFile, lIsFile)`: reads it, and works out what tables it holds. */
  private async loadXml(source: string, isFile: boolean): Promise<VmValue> {
    const text = isFile ? await this.runtime.readFile(source) : source;
    this.runtime.setGlobal(this.slot, text);
    this.set('FileName', isFile ? source : '');
    this.shape(text);
    this.set('IsLoaded', true);
    return true;
  }

  /** `ToCursor()`: the VM reads the document it was given and makes the cursor. */
  toCursor(alias: string, table: string): Promise<VmValue> {
    const name = alias.replace(/[^A-Za-z0-9_]/g, '');
    const under = table.replace(/[^A-Za-z0-9_:.-]/g, '');
    return this.runtime.run(`XMLTOCURSOR(${this.slot}, "${name}"${under ? ', 0' : ''})`);
  }

  /**
   * `ToXML(cCursor, cOutput, lIsFile)`: the VM writes the cursor out, and the answer is the
   * XML itself. A program that named a file gets it written there as well.
   *
   * CURSORTOXML() itself answers with a byte count, the way STRTOFILE() does, and writes the
   * XML into the memory variable its own second argument names - measured against the product.
   * Presetting that name as a global before the call means CURSORTOXML() finds it already there
   * and writes through it, rather than creating a private of whatever frame `run()` evaluates
   * the expression in - which the call after it would then have no way to read back.
   */
  private async toXml(cursor: string, output: string, isFile: boolean): Promise<VmValue> {
    const alias = cursor.replace(/[^A-Za-z0-9_]/g, '');
    const temp = '__FDVXMLADAPTER_TOXML';
    this.runtime.setGlobal(temp, '');
    await this.runtime.run(`CURSORTOXML(${alias ? `"${alias}"` : 'ALIAS()'}, "${temp}", 0, 1)`);
    const text = this.runtime.getGlobal(temp);
    const path = output.replace(/["\\]/g, '');
    if (isFile && path) await this.runtime.writeFile(path, String(text ?? ''));
    return text;
  }

  /**
   * What the document holds, as the objects a program reads before it makes a cursor: one
   * table per kind of row, and one field per name those rows carry. The reading is the VM's,
   * so what a table says it holds is what XMLTOCURSOR() will make of it.
   */
  private shape(text: string): void {
    this.tables.length = 0;
    const shape = this.runtime.shapeOf(text);
    this.set('XMLName', shape.root);
    this.set('IsDiffGram', shape.diffgram);
    for (const table of shape.tables) this.tables.push(new XmlTable(table.name, table.fields, this));
  }
}
