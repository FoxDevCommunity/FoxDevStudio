import { getApi } from '../../api/foxdev';
import { useSettingsStore } from '../../stores/settingsStore';
import { useDocumentsStore, isFileDocument } from '../../stores/documentsStore';
import { useProjectStore } from '../../stores/projectStore';
import { closeAllDocuments, closeDocument, closeProject, newForm, newMenu, newProgram, newProjectDialog, openFileDialog, openProjectDialog, saveAll, saveDocument } from '../../stores/fileActions';
import { getActiveDoc, getActiveFormStore, ownerFormDoc } from '../activeDocument';
import { basename } from '@shared/paths';
import { useSessionStore } from '../../runtime/session';
import { createProjectSource } from '../../runtime/projectSource';
import { buildApp, buildExecutable } from '../../runtime/buildActions';
import { importVfpProjectDialog } from '../../vfp/importActions';
import { useDebugStore } from '../../runtime/debugSession';
import type { StepMode } from '@shared/runtime/host';
import { registerCommands, type Command } from './registry';
import type { AlignKind, SizeKind } from '../../designer/geometry';

export interface ShellState {
  showExplorer: boolean;
  showProperties: boolean;
  showOutput: boolean;
  showDebugger: boolean;
}

const hasForm = () => !!getActiveFormStore();
const hasSelection = (n = 1) => (getActiveFormStore()?.getState().selection.length ?? 0) >= n;
const hasFileDoc = () => {
  const d = getActiveDoc();
  return !!d && isFileDocument(d);
};

function formCmd(id: string, label: string, run: (s: NonNullable<ReturnType<typeof getActiveFormStore>>) => void, extra: Partial<Command> = {}): Command {
  return { id, label, isEnabled: hasForm, ...extra, run: () => run(getActiveFormStore()!) };
}

/**
 * The four ways to let a stopped program go, on the keys Visual FoxPro's debugger uses.
 * All of them mean nothing while nothing is stopped, so all of them are disabled then.
 */
function debugCommands(): Command[] {
  const stopped = () => !!useDebugStore.getState().stop;
  const go = (id: string, label: string, mode: StepMode, shortcut: string): Command => ({
    id,
    label,
    shortcut,
    isEnabled: stopped,
    run: () => useDebugStore.getState().letGo(mode),
  });
  return [
    go('program.continue', 'Continue', 'go', 'Ctrl+Enter'),
    go('program.stepInto', 'Step Into', 'into', 'F8'),
    go('program.stepOver', 'Step Over', 'over', 'F6'),
    go('program.stepOut', 'Step Out', 'out', 'Shift+F7'),
    {
      id: 'program.clearBreakpoints',
      label: 'Clear All Breakpoints',
      isEnabled: () => useDebugStore.getState().breakpoints.length > 0,
      run: () => useDebugStore.getState().clearBreakpoints(),
    },
  ];
}

