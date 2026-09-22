/**
 * Classes that live in a `.vcx`, reached while a program runs.
 *
 * The library read here is `crates/foxvm/tests/fixtures/golden/fdvclasses.vcx`, written by the
 * product itself from `fdvclasses.gen.prg` beside it; the goldens that measure the language
 * side of `SET CLASSLIB` read the same file, so both halves are asked about one library rather
 * than two descriptions of one.
 *
 * What each class in it holds is in the generator: `fdvgreeter` a property and an Init and a
 * method, `fdvchild` a subclass of it in the same file, `fdvpanel` a container with a button
 * inside it, `fdvbutton` a control class.
 */

import { readFileSync } from 'node:fs';
import { beforeAll, beforeEach, describe, expect, it } from 'vitest';
import { setApi } from '@renderer/api/foxdev';
import { createMemoryApi, type MemoryApi } from '@renderer/api/memoryApi';
import { useProjectStore } from '@renderer/stores/projectStore';
import { useSessionStore } from '@renderer/runtime/session';
import { createProjectSource } from '@renderer/runtime/projectSource';
import { compileForm } from '@renderer/runtime/vmBridge';
import { formMethodSources, requireBytes, type ProgramSource } from '@shared/runtime/programSource';
import type { FormDocument } from '@shared/form/schema';
import { loadFoxVm } from '../../src/wasm/foxvm/loader';

const FIXTURES = 'crates/foxvm/tests/fixtures/golden';
const HOME = 'C:/work';
const source = createProjectSource();

const printed = () => useSessionStore.getState().output.filter((o) => o.kind === 'output').map((o) => o.text);

/** Runs a program in the session and answers with what it printed, line by line. */
async function run(...lines: string[]): Promise<string[]> {
  useSessionStore.setState({ output: [] });
  await useSessionStore.getState().execute(source, lines.join('\n'));
  return printed();
}

function seed(api: MemoryApi, name: string): void {
  for (const ext of ['vcx', 'vct']) {
    api.binary$.set(`${HOME}/${name}.${ext}`, new Uint8Array(readFileSync(`${FIXTURES}/fdvclasses.${ext}`)));
  }
}

beforeAll(async () => {
  await loadFoxVm();
});

beforeEach(() => {
  const api = createMemoryApi();
  seed(api, 'fdvclasses');
  setApi(api);
  useProjectStore.setState({ path: `${HOME}/work.fxproject`, doc: null });
  useSessionStore.getState().cancel();
  useSessionStore.setState({ output: [] });
});

describe('a class out of a class library', () => {
  it('is a real object: its properties, its Init, and its own methods', async () => {
    const said = await run(
      'SET CLASSLIB TO fdvclasses',
      'oThing = CREATEOBJECT("fdvgreeter")',
      '? oThing.Greet()',
      '? oThing.Class, oThing.BaseClass',
      '? oThing.nCount',
      'oThing.nCount = 9',
      '? oThing.Greet()',
    );
    // the Init ran: cGreeting starts as "hello" in the file and the Init writes over it
    expect(said[0]).toBe('hello from the library (2)');
    expect(said[1]).toBe('Fdvgreeter Custom');
    expect(said[2]?.trim()).toBe('2');
    expect(said[3]).toBe('hello from the library (9)');
  });

  it('carries what its parent class in the same library gave it', async () => {
    const said = await run(
      'SET CLASSLIB TO fdvclasses',
      'oChild = CREATEOBJECT("fdvchild")',
      '? oChild.Shout()',
      '? oChild.Class, oChild.ParentClass',
    );
    // Shout is fdvchild's, Greet and cGreeting are fdvgreeter's, and nCount is overridden to 5
    expect(said[0]).toBe('HELLO FROM THE LIBRARY (5)');
    expect(said[1]).toBe('Fdvchild Fdvgreeter');
  });

  it('brings the objects inside it, and its Init can reach them', async () => {
    const said = await run(
      'SET CLASSLIB TO fdvclasses',
      'oPanel = CREATEOBJECT("fdvpanel")',
      '? oPanel.ControlCount',
      '? oPanel.cmdgo.Caption',
      '? oPanel.cmdgo.Parent.Class',
    );
    expect(said[0]?.trim()).toBe('1');
    // the container's own Init wrote on the member, which means the member existed first
    expect(said[1]).toBe('ready');
    expect(said[2]).toBe('Fdvpanel');
  });

  it('is forgotten when the libraries are', async () => {
    const said = await run(
      'SET CLASSLIB TO fdvclasses',
      '? VARTYPE(CREATEOBJECT("fdvgreeter"))',
      'SET CLASSLIB TO',
      'TRY',
      '  oGone = CREATEOBJECT("fdvgreeter")',
      'CATCH TO oErr',
      '  ? oErr.ErrorNo, oErr.Message',
      'ENDTRY',
    );
    expect(said[0]).toBe('O');
    expect(said[1]).toBe('      1733 Class definition FDVGREETER is not found.');
  });

  it('is found among the Foundation Classes when it is nowhere else', async () => {
    // a program written against Visual FoxPro names one of those by file alone, because the
    // product finds them on its own search path; the copies FoxDev ships stand in for that
    const api = createMemoryApi();
    for (const ext of ['vcx', 'vct']) {
      api.binary$.set(`${api.classLibraryDir$}/ffclib.${ext}`, new Uint8Array(readFileSync(`${FIXTURES}/fdvclasses.${ext}`)));
    }
    setApi(api);
    const said = await run('SET CLASSLIB TO ffclib', '? CREATEOBJECT("fdvgreeter").Greet()');
    expect(said[0]).toBe('hello from the library (2)');
  });

  it('says which file it could not find', async () => {
    const said = await run(
      'TRY',
      '  SET CLASSLIB TO nosuchlib',
      'CATCH TO oErr',
      '  ? oErr.ErrorNo, oErr.Message',
      'ENDTRY',
      '? "[" + SET("CLASSLIB") + "]"',
    );
    expect(said[0]).toContain('1 File');
    expect(said[0]).toContain('nosuchlib.vcx');
    expect(said[1]).toBe('[]');
  });
});

