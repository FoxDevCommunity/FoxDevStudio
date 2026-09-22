import { applyPatches, enablePatches, produceWithPatches, type Draft, type Patch } from 'immer';

enablePatches();

/**
 * Generic undo/redo over an immutable document using immer patches.
 * `E` is an extra snapshot restored alongside the document (the designer stores the selection there).
 * Pure functions: the caller keeps the state and writes back the results.
 */
export interface UndoEntry<E> {
  label: string;
  patches: Patch[];
  inverse: Patch[];
  before: E;
  after: E;
}

export interface UndoState<E> {
  past: UndoEntry<E>[];
  future: UndoEntry<E>[];
  /** past.length at the last save; -1 when the saved state is no longer reachable. */
  savedIndex: number;
  /** Open transaction: commits are coalesced into one entry until endTxn. */
  txn: { label: string; patches: Patch[]; inverse: Patch[]; before: E } | null;
  limit: number;
}

export function createUndoState<E>(limit = 200): UndoState<E> {
  return { past: [], future: [], savedIndex: 0, txn: null, limit };
}

export interface ChangeResult<D, E> {
  doc: D;
  history: UndoState<E>;
  changed: boolean;
}

export function applyChange<D extends object, E>(
  history: UndoState<E>,
  doc: D,
  label: string,
  before: E,
  after: E,
  recipe: (draft: Draft<D>) => void,
): ChangeResult<D, E> {
  const [next, patches, inverse] = produceWithPatches(doc, recipe);
  if (patches.length === 0) return { doc, history, changed: false };
  if (history.txn) {
    const txn = { ...history.txn, patches: [...history.txn.patches, ...patches], inverse: [...inverse, ...history.txn.inverse] };
    return { doc: next as D, history: { ...history, txn }, changed: true };
  }
  return { doc: next as D, history: pushEntry(history, { label, patches, inverse, before, after }), changed: true };
}

function pushEntry<E>(history: UndoState<E>, entry: UndoEntry<E>): UndoState<E> {
  let past = [...history.past, entry];
  let savedIndex = history.savedIndex;
  if (past.length > history.limit) {
    const trimmed = past.length - history.limit;
    past = past.slice(trimmed);
    savedIndex = savedIndex - trimmed;
  }
  // the saved state lived in the future we are discarding, or fell off the end
  if (savedIndex < 0 || (history.future.length > 0 && savedIndex > past.length - 1)) savedIndex = -1;
  return { ...history, past, future: [], savedIndex };
}

export function beginTxn<E>(history: UndoState<E>, label: string, before: E): UndoState<E> {
  if (history.txn) return history;
  return { ...history, txn: { label, patches: [], inverse: [], before } };
}

export function endTxn<E>(history: UndoState<E>, after: E): UndoState<E> {
  const txn = history.txn;
  if (!txn) return history;
  const closed = { ...history, txn: null };
  if (txn.patches.length === 0) return closed;
  return pushEntry(closed, { label: txn.label, patches: txn.patches, inverse: txn.inverse, before: txn.before, after });
}

export function undo<D extends object, E>(history: UndoState<E>, doc: D): { doc: D; history: UndoState<E>; extra: E } | null {
  if (history.txn || history.past.length === 0) return null;
  const entry = history.past[history.past.length - 1]!;
  return {
    doc: applyPatches(doc, entry.inverse) as D,
    history: { ...history, past: history.past.slice(0, -1), future: [entry, ...history.future] },
    extra: entry.before,
  };
}

export function redo<D extends object, E>(history: UndoState<E>, doc: D): { doc: D; history: UndoState<E>; extra: E } | null {
  if (history.txn || history.future.length === 0) return null;
  const entry = history.future[0]!;
  return {
    doc: applyPatches(doc, entry.patches) as D,
    history: { ...history, past: [...history.past, entry], future: history.future.slice(1) },
    extra: entry.after,
  };
}

export function canUndo<E>(history: UndoState<E>): boolean {
  return !history.txn && history.past.length > 0;
}

export function canRedo<E>(history: UndoState<E>): boolean {
  return !history.txn && history.future.length > 0;
}

export function undoLabel<E>(history: UndoState<E>): string | null {
  return history.past.length ? history.past[history.past.length - 1]!.label : null;
}

export function redoLabel<E>(history: UndoState<E>): string | null {
  return history.future.length ? history.future[0]!.label : null;
}

export function isDirty<E>(history: UndoState<E>): boolean {
  return history.past.length !== history.savedIndex || history.txn !== null;
}

export function markSaved<E>(history: UndoState<E>): UndoState<E> {
  return { ...history, savedIndex: history.past.length };
}
