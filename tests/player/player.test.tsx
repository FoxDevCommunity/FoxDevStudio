import { readFileSync } from 'node:fs';
import { render, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeAll, beforeEach, describe, expect, it } from 'vitest';
import { parseProjectDocument } from '@shared/project/serialize';
import { parseFormDocument } from '@shared/form/serialize';
import { parseMenuDocument } from '@shared/menu/serialize';
import { packBundle, parseBundle, stringifyBundle, type AppBundle } from '@shared/runtime/bundle';
import { setApi } from '@renderer/api/foxdev';
import { createMemoryApi } from '@renderer/api/memoryApi';
import { compileForm, compileProgram } from '@renderer/runtime/vmBridge';
import { useSessionStore } from '@renderer/runtime/session';
import { PlayerApp } from '@renderer/player/PlayerApp';
import { loadFoxVm } from '../../src/wasm/foxvm/loader';

/** The player runs a bundle built from the real sample project - the same bytes Build App emits. */
const sample = (file: string) => readFileSync(`resources/samples/${file}`, 'utf8');
const SOURCES: Record<string, string> = {
  'HelloWorld.fxf': sample('HelloWorld.fxf'),
  'Main.fxm': sample('Main.fxm'),
  'main.prg': sample('main.prg'),
};

let bundle: AppBundle;
let bundleText: string;

beforeAll(async () => {
  const vm = await loadFoxVm();
  const project = parseProjectDocument(sample('HelloWorld.fxproject'));
  if (!project.ok) throw new Error(project.error);
  const packed = await packBundle(
    {
      project: project.doc,
      readItem: (relative) => {
        const text = SOURCES[relative];
        if (text === undefined) throw new Error(`missing sample ${relative}`);
        return Promise.resolve(text);
      },
      parseForm: parseFormDocument,
      parseMenu: parseMenuDocument,
    },
    { compileForm: (name, methods) => compileForm(name, methods), compileProgram: (src, name) => compileProgram(src, name), vmVersion: vm.version() },
  );
  bundle = packed.bundle;
  bundleText = stringifyBundle(bundle);
});

/** The in-memory API serves the bundle from `bundle$`, exactly as the preload serves the real one. */
function installBundle(loaded: { path: string; text: string } | null) {
  const api = createMemoryApi();
  api.bundle$ = loaded;
  setApi(api);
}

beforeEach(() => {
  useSessionStore.getState().cancel();
  useSessionStore.setState({ output: [], history: [] });
});

describe('runtime player', () => {
  it('runs the bundled main form and its Say Hi button', async () => {
    installBundle({ path: '/app.fxa', text: bundleText });
    render(<PlayerApp />);

    const dialog = await screen.findByRole('dialog', { name: 'Hello, World' });
    const name = within(dialog).getByRole('textbox', { name: 'txtName' });
    await userEvent.type(name, 'jorge');
    await userEvent.click(within(dialog).getByRole('checkbox', { name: 'Shout it' }));
    await userEvent.click(within(dialog).getByRole('button', { name: 'Say Hi' }));

    expect(await within(dialog).findByText('HELLO, JORGE!')).toBeInTheDocument();
    expect(document.title).toBe(bundle.name);
  });

  it('reports that the application has ended once the last form closes', async () => {
    installBundle({ path: '/app.fxa', text: bundleText });
    render(<PlayerApp />);

    const dialog = await screen.findByRole('dialog', { name: 'Hello, World' });
    await userEvent.click(within(dialog).getByRole('button', { name: 'Close' }));

    await waitFor(() => expect(screen.queryByRole('dialog', { name: 'Hello, World' })).toBeNull());
    expect(await screen.findByText('The application has ended.')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Quit' })).toBeInTheDocument();
  });

  it('explains how to run an application when no bundle was loaded', async () => {
    installBundle(null);
    render(<PlayerApp />);

    expect(await screen.findByText('No application bundle was loaded.')).toBeInTheDocument();
    expect(screen.getByText(/FoxDevRuntime\.exe --play app\.fxa/)).toBeInTheDocument();
  });

  it('shows the parse error for a corrupt bundle', async () => {
    installBundle({ path: '/broken.fxa', text: '{ not json' });
    render(<PlayerApp />);

    expect(await screen.findByText('Not a FoxDev application bundle: invalid JSON')).toBeInTheDocument();
  });
});

describe('bundle round trip', () => {
  it('packs every project item and points main at the form', () => {
    const parsed = parseBundle(bundleText);
    expect(parsed.ok).toBe(true);
    if (!parsed.ok) return;

    expect(Object.keys(parsed.doc.forms)).toEqual(['helloworld']);
    expect(Object.keys(parsed.doc.menus)).toEqual(['main']);
    expect(Object.keys(parsed.doc.programs)).toEqual(['main']);
    expect(parsed.doc.main).toEqual({ kind: 'form', name: 'HelloWorld' });
    expect(parsed.doc.name).toBe('HelloWorld');
    expect(parsed.doc.forms.helloworld?.doc.form.name).toBe('frmHello');
    expect(parsed.doc.programs.main?.name).toBe('main');
  });
});
