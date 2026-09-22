/**
 * The XMLAdapter end to end: `CREATEOBJECT("XMLAdapter")` through to a cursor with records in it.
 *
 * This is the shape of Visual FoxPro's own `1_XMLAdapter_LoadXML.prg` sample - read a file, look
 * at what it holds, make a cursor of it - so what passes here is what that program needs.
 */

import { beforeAll, beforeEach, describe, expect, it } from 'vitest';
import { setApi } from '@renderer/api/foxdev';
import { createMemoryApi, type MemoryApi } from '@renderer/api/memoryApi';
import { useSessionStore } from '@renderer/runtime/session';
import { compileProgram } from '@renderer/runtime/vmBridge';
import { requireBytes, type ProgramSource } from '@shared/runtime/programSource';
import { loadFoxVm } from '../../src/wasm/foxvm/loader';
import { createElement } from 'react';
import { render, screen } from '@testing-library/react';
import { FluentProvider, webLightTheme } from '@fluentui/react-components';
import { RuntimeDesktopDocument } from '@renderer/runtime/RuntimeDesktop';

const P = '/proj';

const CUSTOMER_XML = [
  '<?xml version="1.0" encoding="utf-8"?>',
  '<VFPData>',
  '<customer><cust_id>ALFKI</cust_id><company>Alfreds Futterkiste</company><orders>12</orders></customer>',
  '<customer><cust_id>BERGS</cust_id><company>Berglunds snabbkop</company><orders>7</orders></customer>',
  '</VFPData>',
].join('\r\n');

let program = '';
const source: ProgramSource = {
  getForm: async () => null,
  getMenu: async () => null,
  getProgram: async () => ({ name: 'test', bytes: requireBytes('test', compileProgram(program, 'test')) }),
};

let api: MemoryApi;

beforeAll(async () => {
  await loadFoxVm();
});

beforeEach(() => {
  api = createMemoryApi();
  setApi(api);
  api.binary$.set(`${P}/CustomerXML.xml`, new TextEncoder().encode(CUSTOMER_XML));
  useSessionStore.getState().cancel();
  useSessionStore.setState({ output: [] });
});

async function run(src: string): Promise<string[]> {
  program = src;
  await useSessionStore.getState().runProgram(source, `${P}/test.prg`);
  return useSessionStore
    .getState()
    .output.filter((line) => line.kind === 'output' || line.kind === 'error')
    .map((line) => line.text);
}

describe('the XMLAdapter', () => {
  it('is a class of its own rather than something to look for in COM', async () => {
    expect(
      await run(`oXML = CREATEOBJECT("XMLAdapter")
? VARTYPE(oXML)
? oXML.Class
? oXML.IsLoaded`),
    ).toEqual(['O', 'XMLAdapter', '.F.']);
  });

  it('reads a file and says what it holds', async () => {
    expect(
      await run(`oXML = CREATEOBJECT("XMLAdapter")
oXML.LoadXML("${P}/CustomerXML.xml", .T.)
? oXML.IsLoaded, oXML.XMLName
? oXML.Tables.Count
oTable = oXML.Tables(1)
? oTable.Alias, oTable.XMLName
? oTable.Fields.Count
? oTable.Fields(1).Name, oTable.Fields(3).Name`),
    ).toEqual(['.T. VFPData', '         1', 'customer customer', '         3', 'cust_id orders']);
  });

  it('makes a cursor of the rows it read', async () => {
    expect(
      await run(`oXML = CREATEOBJECT("XMLAdapter")
oXML.LoadXML("${P}/CustomerXML.xml", .T.)
oXML.Tables(1).ToCursor(.F., "custlist")
? ALIAS(), RECCOUNT(), FCOUNT()
GO TOP
? ALLTRIM(cust_id), ALLTRIM(company), orders
GO BOTTOM
? ALLTRIM(cust_id), orders`),
    ).toEqual(['CUSTLIST          2          3', 'ALFKI Alfreds Futterkiste 12', 'BERGS  7']);
  });

  it('reads XML held in a variable as well as one in a file', async () => {
    expect(
      await run(`cXML = '<VFPData><row id="1" name="Bolt"/><row id="2" name="Nut"/></VFPData>'
oXML = CREATEOBJECT("XMLAdapter")
oXML.LoadXML(cXML)
? oXML.Tables.Count, oXML.Tables(1).Alias
oXML.Tables(1).ToCursor(.F., "parts")
? RECCOUNT()
GO TOP
? id, ALLTRIM(name)`),
    ).toEqual(['         1 row', '         2', ' 1 Bolt']);
  });

  it('writes a cursor back out as XML', async () => {
    expect(
      await run(`CREATE CURSOR parts (code C(4), price N(8,2))
INSERT INTO parts VALUES ("A1", 2.50)
oXML = CREATEOBJECT("XMLAdapter")
cOut = oXML.ToXML("parts")
? IIF("<?xml" $ cOut, "declared", "no declaration")
? IIF("<code>A1</code>" $ cOut, "written", "missing")`),
    ).toEqual(['declared', 'written']);
  });

  it('says so when the file is not there, rather than reporting a missing COM class', async () => {
    expect(
      await run(`oXML = CREATEOBJECT("XMLAdapter")
TRY
  oXML.LoadXML("${P}/nothing.xml", .T.)
CATCH TO oErr
  ? oErr.ErrorNo, oErr.Message
ENDTRY`),
    ).toEqual([
      // the routine name upper-cases, the sentence ends with a full stop, and a number printed
      // on its own is right-aligned in the ten columns a variable's numeric carries
      `Error 1 in TEST line 3: File '${P}/nothing.xml' does not exist. (handled by the program)`,
      `         1 File '${P}/nothing.xml' does not exist.`,
    ]);
  });
});

describe('BROWSE', () => {
  it('shows the records of the cursor an adapter made', async () => {
    await run(`oXML = CREATEOBJECT("XMLAdapter")
oXML.LoadXML("${P}/CustomerXML.xml", .T.)
oXML.Tables(1).ToCursor(.F., "custlist")
BROWSE NOWAIT`);

    render(createElement(FluentProvider, { theme: webLightTheme }, createElement(RuntimeDesktopDocument)));
    expect(screen.getByTestId('browse-custlist')).toBeTruthy();
    expect(screen.getByText('ALFKI')).toBeTruthy();
    expect(screen.getByText('Berglunds snabbkop')).toBeTruthy();
    expect(screen.getByText('2 record(s)')).toBeTruthy();
  });
});