describe('NEWOBJECT() naming the file', () => {
  it('reads a library nothing has loaded', async () => {
    const said = await run(
      'oB = NEWOBJECT("fdvbutton", "fdvclasses.vcx")',
      '? oB.Caption, oB.Class, oB.BaseClass',
      '? "[" + SET("CLASSLIB") + "]"',
      '? JUSTFNAME(oB.ClassLibrary)',
    );
    // measured: a class name comes back with only its first letter upper-cased, base classes
    // included - "Commandbutton", which is how the measured table of base classes spells it too
    expect(said[0]).toBe('Library button Fdvbutton Commandbutton');
    // measured: the file is read for the call and does not join the loaded libraries
    expect(said[1]).toBe('[]');
    expect(said[2]).toBe('fdvclasses.vcx');
  });

  it('refuses a class the file has not got', async () => {
    const said = await run(
      'TRY',
      '  oNope = NEWOBJECT("nosuchclass", "fdvclasses.vcx")',
      'CATCH TO oErr',
      '  ? oErr.ErrorNo, oErr.Message',
      'ENDTRY',
    );
    expect(said[0]).toBe('      1733 Class definition NOSUCHCLASS is not found.');
  });
});

describe('a form looking for a library beside itself', () => {
  /** A source that serves one form, the way the IDE serves the document it has open. */
  function formNamed(doc: FormDocument): ProgramSource {
    return {
      async getForm() {
        return { name: doc.form.name, doc, bytes: requireBytes(doc.form.name, compileForm(doc.form.name, formMethodSources(doc))) };
      },
      async getProgram() {
        return null;
      },
      async getMenu() {
        return null;
      },
    };
  }

  it('finds it through the folder the form was read from', async () => {
    // What every sample Visual FoxPro ships does in its first Init, through the c_solutions
    // object it carries: ask where the form came from, make that the default directory, and name
    // everything else relative to it. Without SYS(1271) the form is nowhere and `..\fdvclasses`
    // is read from the project folder, which is one directory too high.
    const doc: FormDocument = {
      $schema: 'foxdev-form',
      version: 1,
      form: {
        name: 'frmDeep',
        props: {},
        methods: {
          Init: [
            'SET DEFAULT TO (JUSTPATH(SYS(1271, THISFORM)))',
            'SET CLASSLIB TO ..\\fdvclasses',
            'THISFORM.AddProperty("oThing", .F.)',
            'THISFORM.oThing = CREATEOBJECT("fdvgreeter")',
            '? THISFORM.oThing.Greet()',
          ].join('\n'),
        },
        children: [],
      },
      meta: { vfp: { source: 'sub/deep.scx' } },
    };
    useSessionStore.setState({ output: [] });
    await useSessionStore.getState().execute(formNamed(doc), 'DO FORM deep NOSHOW');
    expect(printed()[0]).toBe('hello from the library (2)');
  });
});

describe('a container making one of its own', () => {
  it('adds the class named as a member, hidden, with its methods running', async () => {
    const said = await run(
      'oForm = CREATEOBJECT("Form")',
      '? oForm.NewObject("thebutton", "fdvbutton", "fdvclasses.vcx")',
      '? oForm.ControlCount, oForm.thebutton.Name',
      '? oForm.thebutton.Visible',
      '? oForm.thebutton.Caption, oForm.thebutton.cTag',
      'oForm.thebutton.Click()',
      '? oForm.thebutton.Caption',
      '? oForm.thebutton.Parent.BaseClass',
    );
    // measured in Visual FoxPro 9: the call answers .T., the name is upper-cased, and the
    // member arrives hidden the way AddObject's does
    expect(said[0]).toBe('.T.');
    expect(said[1]).toBe('         1 THEBUTTON');
    expect(said[2]).toBe('.F.');
    expect(said[3]).toBe('Library button from the library');
    // the class's own Click ran, out of the library's module rather than the form's
    expect(said[4]).toBe('clicked');
    expect(said[5]).toBe('Form');
  });

  it('takes a base class with no file, as AddObject does', async () => {
    const said = await run(
      'oForm = CREATEOBJECT("Form")',
      '? oForm.NewObject("plain", "CommandButton")',
      '? oForm.plain.Name, oForm.plain.Visible',
    );
    expect(said[0]).toBe('.T.');
    expect(said[1]).toBe('PLAIN .F.');
  });

  it('reports a class no library has', async () => {
    const said = await run(
      'oForm = CREATEOBJECT("Form")',
      'TRY',
      '  oForm.NewObject("nope", "nosuchclass")',
      'CATCH TO oErr',
      '  ? oErr.ErrorNo, oErr.Message',
      'ENDTRY',
      '? oForm.ControlCount',
    );
    expect(said[0]).toBe('      1733 Class definition NOSUCHCLASS is not found.');
    expect(said[1]?.trim()).toBe('0');
  });
});
