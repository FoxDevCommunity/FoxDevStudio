import { readFileSync } from 'node:fs';
import { beforeAll, describe, expect, it } from 'vitest';
import { parseFormDocument } from '@shared/form/serialize';
import type { FormDocument } from '@shared/form/schema';
import { Desktop, FormInstance, SCREEN_HANDLE, type RuntimeObject } from '@shared/runtime/objectModel';
import { formMethodSources, requireBytes } from '@shared/runtime/programSource';
import { Scheduler, type EventOutcome } from '@shared/runtime/scheduler';
import { HostError, type HostRequest } from '@shared/runtime/host';
import type { VmValue } from '@shared/runtime/values';
import { compileForm, createVm, type WasmVm } from '@renderer/runtime/vmBridge';
import { loadFoxVm } from '../../src/wasm/foxvm/loader';

function sampleDoc(): FormDocument {
  const text = readFileSync('resources/samples/HelloWorld.fxf', 'utf8');
  const parsed = parseFormDocument(text);
  if (!parsed.ok) throw new Error(parsed.error);
  return parsed.doc;
}

interface Session {
  desktop: Desktop;
  vm: WasmVm;
  scheduler: Scheduler;
  requests: HostRequest[];
  errors: string[];
  open(doc: FormDocument, opts?: { modal?: boolean; noshow?: boolean }): Promise<FormInstance>;
}

/** Wires a Desktop, the real wasm VM and the scheduler together the way the session will. */
function makeSession(): Session {
  const desktop = new Desktop();
  desktop.clock = () => new Date(2026, 8, 7, 13, 5, 7);
  const vm = createVm(desktop);
  const requests: HostRequest[] = [];
  const errors: string[] = [];

  const scheduler = new Scheduler(
    vm,
    {
      perform(request) {
        requests.push(request);
        switch (request.kind) {
          case 'SetProp':
            desktop.setProp(request.obj, request.name, request.value);
            return null;
          case 'CallMethod': {
            const result = desktop.callMethod(request.obj, request.name, request.args);
            if (result === undefined) throw new HostError(1925, `Unknown member ${request.name.toUpperCase()}.`);
            return result;
          }
          case 'AddProperty':
            return desktop.addProperty(request.obj, request.name, request.value);
          case 'MessageBox':
            return 6;
          default:
            return null;
        }
      },
    },
    {
      onError: (e) => {
        errors.push(`${e.program}:${e.line} ${e.message}`);
        return 'cancel';
      },
    },
  );

  // the object model runs FoxPro handlers through the scheduler
  desktop.dispatch = (obj, event, args) => {
    const form = obj.form();
    if (!form || form.module < 0) return null;
    return scheduler.dispatch(form.module, obj.path(), event, obj.handle, args ?? []);
  };

  return {
    desktop,
    vm,
    scheduler,
    requests,
    errors,
    async open(doc, opts = {}) {
      const bytes = requireBytes(doc.form.name, compileForm(doc.form.name, formMethodSources(doc)));
      const module = vm.loadModule(bytes);
      const instance = desktop.instantiate(doc.form, module, { ...opts, cursors: doc.data });
      await desktop.runFormLifecycle(instance, opts);
      return instance;
    },
  };
}

const child = (o: RuntimeObject, path: string): RuntimeObject => {
  let cur: RuntimeObject | undefined = o;
  for (const part of path.split('.')) cur = cur?.child(part);
  if (!cur) throw new Error(`no such child: ${path}`);
  return cur;
};

beforeAll(async () => {
  await loadFoxVm();
});