/** Registers every IDE command. `ui` lets view commands toggle shell panels. */
export function registerIdeCommands(ui: { toggle(panel: keyof ShellState): void; get(panel: keyof ShellState): boolean }): void {
  const align = (kind: AlignKind, label: string): Command =>
    formCmd(`format.align.${kind}`, label, (s) => s.getState().align(kind), { isEnabled: () => hasSelection(2) });
  const size = (kind: SizeKind, label: string): Command =>
    formCmd(`format.size.${kind}`, label, (s) => s.getState().sameSize(kind), { isEnabled: () => hasSelection(2) });

  registerCommands([
    // ---- File ----
    { id: 'file.newProject', label: 'New Project...', run: () => void newProjectDialog() },
    { id: 'file.openProject', label: 'Open Project...', shortcut: 'Ctrl+Shift+O', run: () => void openProjectDialog() },
    { id: 'file.closeProject', label: 'Close Project', isEnabled: () => !!useProjectStore.getState().doc, run: () => void closeProject() },
    { id: 'file.newForm', label: 'New Form', shortcut: 'Ctrl+N', run: () => void newForm() },
    { id: 'file.newMenu', label: 'New Menu', run: () => void newMenu() },
    { id: 'file.newProgram', label: 'New Program', run: () => void newProgram() },
    { id: 'file.importVfp', label: 'Import Visual FoxPro Project...', run: () => importVfpProjectDialog() },
    { id: 'file.open', label: 'Open...', shortcut: 'Ctrl+O', run: () => void openFileDialog() },
    { id: 'file.save', label: 'Save', shortcut: 'Ctrl+S', isEnabled: hasFileDoc, run: () => void saveDocument(useDocumentsStore.getState().activeId!) },
    { id: 'file.saveAs', label: 'Save As...', isEnabled: hasFileDoc, run: () => void saveDocument(useDocumentsStore.getState().activeId!, true) },
    { id: 'file.saveAll', label: 'Save All', shortcut: 'Ctrl+Shift+S', run: () => void saveAll() },
    { id: 'file.close', label: 'Close', shortcut: 'Ctrl+W', isEnabled: () => !!getActiveDoc(), run: () => void closeDocument(useDocumentsStore.getState().activeId!) },
    { id: 'file.closeAll', label: 'Close All', isEnabled: () => useDocumentsStore.getState().order.length > 0, run: () => void closeAllDocuments() },
    { id: 'file.exit', label: 'Exit', run: () => void window.dispatchEvent(new CustomEvent('foxdev:exit')) },

    // ---- Edit ----
    formCmd('edit.undo', 'Undo', (s) => s.getState().undo(), { shortcut: 'Ctrl+Z', isEnabled: () => !!getActiveFormStore()?.getState().history.past.length }),
    formCmd('edit.redo', 'Redo', (s) => s.getState().redo(), { shortcut: 'Ctrl+Y', isEnabled: () => !!getActiveFormStore()?.getState().history.future.length }),
    formCmd('edit.cut', 'Cut', (s) => s.getState().cut(), { shortcut: 'Ctrl+X', isEnabled: () => hasSelection() }),
    formCmd('edit.copy', 'Copy', (s) => s.getState().copy(), { shortcut: 'Ctrl+C', isEnabled: () => hasSelection() }),
    formCmd('edit.paste', 'Paste', (s) => s.getState().paste(), { shortcut: 'Ctrl+V' }),
    formCmd('edit.delete', 'Delete', (s) => s.getState().removeSelected(), { shortcut: 'Delete', isEnabled: () => hasSelection() }),
    formCmd('edit.selectAll', 'Select All', (s) => s.getState().selectAll(), { shortcut: 'Ctrl+A' }),

    // ---- Format ----
    align('left', 'Align Left Sides'),
    align('right', 'Align Right Sides'),
    align('top', 'Align Top Edges'),
    align('bottom', 'Align Bottom Edges'),
    align('centerH', 'Align Vertical Centers'),
    align('centerV', 'Align Horizontal Centers'),
    size('width', 'Same Width'),
    size('height', 'Same Height'),
    size('both', 'Same Size'),
    formCmd('format.distribute.horizontal', 'Horizontal Spacing: Make Equal', (s) => s.getState().distribute('horizontal'), { isEnabled: () => hasSelection(3) }),
    formCmd('format.distribute.vertical', 'Vertical Spacing: Make Equal', (s) => s.getState().distribute('vertical'), { isEnabled: () => hasSelection(3) }),
    formCmd('format.bringToFront', 'Bring to Front', (s) => s.getState().setZOrder(s.getState().selection, 'front'), { isEnabled: () => hasSelection() }),
    formCmd('format.sendToBack', 'Send to Back', (s) => s.getState().setZOrder(s.getState().selection, 'back'), { isEnabled: () => hasSelection() }),
    { id: 'format.snapToGrid', label: 'Snap to Grid', isChecked: () => useSettingsStore.getState().snapToGrid, run: () => useSettingsStore.getState().setSnapToGrid(!useSettingsStore.getState().snapToGrid) },
    { id: 'format.showGrid', label: 'Show Grid', isChecked: () => useSettingsStore.getState().showGrid, run: () => useSettingsStore.getState().setShowGrid(!useSettingsStore.getState().showGrid) },
    ...[4, 8, 16].map<Command>((g) => ({
      id: `format.grid.${g}`,
      label: `Grid Size ${g}`,
      isChecked: () => useSettingsStore.getState().gridSize === g,
      run: () => useSettingsStore.getState().setGridSize(g),
    })),

    // ---- Program ----
    {
      id: 'program.run',
      label: 'Run Form',
      shortcut: 'Ctrl+E',
      isEnabled: hasForm,
      run: () => {
        const owner = ownerFormDoc(getActiveDoc());
        if (!owner) return;
        const name = owner.store.getState().doc.form.name;
        useDocumentsStore.getState().openDesktop();
        return useSessionStore.getState().runForm(createProjectSource(), name);
      },
    },
    {
      id: 'program.doProgram',
      label: 'Do Program',
      shortcut: 'Ctrl+D',
      isEnabled: () => getActiveDoc()?.kind === 'program',
      run: () => {
        const doc = getActiveDoc();
        if (doc?.kind !== 'program') return;
        const name = doc.path ? basename(doc.path) : 'Untitled.prg';
        useDocumentsStore.getState().openDesktop();
        return useSessionStore.getState().runProgram(createProjectSource(), name);
      },
    },
    {
      id: 'program.runMain',
      label: 'Run Main',
      shortcut: 'F5',
      isEnabled: () => !!useProjectStore.getState().doc?.main,
      run: () => {
        const main = useProjectStore.getState().doc?.main;
        if (!main) return;
        useDocumentsStore.getState().openDesktop();
        const session = useSessionStore.getState();
        // VFP's "main" may be a form or a program; a form runs and then waits for events
        return main.toLowerCase().endsWith('.prg') ? session.runProgram(createProjectSource(), main) : session.runForm(createProjectSource(), main);
      },
    },
    {
      id: 'program.cancel',
      label: 'Cancel Program',
      shortcut: 'Shift+F5',
      isEnabled: () => useSessionStore.getState().status !== 'idle',
      run: () => useSessionStore.getState().cancel(),
    },
    // ---- the debugger: what a stopped program is driven with ----
    ...debugCommands(),
    {
      id: 'program.build',
      label: 'Build App...',
      shortcut: 'Ctrl+Shift+B',
      isEnabled: () => !!useProjectStore.getState().doc,
      run: () => void buildApp(),
    },
    {
      id: 'program.buildExe',
      label: 'Build Executable...',
      isEnabled: () => !!useProjectStore.getState().doc,
      run: () => void buildExecutable(),
    },
    {
      id: 'program.traceEvents',
      label: 'Trace Events',
      isChecked: () => useSessionStore.getState().traceEvents,
      run: () => useSessionStore.getState().setTraceEvents(!useSessionStore.getState().traceEvents),
    },

    // ---- View ----
    { id: 'view.explorer', label: 'Project Explorer', isChecked: () => ui.get('showExplorer'), run: () => ui.toggle('showExplorer') },
    { id: 'view.properties', label: 'Properties Window', shortcut: 'F4', isChecked: () => ui.get('showProperties'), run: () => ui.toggle('showProperties') },
    { id: 'view.output', label: 'Output Window', shortcut: 'Ctrl+F2', isChecked: () => ui.get('showOutput'), run: () => ui.toggle('showOutput') },
    { id: 'view.debugger', label: 'Debugger', shortcut: 'Ctrl+Shift+D', isChecked: () => ui.get('showDebugger'), run: () => ui.toggle('showDebugger') },
    { id: 'view.toggleTheme', label: 'Toggle Dark Theme', isChecked: () => useSettingsStore.getState().theme === 'dark', run: () => useSettingsStore.getState().toggleTheme() },

    // ---- Help ----
    {
      id: 'help.about',
      label: 'About FoxDev Studio',
      run: async () => {
        const v = await getApi().app.getVersion();
        await getApi().dialog.message({ type: 'info', message: `FoxDev Studio ${v}`, detail: 'A modern Visual FoxPro style IDE.', buttons: ['OK'] });
      },
    },
  ]);
}
