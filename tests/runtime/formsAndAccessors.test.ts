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

// Every expectation here was measured in Visual FoxPro 9 (the probe also had the wrapper's own
// form open, which is one more form than these programs make).
describe('_SCREEN.Forms', () => {
  it('counts forms, topmost first, and leaves a Custom object out', async () => {
    await useSessionStore.getState().execute(
      source,
      [
        'LOCAL c1, o1, o2',
        'c1 = CREATEOBJECT("custom")',
        'o1 = CREATEOBJECT("form")',
        'o1.Name = "one"',
        'o2 = CREATEOBJECT("form")',
        'o2.Name = "two"',
        '? "fc", _SCREEN.FormCount',
        '? "vt", VARTYPE(_SCREEN.Forms(1))',
        '? "names", _SCREEN.Forms(1).Name, _SCREEN.Forms(2).Name',
      ].join('\n'),
    );
    expect(printed()).toEqual(['fc 2', 'vt O', 'names two one']);
  });

  it('is walked by FOR EACH', async () => {
    await useSessionStore.getState().execute(
      source,
      [
        'LOCAL o1, o2, oF, n',
        'o1 = CREATEOBJECT("form")',
        'o1.Name = "one"',
        'o2 = CREATEOBJECT("form")',
        'o2.Name = "two"',
        'n = 0',
        'FOR EACH oF IN _SCREEN.Forms',
        '  n = n + 1',
        '  ? "each", n, oF.Name, oF.BaseClass',
        'ENDFOR',
        '? "count", n',
        'TRY',
        '  x = _SCREEN.Forms(3)',
        'CATCH TO e',
        '  ? "past the end", e.ErrorNo',
        'ENDTRY',
      ].join('\n'),
    );
    expect(printed()).toEqual(['each 1 two Form', 'each 2 one Form', 'count 2', 'past the end 1924']);
  });

  it('leaves FOR EACH over an array property as it was', async () => {
    await useSessionStore.getState().execute(
      source,
      [
        'LOCAL o, x',
        'o = CREATEOBJECT("holder")',
        'FOR EACH x IN o.aItems',
        '  ? "item", x',
        'ENDFOR',
        'DEFINE CLASS holder AS Custom',
        '  DIMENSION aItems[2]',
        '  PROCEDURE Init',
        '    THIS.aItems[1] = "a"',
        '    THIS.aItems[2] = "b"',
        '  ENDPROC',
        'ENDDEFINE',
      ].join('\n'),
    );
    expect(printed()).toEqual(['item a', 'item b']);
  });
});

describe('a released form', () => {
  it('is .NULL. to every variable and property still holding it', async () => {
    // measured in Visual FoxPro 9, and what CodeMine's CloseSplash leans on: it asks
    // ISNULL(THIS.oSplash) before releasing a splash screen that may have gone already
    await useSessionStore.getState().execute(
      source,
      [
        'LOCAL oF, oHold, oF2, x',
        'oF = CREATEOBJECT("form")',
        'oHold = CREATEOBJECT("custom")',
        'oHold.AddProperty("oSplash", oF)',
        'oF.Release()',
        '? "var", VARTYPE(oF), ISNULL(oF)',
        '? "prop", VARTYPE(oHold.oSplash), ISNULL(oHold.oSplash)',
        'oF2 = CREATEOBJECT("form")',
        'oHold.oSplash = oF2',
        'oF2 = .NULL.',
        '? "held", VARTYPE(oHold.oSplash), ISNULL(oHold.oSplash)',
        'x = oHold.oSplash.Release()',
        '? "ret", x',
        '? "after", VARTYPE(oHold.oSplash), ISNULL(oHold.oSplash)',
      ].join('\n'),
    );
    expect(printed()).toEqual(['var X .T.', 'prop X .T.', 'held O .F.', 'ret .T.', 'after X .T.']);
  });
});

