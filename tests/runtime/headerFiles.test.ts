/**
 * Finding the header file a form or a class library names, and compiling its methods with it.
 *
 * Measured in Visual FoxPro 9 by building forms whose eighth reserved field names a header and
 * putting the file in one place at a time: a form finds its header beside itself first and in
 * the default directory after that, a header it cannot find is not an error (the form compiles
 * and the constant is an unknown name only when a line using it runs), a header a header
 * includes is found beside the one that asked for it, and a constant answers to any spelling of
 * its name. The same run showed the scope: a method taken from a class library is compiled with
 * that library's header and never with the form's.
 */

import { beforeAll, beforeEach, describe, expect, it } from 'vitest';
import { setApi } from '@renderer/api/foxdev';
import { createMemoryApi, type MemoryApi } from '@renderer/api/memoryApi';
import { readHeaderFiles } from '@renderer/runtime/headerFiles';
import { compileForm } from '@renderer/runtime/vmBridge';
import { formHeaderRefs, formMethodSources } from '@shared/runtime/programSource';
import type { FormDocument } from '@shared/form/schema';
import { loadFoxVm } from '../../src/wasm/foxvm/loader';

const FORMS = 'C:/work/forms';
const HOME = 'C:/work';

let api: MemoryApi;

beforeAll(async () => {
  await loadFoxVm();
});

beforeEach(() => {
  api = createMemoryApi();
  setApi(api);
});

/** A one-method form whose file named `include`. */
function formNaming(include: string, source: string): FormDocument {
  return {
    $schema: 'foxdev-form',
    version: 1,
    form: { name: 'Form1', props: {}, methods: { Init: source }, children: [] },
    meta: { vfp: { include } },
  };
}

describe('where a form finds the header file it names', () => {
  it('takes the copy beside the form over the one in the default directory', async () => {
    api.files$.set(`${FORMS}/inc.h`, '#DEFINE WORD "beside the form"\r\n');
    api.files$.set(`${HOME}/inc.h`, '#DEFINE WORD "in the default directory"\r\n');

    const headers = await readHeaderFiles(['inc.h'], [FORMS, HOME]);
    expect(headers['INC']).toContain('beside the form');
  });

  it('falls back to the default directory', async () => {
    api.files$.set(`${HOME}/inc.h`, '#DEFINE WORD "in the default directory"\r\n');

    const headers = await readHeaderFiles(['inc.h'], [FORMS, HOME]);
    expect(headers['INC']).toContain('in the default directory');
  });

  it('follows a folder on the name, and a full path as it stands', async () => {
    api.files$.set(`${HOME}/up.h`, '#DEFINE WORD "one up"\r\n');
    api.files$.set('C:/elsewhere/abs.h', '#DEFINE WORD "absolute"\r\n');

    expect((await readHeaderFiles(['../up.h'], [FORMS, HOME]))['UP']).toContain('one up');
    expect((await readHeaderFiles(['C:/elsewhere/abs.h'], [FORMS]))['ABS']).toContain('absolute');
  });

  it('brings in a header the header includes, looked for beside it', async () => {
    api.files$.set(`${FORMS}/outer.h`, '#INCLUDE inner.h\r\n#DEFINE OUTER "outer"\r\n');
    api.files$.set(`${FORMS}/inner.h`, '#DEFINE INNER "inner"\r\n');

    const headers = await readHeaderFiles(['outer.h'], [FORMS, HOME]);
    expect(Object.keys(headers).sort()).toEqual(['INNER', 'OUTER']);
  });

  it('says nothing at all about one it cannot find', async () => {
    expect(await readHeaderFiles(['missing.h'], [FORMS, HOME])).toEqual({});
  });
});

describe('compiling a form with the header it names', () => {
  it('resolves a constant from the header, whatever the spelling', async () => {
    // a constant that stands for a keyword, so the method only parses at all if the header was
    // read, and is written in one case and used in another, because names are not case sensitive
    api.files$.set(`${FORMS}/inc.h`, '#DEFINE EndLoop ENDFOR\r\n');
    const doc = formNaming('inc.h', 'FOR i = 1 TO 2\n? i\nENDLOOP');

    const headers = await readHeaderFiles(formHeaderRefs(doc), [FORMS, HOME]);
    const out = compileForm('Form1', formMethodSources(doc), headers);
    expect(out.methodDiagnostics ?? []).toEqual([]);
    expect(out.bytes).toBeTruthy();
  });

  it('and without the header the same method will not compile', async () => {
    const doc = formNaming('inc.h', 'FOR i = 1 TO 2\n? i\nENDLOOP');

    const out = compileForm('Form1', formMethodSources(doc), {});
    expect(out.bytes).toBeFalsy();
  });

  it('compiles a form whose header is nowhere to be found, as the product does', async () => {
    const doc = formNaming('gone.h', '? SOMECONSTANT');

    const headers = await readHeaderFiles(formHeaderRefs(doc), [FORMS, HOME]);
    const out = compileForm('Form1', formMethodSources(doc), headers);
    expect(out.bytes).toBeTruthy();
  });

  it('keeps a method that names no header out of the form header', async () => {
    api.files$.set(`${FORMS}/inc.h`, '#DEFINE GREETING "hello"\r\n');
    const doc: FormDocument = {
      $schema: 'foxdev-form',
      version: 1,
      form: {
        name: 'Form1',
        props: {},
        methods: { Init: '? GREETING' },
        children: [{ id: 'a', type: 'CommandButton', name: 'cmdOk', props: {}, methods: { Click: '? GREETING' } }],
      },
      // the button came from a class library that named no header of its own
      meta: { vfp: { include: 'inc.h', includes: { 'cmdok.click': '' } } },
    };

    const methods = formMethodSources(doc);
    expect(methods.find((m) => m.objectPath === '')?.include).toBe('INC');
    expect(methods.find((m) => m.objectPath === 'cmdOk')?.include).toBe('');
  });
});
