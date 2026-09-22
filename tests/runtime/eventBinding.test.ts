import { beforeAll, beforeEach, describe, expect, it } from 'vitest';
import { setApi } from '@renderer/api/foxdev';
import { createMemoryApi } from '@renderer/api/memoryApi';
import { useSessionStore } from '@renderer/runtime/session';
import { createProjectSource } from '@renderer/runtime/projectSource';
import { loadFoxVm } from '../../src/wasm/foxvm/loader';

const source = createProjectSource();
const lines = () => useSessionStore.getState().output.filter((o) => o.kind === 'output').map((o) => o.text);

/** Answers any dialog or error so a headless run cannot hang. */
async function run(code: string): Promise<void> {
  const promise = useSessionStore.getState().execute(source, code);
  for (let i = 0; i < 60; i++) {
    await new Promise((r) => setTimeout(r, 20));
    const state = useSessionStore.getState();
    if (state.errorReport) {
      throw new Error(`${state.errorReport.error.message} (${state.errorReport.error.program} line ${state.errorReport.error.line})`);
    }
    if (state.status === 'idle') break;
  }
  await promise;
}

beforeAll(async () => {
  await loadFoxVm();
});

beforeEach(() => {
  setApi(createMemoryApi());
  useSessionStore.getState().cancel();
  useSessionStore.setState({ output: [] });
});

describe('BINDEVENT', () => {
  const handlerClass = [
    'DEFINE CLASS MyHandler AS custom',
    '\tPROCEDURE MyResize',
    '\t\t? "handler ran"',
    '\tENDPROC',
    '\tPROCEDURE Second',
    '\t\t? "second ran"',
    '\tENDPROC',
    'ENDDEFINE',
  ].join('\n');

  it('runs a bound delegate when the source event fires', async () => {
    await run(
      [
        'oHandler = NEWOBJECT("MyHandler")',
        'oForm = CREATEOBJECT("form1")',
        '? BINDEVENT(oForm, "Resize", oHandler, "MyResize")',
        'RAISEEVENT(oForm, "Resize")',
        'RETURN',
        '',
        'DEFINE CLASS form1 AS form',
        '\tCaption = "Bound"',
        'ENDDEFINE',
        '',
        handlerClass,
      ].join('\n'),
    );
    expect(lines()).toContain('handler ran');
  });

  it('unbinds so the delegate stops running', async () => {
    await run(
      [
        'oHandler = NEWOBJECT("MyHandler")',
        'oForm = CREATEOBJECT("form1")',
        'BINDEVENT(oForm, "Resize", oHandler, "MyResize")',
        'RAISEEVENT(oForm, "Resize")',
        // measured against vfp9.exe: UNBINDEVENTS takes the full four-argument binding, named
        // the way BINDEVENT() named it, or a single object alone - two arguments (oSource,
        // cEvent) is neither shape, and errors 11 on the real product
        '? LTRIM(STR(UNBINDEVENTS(oForm, "Resize", oHandler, "MyResize")))',
        'RAISEEVENT(oForm, "Resize")',
        'RETURN',
        '',
        'DEFINE CLASS form1 AS form',
        'ENDDEFINE',
        '',
        handlerClass,
      ].join('\n'),
    );
    // ran once before the unbind, not after
    expect(lines().filter((l) => l === 'handler ran')).toHaveLength(1);
    // UNBINDEVENTS answers with how many bindings it took away, not with a logical
    expect(lines()).toContain('1');
  });

  it('runs several delegates bound to one event, in order', async () => {
    await run(
      [
        'oHandler = NEWOBJECT("MyHandler")',
        'oForm = CREATEOBJECT("form1")',
        'BINDEVENT(oForm, "Resize", oHandler, "MyResize")',
        'BINDEVENT(oForm, "Resize", oHandler, "Second")',
        'RAISEEVENT(oForm, "Resize")',
        'RETURN',
        '',
        'DEFINE CLASS form1 AS form',
        'ENDDEFINE',
        '',
        handlerClass,
      ].join('\n'),
    );
    expect(lines()).toEqual(['handler ran', 'second ran']);
  });

  it('runs the object own method before the bound delegate', async () => {
    await run(
      [
        'oHandler = NEWOBJECT("MyHandler")',
        'oForm = CREATEOBJECT("form1")',
        'BINDEVENT(oForm, "Resize", oHandler, "MyResize")',
        'RAISEEVENT(oForm, "Resize")',
        'RETURN',
        '',
        'DEFINE CLASS form1 AS form',
        '\tPROCEDURE Resize',
        '\t\t? "own method"',
        '\tENDPROC',
        'ENDDEFINE',
        '',
        handlerClass,
      ].join('\n'),
    );
    expect(lines()).toEqual(['own method', 'handler ran']);
  });
});

describe('the functions the samples were missing', () => {
  it('RGB builds a VFP colour integer', async () => {
    await run('? RGB(255, 0, 0)\n? RGB(0, 0, 255)\n? RGB(0, 121, 214)');
    expect(lines()).toEqual(['       255', '  16711680', '  14055680']);
  });

  it('ADDPROPERTY and REMOVEPROPERTY work as a pair', async () => {
    await run(
      [
        'oT = CREATEOBJECT("thing")',
        '? ADDPROPERTY(oT, "Extra", 7)',
        '? oT.Extra',
        '? REMOVEPROPERTY(oT, "Extra")',
        'RETURN',
        '',
        'DEFINE CLASS thing AS custom',
        'ENDDEFINE',
      ].join('\n'),
    );
    expect(lines()).toEqual(['.T.', '         7', '.T.']);
  });

  it('AEMPTY empties an array', async () => {
    await run('DIMENSION a(3)\na[1] = "x"\n? ALEN(a)\n? AEMPTY(a)\n? ALEN(a)');
    expect(lines()).toEqual(['         3', '         0', '         0']);
  });

  it('FONTMETRIC reports a consistent height', async () => {
    // 1 is the whole character cell, 2 the ascent and 3 the descent, as VFP numbers them
    await run('? FONTMETRIC(2, "Segoe UI", 10) + FONTMETRIC(3, "Segoe UI", 10) = FONTMETRIC(1, "Segoe UI", 10)');
    expect(lines()).toEqual(['.T.']);
  });

  it('HOME returns the project directory', async () => {
    await run('? EMPTY(HOME())');
    expect(lines()).toHaveLength(1);
  });
});

describe('HOME', () => {
  it('answers only for the default form, since VFP asks about its own installation', async () => {
    await run('? HOME() == HOME(0)\n? EMPTY(HOME(4))');
    // HOME(4) is VFP's samples directory, which has no meaning here
    expect(lines()).toEqual(['.T.', '.T.']);
  });
});