describe('a form made from a class', () => {
  it('has no DataEnvironment', async () => {
    // measured: only a form read from a form file has one
    await useSessionStore.getState().execute(
      source,
      [
        'LOCAL o, a[1]',
        'o = CREATEOBJECT("fx")',
        '? "pem", PEMSTATUS(o, "DataEnvironment", 5)',
        '? "type", TYPE("o.DataEnvironment")',
        'o = CREATEOBJECT("form")',
        '? "form", PEMSTATUS(o, "DataEnvironment", 5)',
        'DEFINE CLASS fx AS Form',
        'ENDDEFINE',
      ].join('\n'),
    );
    expect(printed()).toEqual(['pem .F.', 'type U', 'form .F.']);
  });
});

describe('Access and Assign methods', () => {
  it('stand in for reading and writing the property, except inside themselves', async () => {
    // measured in Visual FoxPro 9; CodeMine's lDataManagerPresent_Access is the lFlag case
    await useSessionStore.getState().execute(
      source,
      [
        'LOCAL o',
        'o = CREATEOBJECT("cacc")',
        '? "read", o.nValue',
        'o.nValue = 5',
        '? "after write", o.nValue',
        '? "inner", o.ReadInside()',
        '? "flag", o.lFlag',
        'o.lFlag = .T.',
        '? "flag2", o.lFlag',
        '? "calls", o.nAccessed, o.cAssigned',
        'DEFINE CLASS cacc AS Custom',
        '  nValue = 1',
        '  lFlag = .NULL.',
        '  nAccessed = 0',
        '  cAssigned = ""',
        '  PROCEDURE nValue_Access',
        '    THIS.nAccessed = THIS.nAccessed + 1',
        '    RETURN THIS.nValue * 10',
        '  ENDPROC',
        '  PROCEDURE nValue_Assign(vNew)',
        '    THIS.cAssigned = TRANSFORM(vNew) + "/" + TRANSFORM(THIS.nValue)',
        '    THIS.nValue = vNew + 1',
        '  ENDPROC',
        '  PROCEDURE ReadInside',
        '    RETURN THIS.nValue',
        '  ENDPROC',
        '  PROCEDURE lFlag_Access',
        '    IF ISNULL(THIS.lFlag)',
        '      THIS.lFlag = .F.',
        '    ENDIF',
        '    RETURN THIS.lFlag',
        '  ENDPROC',
        'ENDDEFINE',
      ].join('\n'),
    );
    expect(printed()).toEqual(['read 10', 'after write 60', 'inner 60', 'flag .F.', 'flag2 .T.', 'calls 3 5/1']);
  });

  it('hand an array property its subscript, and leave a write, ALEN and DIMENSION alone', async () => {
    // measured in Visual FoxPro 9; the Solution sample aa_fun.scx reads THIS.myButtons[1] in Init.
    // cLog records what each read handed the Access method.
    await useSessionStore.getState().execute(
      source,
      [
        'LOCAL o',
        'o = CREATEOBJECT("carr")',
        '? "e1", o.myButtons[1], o.TakeLog()',
        '? "e2", o.myButtons[2], o.TakeLog()',
        '? "paren", o.myButtons("x"), o.TakeLog()',
        '? "whole", VARTYPE(o.myButtons), o.TakeLog()',
        'o.myButtons[1] = 5',
        '? "write", o.TakeLog()',
        '? "written", o.myButtons[1], o.TakeLog()',
        '? "alen", ALEN(o.myButtons), o.TakeLog()',
        'DIMENSION o.myButtons[4]',
        '? "alen2", ALEN(o.myButtons), o.TakeLog()',
        '? "inside", o.ReadOther(), o.TakeLog()',
        'x = o',
        '? "private", x.myButtons[3], x.TakeLog()',
        'DEFINE CLASS carr AS Custom',
        '  DIMENSION myButtons[3]',
        '  cLog = ""',
        '  PROCEDURE Init',
        '    THIS.myButtons[1] = "a"',
        '    THIS.myButtons[2] = "b"',
        '    THIS.myButtons[3] = "c"',
        '  ENDPROC',
        '  PROCEDURE myButtons_Access',
        '    LPARAMETERS nIndex',
        '    THIS.cLog = THIS.cLog + "[" + VARTYPE(nIndex) + TRANSFORM(nIndex) + "]"',
        '    IF VARTYPE(nIndex) = "N"',
        '      RETURN THIS.myButtons[nIndex]',
        '    ENDIF',
        '    RETURN "whole"',
        '  ENDPROC',
        '  PROCEDURE ReadOther',
        '    RETURN THIS.myButtons[2]',
        '  ENDPROC',
        '  PROCEDURE TakeLog',
        '    LOCAL c',
        '    c = THIS.cLog',
        '    THIS.cLog = ""',
        '    RETURN "log" + c',
        '  ENDPROC',
        'ENDDEFINE',
      ].join('\n'),
    );
    expect(printed()).toEqual([
      'e1 a log[N1]',
      'e2 b log[N2]',
      'paren whole log[Cx]',
      'whole C log[N1]',
      'write log',
      'written 5 log[N1]',
      'alen 3 log',
      'alen2 4 log',
      'inside b log[N2]',
      'private c log[N3]',
    ]);
  });
});

