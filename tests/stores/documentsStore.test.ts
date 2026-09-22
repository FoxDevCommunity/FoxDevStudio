import { beforeEach, describe, expect, it } from 'vitest';
import { docTitle, useDocumentsStore } from '@renderer/stores/documentsStore';
import { FORM_ID } from '@renderer/designer/store/createFormDesignerStore';
import { sampleForm } from '../helpers/fixtures';

beforeEach(() => useDocumentsStore.getState().closeAll());

describe('documents store', () => {
  it('opens method tabs tied to a form and closes them with it', () => {
    const d = () => useDocumentsStore.getState();
    const f = d().openForm(sampleForm(), '/p/Form1.fxf');
    const m = d().openMethod(f, 'Command1', 'Click');
    expect(d().openMethod(f, 'Command1', 'Click')).toBe(m);
    const fm = d().openMethod(f, FORM_ID, 'Init');
    expect(docTitle(d().docs[m]!)).toBe('Form1.Command1.Click');
    expect(docTitle(d().docs[fm]!)).toBe('Form1.Init');
    d().retargetMethod(m, 'Text1', 'Valid');
    expect(docTitle(d().docs[m]!)).toBe('Form1.Text1.Valid');
    expect(d().order).toEqual([f, m, fm]);
    d().activate(m);
    expect(d().activeId).toBe(m);
    d().close(f);
    expect(d().order).toEqual([]);
    expect(d().activeId).toBeNull();
  });

  it('keeps a single runtime desktop tab, independent of any form', () => {
    const d = () => useDocumentsStore.getState();
    const f = d().openForm(sampleForm(), '/p/Form1.fxf');
    const screen = d().openDesktop();
    expect(d().openDesktop()).toBe(screen);
    expect(docTitle(d().docs[screen]!)).toBe('Screen');
    // closing the form leaves the desktop open: it is not owned by a document
    d().close(f);
    expect(d().order).toEqual([screen]);
  });

  it('picks a sensible neighbour when the active tab closes', () => {
    const d = () => useDocumentsStore.getState();
    const a = d().openProgram('', '/a.prg');
    const b = d().openProgram('', '/b.prg');
    const c = d().openProgram('', '/c.prg');
    d().activate(b);
    d().close(b);
    expect(d().activeId).toBe(c);
    d().close(c);
    expect(d().activeId).toBe(a);
    d().close('missing');
    expect(d().order).toEqual([a]);
    expect(docTitle(d().docs[a]!)).toBe('a.prg');
  });
});
