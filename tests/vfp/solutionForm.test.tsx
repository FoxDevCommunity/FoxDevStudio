/**
 * What the Solutions form has on it once it has opened.
 *
 * `samplesRun.test.ts` records what a form *says* while it opens, which is silent about a form
 * that opens quietly and shows nothing. The Solutions launcher is the form a person meets first
 * and the one every report against this product has come through, so this one asserts what is on
 * it: the tree full of sample categories, the page frame, the buttons.
 *
 * It skips itself when Visual FoxPro is not installed.
 */

import { existsSync, readFileSync, readdirSync, statSync, writeFileSync } from 'node:fs';
import { beforeAll, describe, expect, it } from 'vitest';
import { screen } from '@testing-library/react';
import { RuntimeControl } from '@renderer/runtime/RuntimeControl';
import { renderWithProviders } from '../helpers/render';
import { importFormFile } from '@shared/vfp/importForm';
import { formsetDocumentName } from '@shared/form/formset';
import { parseFormDocument, stringifyFormDocument } from '@shared/form/serialize';
import type { FormDocument } from '@shared/form/schema';
import { baseName, formMethodSources, requireBytes, type ProgramSource } from '@shared/runtime/programSource';
import type { RuntimeObject } from '@shared/runtime/objectModel';
import { TreeView } from '@shared/runtime/oleObjects';
import { createMemoryApi, type MemoryApi } from '@renderer/api/memoryApi';
import { setApi } from '@renderer/api/foxdev';
import { useProjectStore } from '@renderer/stores/projectStore';
import { useSessionStore } from '@renderer/runtime/session';
import { readVfpTable } from '@renderer/vfp/openVfpFile';
import { loadClassLibraries } from '@renderer/vfp/classLibraries';
import { compileForm, compileProgram } from '@renderer/runtime/vmBridge';
import { loadFoxVm } from '../../src/wasm/foxvm/loader';

const SAMPLES = 'C:/Program Files (x86)/Microsoft Visual FoxPro 9/Samples';
const SOLUTION = `${SAMPLES}/Solution`;
const installed = existsSync(`${SOLUTION}/solution.scx`);
const TEXT = new Set(['prg', 'mpr', 'qpr', 'txt', 'h', 'ini', 'dbc', 'log']);

function seed(api: MemoryApi, dir: string, depth = 2): void {
  for (const entry of readdirSync(dir)) {
    const path = `${dir}/${entry}`;
    if (statSync(path).isDirectory()) {
      if (depth > 0) seed(api, path, depth - 1);
      continue;
    }
    const ext = entry.slice(entry.lastIndexOf('.') + 1).toLowerCase();
    if (TEXT.has(ext)) api.files$.set(path, readFileSync(path, 'latin1'));
    else api.binary$.set(path, new Uint8Array(readFileSync(path)));
  }
}

async function documentsOf(path: string): Promise<Map<string, FormDocument>> {
  const table = await readVfpTable(path);
  const dir = path.slice(0, path.lastIndexOf('/'));
  const { libraries } = await loadClassLibraries(table, dir, readVfpTable, [dir, `${dir}/..`, SOLUTION]);
  const stem = path.slice(path.lastIndexOf('/') + 1).replace(/\.[^.]+$/, '');
  const out = new Map<string, FormDocument>();
  for (const [i, entry] of importFormFile(table, stem, libraries).entries()) {
    // the project importer writes down where a form came from, and a form's data environment
    // names its tables relative to that, so the harness records it the same way
    entry.imported.doc.meta = { ...entry.imported.doc.meta, vfp: { ...entry.imported.doc.meta?.vfp, source: path.slice(SOLUTION.length + 1) } };
    const saved = parseFormDocument(stringifyFormDocument(entry.imported.doc));
    if (!saved.ok) throw new Error(saved.error);
    out.set(formsetDocumentName(stem, entry.formName, i).toLowerCase(), saved.doc);
  }
  return out;
}

function sourceOver(docs: Map<string, FormDocument>): ProgramSource {
  return {
    async getForm(name) {
      const doc = docs.get(baseName(name).toLowerCase());
      if (!doc) return null;
      const bytes = requireBytes(doc.form.name, compileForm(doc.form.name, formMethodSources(doc)));
      return { name: doc.form.name, doc, bytes };
    },
    async getProgram(name) {
      const display = baseName(name);
      return { name: display, bytes: requireBytes(display, compileProgram(`? "${display}"`, display)) };
    },
    async getMenu() {
      return null;
    },
  };
}

