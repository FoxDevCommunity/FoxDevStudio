/**
 * The table browser, against a real DBF and its memo file.
 *
 * A `.dbf` or `.dbc` used to open in the code editor, which showed the bytes as text. This asserts
 * it opens as data instead, reads through the data engine, and shows what BROWSE shows: field
 * names, record numbers, deleted records struck out, and Memo for a memo field until it is opened.
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
import { buildDbf, buildFpt } from './dbfFixture';

/** The file as one character per byte, for asserting on what was written. */
function latin1(bytes: Uint8Array): string {
  let out = '';
  for (const b of bytes) out += String.fromCharCode(b);
  return out;
}

const P = '/proj';
let api: MemoryApi;

beforeAll(async () => {
  await loadFoxVm();
});

beforeEach(() => {
  api = createMemoryApi();
  setApi(api);
  api.binary$.set(
    `${P}/dvds.dbf`,
    buildDbf(
      [
        { name: 'OBJECTNAME', kind: 'C', width: 10 },
        { name: 'PRICE', kind: 'N', width: 8, decimals: 2 },
        { name: 'INSTOCK', kind: 'L', width: 1 },
        { name: 'CODE', kind: 'M', width: 4 },
      ],
      [
        ['Table', '9.99', 'T', '1'],
        ['View', '12.50', 'F', '0'],
        ['Index', '0.00', 'T', '0'],
      ],
      [2],
    ),
  );
  api.binary$.set(`${P}/dvds.fpt`, buildFpt(['CREATE TABLE dvds']));
  // the browser lives in the document area, which only exists once a project is open
  api.files$.set(`${P}/Sample.fxproject`, JSON.stringify({ $schema: 'foxdev-project', version: 1, name: 'Sample', items: [] }));
  useProjectStore.getState().close();
  useDocumentsStore.getState().closeAll();
});

async function openProject() {
  render(<App />);
  await screen.findByTestId('welcome');
  await act(() => useProjectStore.getState().openProject(`${P}/Sample.fxproject`));
}

describe('the table browser', () => {
  it('opens a DBF-based file as data rather than as text', async () => {
    await openProject();
    await openFile(`${P}/dvds.dbf`);

    const browser = await screen.findByTestId('table-browser');
    expect(screen.getByRole('tab', { name: 'dvds.dbf' })).toBeInTheDocument();
    expect(browser).toHaveTextContent('4 fields, 3 records');

    // the fields, from the header
    for (const name of ['OBJECTNAME', 'PRICE', 'INSTOCK', 'CODE']) {
      expect(within(browser).getByRole('columnheader', { name })).toBeInTheDocument();
    }

    // the records, decoded from a page the data engine read
    await waitFor(() => expect(within(browser).getByText('Table')).toBeInTheDocument());
    expect(within(browser).getByText('9.99')).toBeInTheDocument();
    expect(within(browser).getAllByText('.T.')).toHaveLength(2);
    expect(within(browser).getByText('.F.')).toBeInTheDocument();

    // the second record is deleted, which BROWSE shows rather than hides
    expect(within(browser).getByText('View').closest('tr')).toHaveClass('fx-browse__deleted');
  });

  it('edits a cell in place and writes it back to the file', async () => {
    await openProject();
    await openFile(`${P}/dvds.dbf`);
    const browser = await screen.findByTestId('table-browser');
    await waitFor(() => expect(within(browser).getByText('Table')).toBeInTheDocument());

    await userEvent.dblClick(within(browser).getByText('Table'));
    const box = within(browser).getByRole('textbox', { name: 'OBJECTNAME of record 1' });
    await userEvent.clear(box);
    await userEvent.type(box, 'Cursor{Enter}');

    // the grid re-reads the page, so what it shows is what is in the file
    expect(await within(browser).findByText('Cursor')).toBeInTheDocument();
    const bytes = api.binary$.get(`${P}/dvds.dbf`)!;
    expect(latin1(bytes).includes('Cursor')).toBe(true);
  });

  it('refuses a value the field cannot hold, and says why', async () => {
    await openProject();
    await openFile(`${P}/dvds.dbf`);
    const browser = await screen.findByTestId('table-browser');
    await waitFor(() => expect(within(browser).getByText('9.99')).toBeInTheDocument());

    await userEvent.dblClick(within(browser).getByText('9.99'));
    const box = within(browser).getByRole('textbox', { name: 'PRICE of record 1' });
    await userEvent.clear(box);
    await userEvent.type(box, 'lots{Enter}');
    expect(await within(browser).findByRole('alert')).toHaveTextContent('not a number');
  });

  it('marks a record deleted and brings it back', async () => {
    await openProject();
    await openFile(`${P}/dvds.dbf`);
    const browser = await screen.findByTestId('table-browser');
    await waitFor(() => expect(within(browser).getByText('Table')).toBeInTheDocument());

    const row = () => within(browser).getByText('Table').closest('tr')!;
    const marker = () => within(browser).getByRole('button', { name: /record 1$/ });
    expect(row()).not.toHaveClass('fx-browse__deleted');
    expect(marker()).toHaveAccessibleName('Delete record 1');

    await userEvent.click(marker());
    await waitFor(() => expect(row()).toHaveClass('fx-browse__deleted'));
    expect(marker()).toHaveAccessibleName('Recall record 1');

    await userEvent.click(marker());
    await waitFor(() => expect(row()).not.toHaveClass('fx-browse__deleted'));
  });

  it('shows Memo for a memo field and fetches the text when it is opened', async () => {
    await openProject();
    await openFile(`${P}/dvds.dbf`);
    const browser = await screen.findByTestId('table-browser');

    // one record points at a block, the other two at nothing
    await waitFor(() => expect(within(browser).getAllByRole('button', { name: 'Memo' })).toHaveLength(1));
    await userEvent.click(within(browser).getByRole('button', { name: 'Memo' }));
    expect(await within(browser).findByText('CREATE TABLE dvds')).toBeInTheDocument();
  });
});