describe('an event called as a method', () => {
  it('answers .T. when nothing was written for it', async () => {
    // measured: CodeMine's pageframe calls m.oPage.Init() to fire what VFP does not
    await useSessionStore.getState().execute(
      source,
      [
        'LOCAL o, p',
        'o = CREATEOBJECT("fx")',
        'p = o.pf.Page1',
        '? "page", p.Init(), p.Click(), p.Destroy(), p.Activate()',
        '? "form", o.Init(), o.Load(), o.Click()',
        '? "cmd", o.cmd.Init(), o.cmd.Click(), o.cmd.Valid(), o.cmd.When()',
        'DEFINE CLASS fx AS Form',
        '  ADD OBJECT pf AS PageFrame WITH PageCount = 2',
        '  ADD OBJECT cmd AS CommandButton',
        'ENDDEFINE',
      ].join('\n'),
    );
    expect(useSessionStore.getState().output.filter((o) => o.kind === 'error').map((o) => o.text)).toEqual([]);
    expect(printed()).toEqual(['page .T. .T. .T. .T.', 'form .T. .T. .T.', 'cmd .T. .T. .T. .T.']);
  });
});

describe('a page', () => {
  it('answers PageOrder with its place in the pageframe', async () => {
    // measured; CodeMine's ActivePage_Assign sets ActivePage from a page's PageOrder
    await useSessionStore.getState().execute(
      source,
      [
        'LOCAL o',
        'o = CREATEOBJECT("fx")',
        '? "order", o.pf.Pages(1).PageOrder, o.pf.Pages(2).PageOrder, o.pf.Pages(3).PageOrder',
        '? "active", o.pf.ActivePage',
        'o.pf.ActivePage = o.pf.Pages(2).PageOrder',
        '? "after", o.pf.ActivePage',
        'DEFINE CLASS fx AS Form',
        '  ADD OBJECT pf AS PageFrame WITH PageCount = 3',
        'ENDDEFINE',
      ].join('\n'),
    );
    expect(printed()).toEqual(['order 1 2 3', 'active 1', 'after 2']);
  });
});

describe('_VFP', () => {
  it('takes what a program writes on it, and refuses a name it does not have', async () => {
    // measured: CodeMine's RestoreSizeAndPosition writes the main window's size this way
    await useSessionStore.getState().execute(
      source,
      [
        '_VFP.Height = 500',
        '_VFP.Width = 700',
        '? "after", _VFP.Height, _VFP.Width',
        'TRY',
        '  _VFP.Foo = 1',
        'CATCH TO e',
        '  ? "unknown", e.ErrorNo',
        'ENDTRY',
      ].join('\n'),
    );
    expect(printed()).toEqual(['after 500 700', 'unknown 1426']);
  });
});
