import { beforeEach, describe, expect, it } from 'vitest';
import { createFormDesignerStore, FORM_ID, getRect, selectCanRedo, selectCanUndo, selectIsDirty, setClipboard, type FormDesignerStore } from '@renderer/designer/store/createFormDesignerStore';
import { findNode } from '@shared/form/tree';
import { sampleForm } from '../helpers/fixtures';

let store: FormDesignerStore;
let counter = 0;
const s = () => store.getState();
const node = (id: string) => findNode(s().doc.form, id)!;

beforeEach(() => {
  counter = 0;
  store = createFormDesignerStore(sampleForm(), { newId: () => `id${++counter}` });
  setClipboard(null);
});

describe('form designer store', () => {
  it('adds controls with VFP names, auto children and selection', () => {
    const id = s().addControl('TextBox', { left: 10, top: 20 })!;
    expect(node(id)).toMatchObject({ name: 'Text2', props: { Left: 10, Top: 20 } });
    expect(s().selection).toEqual([id]);

    const pf = s().addControl('PageFrame', { left: 0, top: 0 }, { size: { width: 300, height: 200 } })!;
    expect(node(pf).name).toBe('Pageframe2');
    expect(node(pf).props).toMatchObject({ Width: 300, Height: 200 });
    expect(node(pf).children!.map((c) => [c.type, c.name, c.props['Caption']])).toEqual([
      ['Page', 'Page1', 'Page1'],
      ['Page', 'Page2', 'Page2'],
    ]);
    const og = s().addControl('OptionGroup', { left: 0, top: 0 })!;
    expect(node(og).children!.map((c) => c.props['Top'])).toEqual([5, 22]);
    const grid = s().addControl('Grid', { left: 0, top: 0 })!;
    expect(node(grid).children![0]!.children![0]!.type).toBe('Header');

    // containment rules
    expect(s().addControl('Page', { left: 0, top: 0 })).toBeNull();
    expect(s().addControl('TextBox', { left: 0, top: 0 }, { parentId: pf })).toBeNull();
    const inPage = s().addControl('TextBox', { left: 0, top: 0 }, { parentId: node(pf).children![0]!.id })!;
    expect(node(inPage).name).toBe('Text1'); // names are unique per container, like VFP
    expect(s().addControl('CommandButton', { left: 0, top: 0 }, { parentId: 'nope' })).toBeNull();
  });

  it('syncs auto children when the count property changes', () => {
    s().setProp(['Pageframe1'], 'PageCount', 4);
    expect(node('Pageframe1').children!.map((c) => c.name)).toEqual(['Page1', 'Page2', 'Page3', 'Page4']);
    s().setProp(['Pageframe1'], 'PageCount', 1);
    expect(node('Pageframe1').children!.map((c) => c.name)).toEqual(['Page1']);
    expect(findNode(s().doc.form, 'Label1')).toBeDefined(); // lives on Page1
    s().setProp(['Pageframe1'], 'PageCount', 0);
    expect(findNode(s().doc.form, 'Label1')).toBeUndefined();
    s().undo();
    expect(findNode(s().doc.form, 'Label1')).toBeDefined();
  });

  it('edits props, names and methods on controls and the form', () => {
    s().setProp(['Command1', 'Command2'], 'Enabled', false);
    expect(node('Command1').props['Enabled']).toBe(false);
    expect(node('Command2').props['Enabled']).toBe(false);
    s().setProp([FORM_ID], 'Caption', 'Hi');
    expect(s().doc.form.props['Caption']).toBe('Hi');

    expect(s().setName('Command1', 'cmdOk')).toBe(true);
    expect(node('Command1').name).toBe('cmdOk');
    expect(s().setName('Command2', 'CMDOK')).toBe(false); // sibling clash, case-insensitive
    expect(s().setName('Command2', '1bad')).toBe(false);
    expect(s().setName(FORM_ID, 'frmMain')).toBe(true);
    expect(s().doc.form.name).toBe('frmMain');

    s().setMethod('Command1', 'Click', 'RETURN');
    expect(node('Command1').methods['Click']).toBe('RETURN');
    s().setMethod('Command1', 'Click', '');
    expect('Click' in node('Command1').methods).toBe(false);
    s().setMethod(FORM_ID, 'Load', 'SET TALK OFF');
    expect(s().doc.form.methods['Load']).toBe('SET TALK OFF');
  });

  it('moves, resizes, aligns and reorders', () => {
    s().moveBy(['Command1', 'Text1'], 3, -2);
    expect(getRect(node('Command1'))).toMatchObject({ left: 19, top: 14 });
    expect(getRect(node('Text1'))).toMatchObject({ left: 19, top: 62 });
    s().setRects({ Command1: { left: 0, top: 0, width: 100, height: 30 } });
    expect(getRect(node('Command1'))).toEqual({ left: 0, top: 0, width: 100, height: 30 });

    s().select(['Command1', 'Command2', 'Text1']);
    s().align('left');
    expect(node('Command2').props['Left']).toBe(0);
    expect(node('Text1').props['Left']).toBe(0);
    s().sameSize('both');
    expect(getRect(node('Text1'))).toMatchObject({ width: 100, height: 30 });

    s().setZOrder(['Command1'], 'front');
    expect(s().doc.form.children[s().doc.form.children.length - 1]!.id).toBe('Command1');
    s().setZOrder(['Command1'], 'back');
    expect(s().doc.form.children[0]!.id).toBe('Command1');
  });

  it('reparents with containment checks and name dedupe', () => {
    expect(s().reparent('Command1', 'Pageframe1')).toBe(false); // pageframes only take pages
    expect(s().reparent('Command1', 'Page1')).toBe(true);
    expect(node('Page1').children!.map((c) => c.id)).toEqual(['Label1', 'Command1']);
    expect(s().reparent('Command1', null, 0)).toBe(true);
    expect(s().doc.form.children[0]!.id).toBe('Command1');
    // moving a control next to a sibling with the same name renames it
    const extra = s().addControl('Label', { left: 0, top: 0 })!; // Label1 at form level
    expect(node(extra).name).toBe('Label1');
    expect(s().reparent('Label1', null)).toBe(true);
    expect(node('Label1').name).toBe('Label2');
  });

  it('cut/copy/paste with offsets and fresh names, into the form or a container', () => {
    s().select(['Command1', 'Text1']);
    s().copy();
    const pasted = s().paste();
    expect(pasted).toHaveLength(2);
    expect(s().selection).toEqual(pasted);
    expect(node(pasted[0]!)).toMatchObject({ name: 'Command3', props: { Left: 24, Top: 24, Caption: 'OK' } });
    expect(node(pasted[1]!)).toMatchObject({ name: 'Text2' });
    expect(node(pasted[0]!).methods['Click']).toBe('WAIT WINDOW "ok"');

    // paste into a selected page
    s().select(['Page2']);
    const inPage = s().paste();
    expect(node('Page2').children!.map((c) => c.id)).toEqual(inPage);
    expect(node(inPage[0]!).name).toBe('Command1'); // free inside Page2

    s().select(['Command2']);
    s().cut();
    expect(findNode(s().doc.form, 'Command2')).toBeUndefined();
    expect(s().selection).toEqual([]);
    const back = s().paste();
    expect(node(back[0]!)).toMatchObject({ name: 'Command2', props: { Left: 128, Top: 24 } });

    // pages cannot be copied on their own; nested selection copies only the top-most nodes
    s().select(['Page1', 'Pageframe1']);
    s().copy();
    expect(s().paste()).toHaveLength(1);
  });

  it('deletes selected controls but never auto children', () => {
    s().select(['Page1', 'Command1']);
    s().removeSelected();
    expect(findNode(s().doc.form, 'Command1')).toBeUndefined();
    expect(findNode(s().doc.form, 'Page1')).toBeDefined();
    expect(s().selection).toEqual([]);
  });

  it('selects in replace/add/toggle modes, all, and by marquee', () => {
    s().select(['Command1']);
    s().select(['Command2'], 'add');
    expect(s().selection).toEqual(['Command1', 'Command2']);
    s().select(['Command1'], 'toggle');
    expect(s().selection).toEqual(['Command2']);
    s().select(['ghost']);
    expect(s().selection).toEqual([]);
    s().selectAll();
    expect(s().selection).toEqual(['Command1', 'Command2', 'Text1', 'Pageframe1', 'Timer1']);
    s().marqueeSelect({ left: 0, top: 0, width: 110, height: 30 });
    expect(s().selection).toEqual(['Command1']);
    s().selectAll('Page1');
    expect(s().selection).toEqual(['Label1']);
  });

  it('undo/redo restores document and selection, transactions coalesce drags, dirty follows saves', () => {
    expect(selectIsDirty(s())).toBe(false);
    s().select(['Command1']);
    s().setProp(['Command1'], 'Caption', 'A');
    s().select(['Command2']);
    s().setProp(['Command2'], 'Caption', 'B');
    expect(selectIsDirty(s())).toBe(true);
    s().undo();
    expect(node('Command2').props['Caption']).toBe('Cancel');
    expect(s().selection).toEqual(['Command2']);
    s().undo();
    expect(node('Command1').props['Caption']).toBe('OK');
    expect(s().selection).toEqual(['Command1']);
    expect(selectCanUndo(s())).toBe(false);
    expect(selectCanRedo(s())).toBe(true);
    s().redo();
    expect(node('Command1').props['Caption']).toBe('A');

    s().beginTxn('Drag');
    for (let i = 1; i <= 5; i++) s().setRects({ Command1: { left: i * 10, top: 0, width: 84, height: 27 } });
    expect(selectCanUndo(s())).toBe(false); // still inside the transaction
    s().endTxn();
    expect(node('Command1').props['Left']).toBe(50);
    s().undo();
    expect(node('Command1').props['Left']).toBe(16);
    s().redo();

    s().markSaved();
    expect(selectIsDirty(s())).toBe(false);
    s().undo();
    expect(selectIsDirty(s())).toBe(true);
    s().redo();
    expect(selectIsDirty(s())).toBe(false);

    const id = s().addControl('Label', { left: 0, top: 0 })!;
    expect(s().selection).toEqual([id]);
    s().undo();
    expect(findNode(s().doc.form, id)).toBeUndefined();
    expect(s().selection).toEqual(['Command1']); // what was selected before the add
    s().redo();
    expect(s().selection).toEqual([id]);
  });

  it('load replaces everything', () => {
    s().setProp(['Command1'], 'Caption', 'x');
    s().load(sampleForm());
    expect(selectIsDirty(s())).toBe(false);
    expect(selectCanUndo(s())).toBe(false);
    expect(node('Command1').props['Caption']).toBe('OK');
  });
});
