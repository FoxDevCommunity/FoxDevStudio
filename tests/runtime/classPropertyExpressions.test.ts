import { beforeAll, beforeEach, describe, expect, it } from 'vitest';
import { setApi } from '@renderer/api/foxdev';
import { createMemoryApi } from '@renderer/api/memoryApi';
import { useSessionStore } from '@renderer/runtime/session';
import { createProjectSource } from '@renderer/runtime/projectSource';
import { loadFoxVm } from '../../src/wasm/foxvm/loader';

const source = createProjectSource();
const printed = () =>
  useSessionStore
    .getState()
    .output.filter((o) => o.kind === 'output')
    .map((o) => o.text.replace(/\s+/g, ' ').trim());

beforeAll(async () => {
  await loadFoxVm();
});

beforeEach(() => {
  setApi(createMemoryApi());
  useSessionStore.getState().cancel();
  useSessionStore.setState({ output: [] });
});

// Measured in Visual FoxPro 9.
describe('a class property written as an expression', () => {
  it('is worked out when the object is made, where CREATEOBJECT() was called', async () => {
    // measured in Visual FoxPro 9; Shutter Ace's QuickBooksInvoice.TxnDate is the real case
    await useSessionStore.getState().execute(
      source,
      [
        'o = CREATEOBJECT("cx")',
        '? "a", o.cDate == TRANSFORM(DTOS(DATE()),"@R xxxx-xx-xx"), o.nSum, VARTYPE(o.dWhen), o.dWhen == DATE(), o.cUp',
        'o3 = CREATEOBJECT("cchild")',
        '? "c", o3.nSum, o3.cDate == o.cDate',
        'nBase = 7',
        'o4 = CREATEOBJECT("cvar")',
        '? "d", VARTYPE(o4), o4.nFromVar',
        'DEFINE CLASS cx AS Custom',
        '  cDate = TRANSFORM(DTOS(DATE()),"@R xxxx-xx-xx")',
        '  nSum = 2 + 3',
        '  dWhen = DATE()',
        '  cUp = UPPER("abc")',
        'ENDDEFINE',
        'DEFINE CLASS cchild AS cx',
        '  nSum = 10 * 2',
        'ENDDEFINE',
        'DEFINE CLASS cvar AS Custom',
        '  nFromVar = nBase + 1',
        'ENDDEFINE',
      ].join('\n'),
    );
    expect(printed()).toEqual(['a .T. 5 D .T. ABC', 'c 20 .T.', 'd O 8']);
  });
});