describe('runtime object model', () => {
  it('_SCREEN holds what a program writes to it and what AddProperty gives it', () => {
    // a CodeMine application's first lines: the caption, and a property that carries the
    // start-up parameter across CLEAR ALL
    const desktop = new Desktop();
    desktop.setProp(SCREEN_HANDLE, 'Caption', 'ShutterDesign II');
    expect(desktop.getProp(SCREEN_HANDLE, 'CAPTION')).toBe('ShutterDesign II');

    expect(desktop.getMember(SCREEN_HANDLE, 'uCodeMineAppParameter')).toBe('none');
    expect(desktop.callMethod(SCREEN_HANDLE, 'AddProperty', ['uCodeMineAppParameter', 'x'])).toBe(true);
    expect(desktop.getProp(SCREEN_HANDLE, 'ucodemineappparameter')).toBe('x');
    expect(desktop.getMember(SCREEN_HANDLE, 'uCodeMineAppParameter')).toBe('prop');

    // a name the screen has never had is refused, as it is on a form
    expect(() => desktop.setProp(SCREEN_HANDLE, 'cNothing', 1)).toThrow(/CNOTHING is not found/);
  });

  it('builds the live tree from a form document', () => {
    const { desktop } = makeSession();
    const form = desktop.instantiate(sampleDoc().form, -1);

    expect(form.name).toBe('frmHello');
    expect(form.baseClass).toBe('Form');
    expect(form.get('Caption')).toBe('Hello, World');
    expect(child(form, 'txtName').type).toBe('TextBox');
    expect(child(form, 'pgfMain.Page1.lblGreeting').path()).toBe('pgfMain.Page1.lblGreeting');
    expect(child(form, 'pgfMain.Page1.lblGreeting').form()).toBe(form);
    // VFP property access is case-insensitive
    expect(child(form, 'txtName').get('value')).toBe(child(form, 'txtName').get('Value'));
    expect(desktop.object(form.handle)).toBe(form);
  });

  it('runs the sample form: Init focuses the text box, Click builds the greeting', async () => {
    const session = makeSession();
    const focused: string[] = [];
    const form = await session.open(sampleDoc());

    // Init ran: THISFORM.txtName.SetFocus()
    expect(session.requests.some((r) => r.kind === 'CallMethod' && r.name.toLowerCase() === 'setfocus')).toBe(true);
    focused.push('ok');

    const txt = child(form, 'txtName');
    const chk = child(form, 'chkLoud');
    const label = child(form, 'pgfMain.Page1.lblGreeting');
    expect(label.get('Caption')).toBe('(nothing yet)');

    txt.set('Value', 'jorge', 'interactive');
    chk.set('Value', true, 'interactive');
    await session.desktop.dispatch(child(form, 'cmdSayHi'), 'Click');

    expect(label.get('Caption')).toBe('HELLO, JORGE!');
    expect(session.errors).toEqual([]);
    expect(focused).toEqual(['ok']);
  });

  it('calls a method the form added for itself, and reads a property it added', async () => {
    // A VFP form carries methods and properties of its own - CenterForm, lCalledBySolution - and
    // both are used like any other. Without them a whole form stops at "Unknown member".
    const doc = sampleDoc();
    doc.form.props['lReady'] = false;
    doc.form.methods['SetReady'] = 'THISFORM.lReady = .T.';
    doc.form.methods['Init'] = 'THISFORM.SetReady()';

    const session = makeSession();
    const form = await session.open(doc);
    expect(session.errors).toEqual([]);
    expect(form.get('lReady')).toBe(true);
  });

  it('SetAll reaches every control inside the container', async () => {
    const doc = sampleDoc();
    doc.form.methods['Init'] = 'THISFORM.SetAll("Enabled", .F.)';
    const session = makeSession();
    const form = await session.open(doc);
    expect(session.errors).toEqual([]);
    const controls = form.descendants();
    expect(controls.length).toBeGreaterThan(1);
    expect(controls.every((c) => !c.has('Enabled') || c.get('Enabled') === false)).toBe(true);
  });

  it('THISFORM.Release() tears the form down and unregisters its handles', async () => {
    const session = makeSession();
    const form = await session.open(sampleDoc());
    const label = child(form, 'pgfMain.Page1.lblGreeting');
    expect(session.desktop.forms).toHaveLength(1);

    await session.desktop.dispatch(child(form, 'cmdClose'), 'Click');

    expect(form.alive).toBe(false);
    expect(label.alive).toBe(false);
    expect(session.desktop.forms).toEqual([]);
    expect(session.desktop.object(form.handle)).toBeUndefined();
    // a released object reports no class, which the VM turns into "Member  does not evaluate to an object."
    expect(session.desktop.objectClass(form.handle)).toBeNull();
  });

  it('fires the VFP lifecycle in order and cancels when Init returns .F.', async () => {
    const doc = sampleDoc();
    doc.form.methods['Load'] = '';
    const order: string[] = [];
    const session = makeSession();
    const base = session.desktop.dispatch;
    session.desktop.dispatch = (obj, event, args) => {
      order.push(`${obj.path() || obj.name}.${event}`);
      return base(obj, event, args);
    };
    const form = await session.open(doc);
    const at = (entry: string) => order.indexOf(entry);

    // Load first, then every control's Init, then the form's, then Show and Activate
    expect(order[0]).toBe('frmHello.Load');
    expect(at('frmHello.Init')).toBeGreaterThan(at('txtName.Init'));
    expect(at('frmHello.Init')).toBeGreaterThan(at('pgfMain.Init'));
    // a child initialises before the container that holds it
    expect(at('pgfMain.Page1.lblGreeting.Init')).toBeLessThan(at('pgfMain.Page1.Init'));
    expect(at('pgfMain.Page1.Init')).toBeLessThan(at('pgfMain.Init'));
    expect(at('frmHello.Show')).toBeGreaterThan(at('frmHello.Init'));
    expect(at('frmHello.Activate')).toBeGreaterThan(at('frmHello.Show'));
    expect(form.get('Visible')).toBe(true);

    const cancelling = sampleDoc();
    cancelling.form.methods['Init'] = 'RETURN .F.';
    const second = makeSession();
    const bytes = requireBytes('x', compileForm('x', formMethodSources(cancelling)));
    const module = second.vm.loadModule(bytes);
    const instance = second.desktop.instantiate(cancelling.form, module);
    expect(await second.desktop.runFormLifecycle(instance)).toBe(false);
    expect(second.desktop.forms).toEqual([]);
  });

  it('distinguishes programmatic from interactive value changes', async () => {
    const doc = sampleDoc();
    doc.form.children.find((c) => c.name === 'txtName')!.methods['InteractiveChange'] = '? "interactive"';
    doc.form.children.find((c) => c.name === 'txtName')!.methods['ProgrammaticChange'] = '? "programmatic"';
    const session = makeSession();
    const form = await session.open(doc);
    session.desktop.clearOutput();

    child(form, 'txtName').set('Value', 'typed', 'interactive');
    child(form, 'txtName').set('Value', 'assigned', 'program');
    expect(session.desktop.lines).toEqual(['interactive', 'programmatic']);
  });

  it('answers intrinsic properties and _SCREEN members', async () => {
    const session = makeSession();
    const form = await session.open(sampleDoc());
    const txt = child(form, 'txtName');

    expect(session.desktop.getProp(txt.handle, 'Name')).toBe('txtName');
    expect(session.desktop.getProp(txt.handle, 'Parent')).toEqual({ $obj: form.handle });
    // the product's own spelling, from tests/reference/vfp-base-classes.tsv: first letter up,
    // the rest down, whatever the node type this control was built from happens to be called
    expect(session.desktop.getProp(txt.handle, 'BaseClass')).toBe('Textbox');
    expect(session.desktop.getProp(form.handle, 'ControlCount')).toBe(form.children.length);
    expect(session.desktop.getProp(SCREEN_HANDLE, 'FormCount')).toBe(1);
    expect(session.desktop.getProp(SCREEN_HANDLE, 'ActiveForm')).toEqual({ $obj: form.handle });

    expect(session.desktop.getMember(form.handle, 'txtname')).toBe(txt.handle);
    expect(session.desktop.getMember(form.handle, 'Caption')).toBe('prop');
    expect(session.desktop.getMember(form.handle, 'Release')).toBe('method');
    expect(session.desktop.getMember(form.handle, 'Nope')).toBe('none');
    expect(session.desktop.getMember(SCREEN_HANDLE, 'frmHello')).toBe(form.handle);
  });

  /**
   * A form on its own is contained by nothing. The product raises 1924 rather than handing back
   * the screen, and `TYPE("THISFORM.Parent")` is "U" - measured - which is the question a form
   * asks to tell whether it is in a form set. Answering the screen made every such form take the
   * form-set branch, and the Solution sample's Close button then said "Object is not contained
   * in a FORMSET" instead of closing.
   */
  it('gives a form no parent unless something holds it', async () => {
    const session = makeSession();
    const form = await session.open(sampleDoc());

    expect(() => session.desktop.getProp(form.handle, 'Parent')).toThrow(/PARENT is not an object/);
    expect(() => session.desktop.getProp(SCREEN_HANDLE, 'Parent')).toThrow(/PARENT is not an object/);
    expect(session.desktop.getProp(child(form, 'txtName').handle, 'Parent')).toEqual({ $obj: form.handle });
    // and ParentClass is the class a class was made from, which a base class has none of
    expect(session.desktop.getProp(form.handle, 'ParentClass')).toBe('');
  });

  /**
   * Which members a class has, and which of them a program may write, are the product's answers
   * (`tests/reference/vfp-base-classes.tsv`) rather than one list shared by everything.
   */
  it('answers only the members its class has, and refuses a write the product refuses', async () => {
    const session = makeSession();
    const form = await session.open(sampleDoc());
    const label = child(form, 'lblName');
    const txt = child(form, 'txtName');

    // a form is a container and a label is not, so AddObject belongs to one of them
    expect(session.desktop.getMember(form.handle, 'AddObject')).toBe('method');
    expect(session.desktop.getMember(label.handle, 'AddObject')).toBe('none');
    expect(session.desktop.getMember(label.handle, 'SetAll')).toBe('none');
    expect(session.desktop.getMember(label.handle, 'Refresh')).toBe('method');
    // and a command button has a Click but no DblClick
    expect(session.desktop.getMember(child(form, 'cmdSayHi').handle, 'Click')).toBe('method');
    expect(session.desktop.getMember(child(form, 'cmdSayHi').handle, 'DblClick')).toBe('none');

    // the product works these out and will not have them written
    expect(() => session.desktop.setProp(form.handle, 'Class', 'other')).toThrow(/read-only/);
    expect(() => session.desktop.setProp(txt.handle, 'Text', 'typed')).toThrow(/read-only/);
    expect(() => session.desktop.setProp(form.handle, 'ViewPortHeight', 10)).toThrow(/read-only/);
    // while the ones beside them take a write as they always did
    session.desktop.setProp(txt.handle, 'Value', 'Ada');
    expect(txt.get('Value')).toBe('Ada');
  });

  it('reports an unknown property as a FoxPro error with the failing line', async () => {
    const doc = sampleDoc();
    doc.form.children.find((c) => c.name === 'cmdSayHi')!.methods['Click'] = '* first\nTHISFORM.txtName.Nope = 1';
    const session = makeSession();
    const form = await session.open(doc);

    // onError answers synchronously, so the cancelled dispatch throws rather than rejecting
    expect(() => session.desktop.dispatch(child(form, 'cmdSayHi'), 'Click')).toThrow();
    expect(session.errors).toEqual(['CMDSAYHI.CLICK:2 Property NOPE is not found.']);
  });

  it('notifies subscribers only for the object that changed', async () => {
    const session = makeSession();
    const form = await session.open(sampleDoc());
    const label = child(form, 'pgfMain.Page1.lblGreeting');
    const txt = child(form, 'txtName');
    let labelHits = 0;
    let txtHits = 0;
    label.subscribe(() => labelHits++);
    txt.subscribe(() => txtHits++);

    label.set('Caption', 'changed');
    expect(labelHits).toBe(1);
    expect(txtHits).toBe(0);
    expect(label.getSnapshot()).toBe(label.version);

    // setting the same value again is not a change
    label.set('Caption', 'changed');
    expect(labelHits).toBe(1);
  });

  it('supports ADDPROPERTY and user properties', async () => {
    const session = makeSession();
    const form = await session.open(sampleDoc());
    expect(session.desktop.addProperty(form.handle, 'nCounter', 5)).toBe(true);
    expect(form.get('ncounter')).toBe(5);
    expect(session.desktop.getMember(form.handle, 'nCounter')).toBe('prop');
    expect(form.resolved()['nCounter']).toBe(5);
  });

  it('resolves a modal form and its Unload result for DO FORM ... TO', async () => {
    const doc = sampleDoc();
    doc.form.props['WindowType'] = 1;
    doc.form.methods['Unload'] = 'RETURN "closed by user"';
    const session = makeSession();
    const form = await session.open(doc);
    expect(form.modal).toBe(true);

    const closed: Promise<VmValue> = form.whenClosed();
    await session.desktop.releaseForm(form, { queryUnload: false });
    await expect(closed).resolves.toBe('closed by user');
  });

  it('lets QueryUnload cancel a close with NODEFAULT', async () => {
    const doc = sampleDoc();
    doc.form.methods['QueryUnload'] = 'NODEFAULT';
    const session = makeSession();
    const form = await session.open(doc);

    expect(await session.desktop.releaseForm(form, { queryUnload: true })).toBe(false);
    expect(form.alive).toBe(true);
    expect(await session.desktop.releaseForm(form, { queryUnload: false })).toBe(true);
    expect(form.alive).toBe(false);
  });

  it('flattens method sources with the event parameter list', () => {
    const sources = formMethodSources(sampleDoc());
    expect(sources).toContainEqual(expect.objectContaining({ objectPath: '', event: 'Init' }));
    expect(sources).toContainEqual(expect.objectContaining({ objectPath: 'cmdSayHi', event: 'Click' }));
    // blank methods are skipped
    expect(sources.every((s) => s.source.trim().length > 0)).toBe(true);
  });

  it('returns null for an event the object has no code for', async () => {
    const session = makeSession();
    const form = await session.open(sampleDoc());
    const outcome: EventOutcome | Promise<EventOutcome> | null = session.desktop.dispatch(child(form, 'txtName'), 'MouseMove');
    expect(outcome).toBeNull();
  });

  it('takes the controls anchored to an edge with the form when it is resized', async () => {
    const session = makeSession();
    const form = await session.open(sampleDoc());
    const txt = child(form, 'txtName');
    const left = Number(txt.get('Left'));
    const top = Number(txt.get('Top'));
    const width = Number(txt.get('Width'));

    // 8 is the right edge: the control keeps its distance from it, so it moves
    txt.set('Anchor', 8);
    form.set('Width', Number(form.get('Width')) + 40);
    expect(txt.get('Left')).toBe(left + 40);

    // held to both sides it stretches instead
    txt.set('Left', left);
    txt.set('Anchor', 2 + 8);
    form.set('Width', Number(form.get('Width')) + 10);
    expect(txt.get('Left')).toBe(left);
    expect(txt.get('Width')).toBe(width + 10);

    // and the bottom edge moves it down
    txt.set('Anchor', 4);
    form.set('Height', Number(form.get('Height')) + 25);
    expect(txt.get('Top')).toBe(top + 25);

    // a control with no anchor stays where it was put
    const label = child(form, 'chkLoud');
    const where = Number(label.get('Left'));
    form.set('Width', Number(form.get('Width')) + 15);
    expect(label.get('Left')).toBe(where);
  });

  it('gives a form the data environment its document describes', async () => {
    const doc = sampleDoc();
    doc.data = [{ alias: 'customer', source: 'data/customer.dbf', order: 'cust_id' }];
    const session = makeSession();
    const form = await session.open(doc);

    const environment = session.desktop.getMember(form.handle, 'DataEnvironment');
    expect(typeof environment).toBe('number');
    const handle = environment as number;
    expect(session.desktop.getProp(handle, 'BaseClass')).toBe('DataEnvironment');
    expect(session.desktop.getProp(handle, 'InitialSelectedAlias')).toBe('customer');
    expect(session.desktop.getProp(handle, 'AutoOpenTables')).toBe(true);

    // the cursors are objects of their own, named as VFP names them
    const cursor = session.desktop.getMember(handle, 'Cursor1');
    expect(typeof cursor).toBe('number');
    expect(session.desktop.getProp(cursor as number, 'Alias')).toBe('customer');
    expect(session.desktop.getProp(cursor as number, 'CursorSource')).toBe('data/customer.dbf');
    expect(session.desktop.getProp(cursor as number, 'Order')).toBe('cust_id');
    expect(session.desktop.getProp(cursor as number, 'BufferModeOverride')).toBe(1);
  });

  it('draws on the form, and answers the methods every object has', async () => {
    const session = makeSession();
    const form = await session.open(sampleDoc());
    const txt = child(form, 'txtName');

    // what a form is told to draw is kept, in order, until it is cleared
    expect(session.desktop.callMethod(form.handle, 'Box', [10, 20, 60, 70])).toBe(null);
    expect(session.desktop.callMethod(form.handle, 'Line', [0, 0, 100, 100])).toBe(null);
    expect(session.desktop.callMethod(form.handle, 'Circle', [25, 50, 50])).toBe(null);
    expect(form.drawings.map((d) => d.shape)).toEqual(['box', 'line', 'circle']);
    session.desktop.callMethod(form.handle, 'Cls', []);
    expect(form.drawings).toHaveLength(0);

    // the text measurements are worked out from the font
    expect(session.desktop.callMethod(form.handle, 'TextHeight', ['x'])).toBeGreaterThan(0);
    expect(session.desktop.callMethod(form.handle, 'TextWidth', ['12345'])).toBeGreaterThan(
      session.desktop.callMethod(form.handle, 'TextWidth', ['1']) as number,
    );

    // a property added at runtime, and one put back the way it started
    expect(session.desktop.callMethod(txt.handle, 'AddProperty', ['cNote', 'hello'])).toBe(true);
    expect(txt.get('cNote')).toBe('hello');
    txt.set('Width', 999);
    session.desktop.callMethod(txt.handle, 'ResetToDefault', ['Width']);
    expect(txt.get('Width')).toBe(100);

    // the source of a method, read and written
    const source = session.desktop.callMethod(child(form, 'cmdSayHi').handle, 'ReadMethod', ['Click']);
    expect(String(source)).toContain('THISFORM');
    session.desktop.callMethod(txt.handle, 'WriteMethod', ['Click', '? hi']);
    expect(session.desktop.callMethod(txt.handle, 'ReadMethod', ['Click'])).toBe('? hi');

    // ZOrder puts a control at the front of its container
    const first = form.children[0]!;
    session.desktop.callMethod(first.handle, 'ZOrder', [0]);
    expect(form.children[form.children.length - 1]).toBe(first);
  });
});