const settle = (): Promise<unknown> => new Promise((r) => setTimeout(r, 0));
const deadline = (ms: number): Promise<unknown> => new Promise((r) => setTimeout(r, ms));

/** Every object on the form, whatever it is nested inside. */
function walk(obj: RuntimeObject, out: RuntimeObject[] = []): RuntimeObject[] {
  out.push(obj);
  for (const child of obj.children) walk(child, out);
  return out;
}

beforeAll(async () => {
  if (installed) await loadFoxVm();
}, 120_000);

describe.skipIf(!installed)('the Solutions launcher', () => {
  it('opens with its samples in the tree', async () => {
    const api = createMemoryApi();
    seed(api, SOLUTION);
    setApi(api);
    useProjectStore.setState({ path: `${SOLUTION}/solution.fxproject`, doc: null });
    useSessionStore.getState().cancel();
    useSessionStore.setState({ output: [] });

    const docs = await documentsOf(`${SOLUTION}/solution.scx`);
    const first = [...docs.keys()][0]!;
    const started = Date.now();
    await Promise.race([useSessionStore.getState().execute(sourceOver(docs), `DO FORM ${first} NOSHOW`), deadline(8000)]);
    for (let i = 0; i < 400 && useSessionStore.getState().status === 'running' && Date.now() - started < 8000; i++) await settle();

    const desktop = useSessionStore.getState().desktop;
    if (!desktop) throw new Error('no session desktop');
    const form = desktop.forms[0];
    if (!form) throw new Error('the form did not open');
    const objects = walk(form);

    // The tree is filled by the form's own filltree, which SCANs the samples table and calls
    // oTree.Nodes.Add for each row. An empty tree means the form opened and did nothing, which
    // is what a person sees as a broken window - and what no error line ever reported.
    const trees = objects.map((o) => o.ole).filter((o): o is TreeView => o instanceof TreeView);
    expect(trees.length, 'the form has a tree on it').toBeGreaterThan(0);
    const nodes = trees[0]!.nodeList;
    expect(nodes.length, 'sample categories in the tree').toBeGreaterThan(5);
    expect(nodes.map((n) => n.text)).toEqual(expect.arrayContaining(['ActiveX', 'Controls', 'Forms']));

    // ...and then drawn. A tree full of nodes behind a control the renderer never draws is what
    // a person meets as an empty window: OleControl was measured by adding one to a form, which
    // adds it hidden, so every ActiveX control on every form came out Visible .F. and nothing
    // said a word about it. What the model holds and what the screen shows are two assertions.
    renderWithProviders(
      <div>
        {form.children.map((c) => (
          <RuntimeControl key={c.handle} obj={c} />
        ))}
      </div>,
    );
    expect(screen.getByRole('tree'), 'the tree is drawn').toBeInTheDocument();
    expect(screen.getAllByRole('treeitem').length, 'tree rows on screen').toBeGreaterThan(5);
    expect(screen.getByText('ActiveX')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /Close/ })).toBeInTheDocument();

    if (process.env['FORM_DUMP']) {
      const lines: string[] = [];
      for (const o of objects) {
        const props = o.resolved();
        lines.push(
          [
            '  '.repeat(o.path().split('.').length - 1) + o.type + ' ' + o.name,
            'at ' + props['Left'] + ',' + props['Top'] + ' ' + props['Width'] + 'x' + props['Height'],
            'back=' + props['BackColor'] + ' fore=' + props['ForeColor'] + ' style=' + props['BackStyle'],
            'visible=' + props['Visible'],
            props['Caption'] !== undefined ? 'caption=' + JSON.stringify(props['Caption']) : '',
            props['RowSourceType'] !== undefined ? 'rowsource=' + props['RowSourceType'] + ':' + props['RowSource'] + ' cols=' + props['ColumnCount'] + ' widths=' + props['ColumnWidths'] : '',
            o.items.length > 0 ? 'items=' + o.items.length + ' ' + JSON.stringify(o.items.slice(0, 6).map((i) => i.text)) : '',
          ]
            .filter(Boolean)
            .join(' | '),
        );
      }
      writeFileSync(process.env['FORM_DUMP'], lines.join(String.fromCharCode(10)));
    }

    useSessionStore.getState().cancel();
  }, 120_000);
});
