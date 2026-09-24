import { useEffect, useState } from 'react';
import { FormDesignerProvider } from '../designer/store/FormDesignerContext';
import { PropertiesWindow } from '../designer/properties/PropertiesWindow';
import { SymbolExplorer } from '../designer/properties/SymbolExplorer';
import { ProjectExplorer } from '../explorer/ProjectExplorer';
import { useProjectStore } from '../stores/projectStore';
import { useActiveFormDoc } from './activeDocument';
import { registerIdeCommands, type ShellState } from './commands';
import { DocumentArea } from './DocumentArea';
import { MenuBar } from './MenuBar';
import { SplitPane } from './SplitPane';
import { StatusBar } from './StatusBar';
import { useShortcuts } from './useShortcuts';
import { WelcomePage } from './WelcomePage';
import { OutputPanel } from '../runtime/OutputPanel';
import { DebuggerPanel } from '../runtime/DebuggerPanel';
import { runtimeUi, useSessionStore } from '../runtime/session';
import { createProjectSource } from '../runtime/projectSource';
import { basename, extname } from '@shared/paths';
import { ProgramErrorDialog, ReadDialog, RuntimeDialog, WaitWindowToast } from '../runtime/dialogs/RuntimeDialogs';
import { openMethodAt } from '../runtime/navigateToError';
import { useDocumentsStore } from '../stores/documentsStore';
import { newDocument, openFile } from '../stores/fileActions';

let shellState: ShellState = { showExplorer: true, showProperties: true, showOutput: true, showDebugger: false };
const listeners = new Set<() => void>();
function setShell(patch: Partial<ShellState>) {
  shellState = { ...shellState, ...patch };
  listeners.forEach((l) => l());
}
registerIdeCommands({ toggle: (p) => setShell({ [p]: !shellState[p] }), get: (p) => shellState[p] });
// a form opened by any means (Run, a menu item, the Command Window) surfaces the desktop tab
runtimeUi.showDesktop = () => void useDocumentsStore.getState().openDesktop();
runtimeUi.development = true;
// a program that stops has to be seen stopping, however the debugger window was left
runtimeUi.showDebugger = () => setShell({ showDebugger: true });
// MODIFY COMMAND and BROWSE open a file in a tab: the same thing File > Open does
runtimeUi.openDocument = async (path) => void (await openFile(path));
// CREATE FORM and the rest open a designer on something new, which File > New also does
runtimeUi.newDocument = newDocument;
// a program that stops brings its own source to the front, so the marked line is one that
// can be seen - stepping into a routine whose tab is not open is exactly when it matters
runtimeUi.showSource = (program) => void openMethodAt(program, 0);
// a file of the project run by name: a form is shown, a program is run
runtimeUi.runFile = async (path) => {
  const session = useSessionStore.getState();
  useDocumentsStore.getState().openDesktop();
  const name = basename(path);
  await (extname(path).toLowerCase() === '.fxf'
    ? session.runForm(createProjectSource(), name)
    : session.runProgram(createProjectSource(), name));
};

/** Menu bar, explorer | documents | properties, status bar. */
export function IdeLayout() {
  const [, force] = useState(0);
  useEffect(() => {
    const l = () => force((n) => n + 1);
    listeners.add(l);
    return () => void listeners.delete(l);
  }, []);
  useShortcuts();
  const hasProject = useProjectStore((s) => !!s.doc);
  const form = useActiveFormDoc();

  // the properties window edits one object; the symbol explorer under it lists them all
  const properties = form ? (
    <FormDesignerProvider store={form.store} docId={form.id}>
      <div style={{ display: 'flex', flexDirection: 'column', height: '100%', minHeight: 0 }}>
        <div style={{ flex: 1, minHeight: 0, display: 'flex' }}>
          <PropertiesWindow />
        </div>
        <SymbolExplorer />
      </div>
    </FormDesignerProvider>
  ) : (
    <PropertiesWindow />
  );

  // the Output window and the Debugger share the docked strip under the documents: a program
  // that stops needs both at once - what it printed, and where it stopped
  const output = hasProject && shellState.showOutput ? <OutputPanel /> : undefined;
  const debugger_ = hasProject && shellState.showDebugger ? <DebuggerPanel /> : undefined;
  const bottom =
    output && debugger_ ? (
      <div style={{ display: 'flex', height: '100%', minHeight: 0 }}>
        <div style={{ flex: 1, minWidth: 0, display: 'flex', flexDirection: 'column' }}>{output}</div>
        <div style={{ flex: 2, minWidth: 0, display: 'flex', flexDirection: 'column', borderLeft: '1px solid var(--colorNeutralStroke2)' }}>
          {debugger_}
        </div>
      </div>
    ) : (
      (output ?? debugger_)
    );

  return (
    <div style={{ display: 'flex', flexDirection: 'column', height: '100%' }} data-testid="ide">
      <MenuBar />
      <SplitPane
        left={shellState.showExplorer ? <ProjectExplorer /> : undefined}
        center={hasProject ? <DocumentArea /> : <WelcomePage />}
        right={shellState.showProperties ? properties : undefined}
        bottom={bottom}
      />
      <StatusBar />
      {/* app-modal: a MESSAGEBOX or an error must show whichever tab is in front */}
      <WaitWindowToast />
      <RuntimeDialog />
      <ReadDialog />
      <ProgramErrorDialog onEdit={openMethodAt} />
    </div>
  );
}
