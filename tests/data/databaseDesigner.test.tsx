/**
 * A `.dbc` opens as the tree it describes rather than as the table it is stored in.
 *
 * A database container's records are OBJECTID, PARENTID, OBJECTTYPE and OBJECTNAME: a database
 * owns tables, a table owns its fields, indexes and relations. Twenty-six rows of that read as a
 * grid tell you nothing about the database, so the designer assembles it back into a tree.
 */

import { act, render, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeAll, beforeEach, describe, expect, it } from 'vitest';
import { App } from '@renderer/App';
import { setApi } from '@renderer/api/foxdev';
import { createMemoryApi, type MemoryApi } from '@renderer/api/memoryApi';
import { useDocumentsStore } from '@renderer/stores/documentsStore';
import { useProjectStore } from '@renderer/stores/projectStore';
import { openFile } from '@renderer/stores/fileActions';
import { loadFoxVm } from '../../src/wasm/foxvm/loader';
import { buildDbf } from './dbfFixture';

const P = '/proj';
let api: MemoryApi;

/** The shape Visual FoxPro gives a database container. */
const CONTAINER_FIELDS = [
  { name: 'OBJECTID', kind: 'N' as const, width: 6 },
  { name: 'PARENTID', kind: 'N' as const, width: 6 },
  { name: 'OBJECTTYPE', kind: 'C' as const, width: 10 },
  { name: 'OBJECTNAME', kind: 'C' as const, width: 12 },
];

beforeAll(async () => {
  await loadFoxVm();
});

beforeEach(() => {
  api = createMemoryApi();
  setApi(api);
  // a database of two tables, one with two fields and an index
  api.binary$.set(
    `${P}/dvds.dbc`,
    buildDbf(CONTAINER_FIELDS, [
      ['1', '1', 'Database', 'Database'],
      ['2', '1', 'Database', 'TransactionLog'],
      ['6', '1', 'Table', 'dvds'],
      ['7', '1', 'Table', 'dvdprice'],
      ['20', '6', 'Field', 'name'],
      ['21', '6', 'Field', 'id'],
      ['8', '6', 'Index', 'id'],
    ]),
  );
  api.binary$.set(`${P}/dvds.dbf`, buildDbf([{ name: 'NAME', kind: 'C', width: 10 }], [['Alien']]));
  api.files$.set(`${P}/Sample.fxproject`, JSON.stringify({ $schema: 'foxdev-project', version: 1, name: 'Sample', items: [] }));
  useProjectStore.getState().close();
  useDocumentsStore.getState().closeAll();
});

async function openProject() {
  render(<App />);
  await screen.findByTestId('welcome');
  await act(() => useProjectStore.getState().openProject(`${P}/Sample.fxproject`));
}

describe('the database designer', () => {
  it('shows a container as the tree it describes', async () => {
    await openProject();
    await openFile(`${P}/dvds.dbc`);

    const designer = await screen.findByTestId('database-designer');
    expect(designer).toHaveTextContent('2 tables');
    expect(within(designer).getByLabelText('Database Database')).toBeInTheDocument();
    expect(within(designer).getByLabelText('Table dvds')).toBeInTheDocument();
    expect(within(designer).getByLabelText('Table dvdprice')).toBeInTheDocument();

    // the fields belong to the table they name as their parent, not to the database
    expect(within(designer).getByLabelText('Field name')).toBeInTheDocument();
    expect(within(designer).getByLabelText('Index id')).toBeInTheDocument();
  });

  it('leaves out the records the container keeps for its own bookkeeping', async () => {
    await openProject();
    await openFile(`${P}/dvds.dbc`);
    const designer = await screen.findByTestId('database-designer');
    // the transaction log and the stored-procedure records are not objects of the database
    expect(within(designer).queryByLabelText('Database TransactionLog')).toBeNull();
    expect(within(designer).getByLabelText('Database Database')).toBeInTheDocument();
  });

  it('renames an object in place and writes it back', async () => {
    await openProject();
    await openFile(`${P}/dvds.dbc`);
    const designer = await screen.findByTestId('database-designer');

    // a table opens on double-click; everything else is renamed in place
    await userEvent.dblClick(within(designer).getByLabelText('Field name'));

    const box = within(designer).getByRole('textbox', { name: 'Name of Field name' });
    await userEvent.clear(box);
    await userEvent.type(box, 'title{Enter}');

    const bytes = () => {
      let text = '';
      for (const b of api.binary$.get(`${P}/dvds.dbc`)!) text += String.fromCharCode(b);
      return text;
    };
    await waitFor(() => expect(bytes()).toContain('title'));
    expect(await within(designer).findByLabelText('Field title')).toBeInTheDocument();
  });

  it('opens a table of the database in the browser', async () => {
    await openProject();
    await openFile(`${P}/dvds.dbc`);
    const designer = await screen.findByTestId('database-designer');

    await userEvent.dblClick(within(designer).getByLabelText('Table dvds'));
    expect(await screen.findByRole('tab', { name: 'dvds.dbf' })).toBeInTheDocument();
    await waitFor(() => expect(screen.getByTestId('table-browser')).toHaveTextContent('Alien'));
  });

  it('shows the records themselves when asked, and goes back', async () => {
    await openProject();
    await openFile(`${P}/dvds.dbc`);
    const designer = await screen.findByTestId('database-designer');

    await userEvent.click(within(designer).getByRole('button', { name: 'Show records' }));
    const grid = await screen.findByTestId('table-browser');
    expect(within(grid).getByRole('columnheader', { name: 'OBJECTTYPE' })).toBeInTheDocument();

    await userEvent.click(within(grid).getByRole('button', { name: 'Show database' }));
    expect(await screen.findByTestId('database-designer')).toBeInTheDocument();
  });
});
