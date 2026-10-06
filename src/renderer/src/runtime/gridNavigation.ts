/** Navigation for the bounded, in-memory Grid view. No database writes. */
import type { RuntimeObject } from '@shared/runtime/objectModel';
import { useSessionStore } from './session';

const pending = new WeakSet<RuntimeObject>();

function enabled(obj: RuntimeObject): boolean {
  for (let current: RuntimeObject | null = obj; current; current = current.parent) {
    if (!current.alive || current.get('Enabled') === false || current.get('Visible') === false) return false;
  }
  return true;
}

function control(grid: RuntimeObject, column: number): RuntimeObject | undefined {
  const col = grid.children[column - 1];
  return col?.child(String(col.get('CurrentControl') || 'Text1'));
}

/** Record numbers are physical DBF positions, not indexes in the displayed array. */
export async function selectGridCell(grid: RuntimeObject, record: number, column: number, generation: string): Promise<boolean> {
  const state = useSessionStore.getState();
  const vm = state.vm;
  if (!vm || state.scheduler?.isExecuting || pending.has(grid) || !enabled(grid)) return false;
  const alias = String(grid.get('RecordSource') ?? '');
  if (Number(grid.get('RecordSourceType')) !== 1 || column < 1 || column > grid.children.length) return false;
  const before = vm.gridPreview(alias);
  if ('error' in before) throw new Error(before.error);
  if (before.generation !== generation || !before.records.includes(record)) return false;
  const oldColumn = Math.max(1, Number(grid.get('ActiveColumn')) || 1);
  const change = (before.currentRecord !== record ? 1 : 0) | (oldColumn !== column ? 2 : 0);
  if (!change) return true;
  const stillBound = () => enabled(grid) && useSessionStore.getState().vm === vm
    && String(grid.get('RecordSource')) === alias && Number(grid.get('RecordSourceType')) === 1;
  const unchanged = () => {
    if (!stillBound()) return false;
    const current = vm.gridPreview(alias);
    return !('error' in current) && current.generation === generation && current.currentRecord === before.currentRecord;
  };
  pending.add(grid);
  try {
    grid.set('RowColChange', change);
    const entering = control(grid, column);
    const leaving = control(grid, oldColumn);
    const outcome = await grid.desktop.dispatch(grid, 'BeforeRowColChange', [oldColumn]);
    if (outcome?.nodefault || !unchanged()) return false;
    if (leaving) {
      const valid = await grid.desktop.dispatch(leaving, 'Valid', []);
      if (valid?.value === false || !unchanged()) return false;
    }
    vm.gridSelect(alias, record, generation);
    if (entering) {
      const when = await grid.desktop.dispatch(entering, 'When', []);
      if (!stillBound()) return false;
      const current = vm.gridPreview(alias);
      if ('error' in current || current.generation !== generation || current.currentRecord !== record) return false;
      if (when?.value === false) {
        // This view has no alternate editable cell to focus. Restore the prior
        // visible record on rejection rather than displaying a false selection.
        if (current.records.includes(before.currentRecord)) vm.gridSelect(alias, before.currentRecord, generation);
        return false;
      }
    }
    if (!stillBound()) return false;
    const selected = vm.gridPreview(alias);
    if ('error' in selected || selected.generation !== generation || selected.currentRecord !== record) return false;
    grid.set('ActiveRow', selected.records.indexOf(record) + 1);
    grid.set('ActiveColumn', column);
    grid.setFocus();
    if (!stillBound()) return false;
    const focused = vm.gridPreview(alias);
    if ('error' in focused || focused.generation !== generation || focused.currentRecord !== record) return false;
    await grid.desktop.dispatch(grid, 'AfterRowColChange', [column]);
    return true;
  } finally {
    pending.delete(grid);
    // Direct VM movement has no scheduler transition; notify all bound grids.
    useSessionStore.setState(s => ({revision: s.revision + 1}));
  }
}
