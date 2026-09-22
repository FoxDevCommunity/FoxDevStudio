import { describe, expect, it } from 'vitest';
import * as H from '@renderer/stores/history';

interface Doc {
  items: string[];
  n: number;
}

describe('undo history', () => {
  it('records changes with before/after extras and undoes/redoes them', () => {
    let doc: Doc = { items: [], n: 0 };
    let h = H.createUndoState<string>();
    ({ doc, history: h } = H.applyChange(h, doc, 'add a', 'sel0', 'selA', (d) => void d.items.push('a')));
    ({ doc, history: h } = H.applyChange(h, doc, 'set n', 'selA', 'selA', (d) => void (d.n = 5)));
    expect(doc).toEqual({ items: ['a'], n: 5 });
    expect(H.undoLabel(h)).toBe('set n');
    expect(H.isDirty(h)).toBe(true);

    const u = H.undo(h, doc)!;
    expect(u.doc).toEqual({ items: ['a'], n: 0 });
    expect(u.extra).toBe('selA');
    expect(H.redoLabel(u.history)).toBe('set n');
    const u2 = H.undo(u.history, u.doc)!;
    expect(u2.doc).toEqual({ items: [], n: 0 });
    expect(u2.extra).toBe('sel0');
    expect(H.isDirty(u2.history)).toBe(false);
    expect(H.undo(u2.history, u2.doc)).toBeNull();

    const r = H.redo(u2.history, u2.doc)!;
    expect(r.doc).toEqual({ items: ['a'], n: 0 });
    expect(r.extra).toBe('selA');
  });

  it('ignores no-op recipes and truncates redo on new changes', () => {
    let doc: Doc = { items: [], n: 0 };
    let h = H.createUndoState<null>();
    const r0 = H.applyChange(h, doc, 'noop', null, null, () => {});
    expect(r0.changed).toBe(false);
    expect(r0.doc).toBe(doc);
    ({ doc, history: h } = H.applyChange(h, doc, 'a', null, null, (d) => void (d.n = 1)));
    ({ doc, history: h } = H.applyChange(h, doc, 'b', null, null, (d) => void (d.n = 2)));
    ({ doc, history: h } = H.undo(h, doc)!);
    expect(H.canRedo(h)).toBe(true);
    ({ doc, history: h } = H.applyChange(h, doc, 'c', null, null, (d) => void (d.n = 3)));
    expect(H.canRedo(h)).toBe(false);
    expect(h.past.map((e) => e.label)).toEqual(['a', 'c']);
    expect(doc.n).toBe(3);
  });

  it('coalesces a transaction into one entry', () => {
    let doc: Doc = { items: [], n: 0 };
    let h = H.createUndoState<string>();
    h = H.beginTxn(h, 'drag', 'before');
    h = H.beginTxn(h, 'nested-ignored', 'x');
    for (let i = 1; i <= 3; i++) ({ doc, history: h } = H.applyChange(h, doc, 'step', 'x', 'x', (d) => void (d.n = i)));
    expect(h.past).toHaveLength(0);
    expect(H.canUndo(h)).toBe(false);
    h = H.endTxn(h, 'after');
    expect(h.past).toHaveLength(1);
    expect(h.past[0]!.label).toBe('drag');
    const u = H.undo(h, doc)!;
    expect(u.doc.n).toBe(0);
    expect(u.extra).toBe('before');
    expect(H.redo(u.history, u.doc)!.doc.n).toBe(3);
    expect(H.redo(u.history, u.doc)!.extra).toBe('after');
    // an empty transaction leaves no entry
    expect(H.endTxn(H.beginTxn(h, 'empty', 'e'), 'e').past).toHaveLength(1);
  });

  it('tracks the saved point', () => {
    let doc: Doc = { items: [], n: 0 };
    let h = H.createUndoState<null>();
    ({ doc, history: h } = H.applyChange(h, doc, 'a', null, null, (d) => void (d.n = 1)));
    h = H.markSaved(h);
    expect(H.isDirty(h)).toBe(false);
    ({ doc, history: h } = H.undo(h, doc)!);
    expect(H.isDirty(h)).toBe(true);
    ({ doc, history: h } = H.redo(h, doc)!);
    expect(H.isDirty(h)).toBe(false);
    ({ doc, history: h } = H.undo(h, doc)!);
    ({ doc, history: h } = H.applyChange(h, doc, 'b', null, null, (d) => void (d.n = 9)));
    expect(H.isDirty(h)).toBe(true);
    expect(h.savedIndex).toBe(-1);
    expect(doc.n).toBe(9);
  });

  it('enforces the entry limit', () => {
    let doc: Doc = { items: [], n: 0 };
    let h = H.createUndoState<null>(3);
    for (let i = 1; i <= 5; i++) ({ doc, history: h } = H.applyChange(h, doc, `s${i}`, null, null, (d) => void (d.n = i)));
    expect(h.past.map((e) => e.label)).toEqual(['s3', 's4', 's5']);
    expect(H.isDirty(h)).toBe(true);
  });
});
