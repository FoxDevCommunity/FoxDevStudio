/**
 * The runtime player: a whole window given over to one running application. It loads the
 * `.fxa` bundle the window was launched with, starts its main item, and otherwise stays out
 * of the way - no editor, no project, no Command Window.
 */

import { useEffect, useRef, useState, type ReactNode } from 'react';
import { Button, FluentProvider, Text, webLightTheme } from '@fluentui/react-components';
import { createBundleSource, parseBundle } from '@shared/runtime/bundle';
import { getApi } from '../api/foxdev';
import { useSessionStore } from '../runtime/session';
import { RuntimeDesktopDocument } from '../runtime/RuntimeDesktop';
import { ProgramErrorDialog, ReadDialog, RuntimeDialog, WaitWindowToast } from '../runtime/dialogs/RuntimeDialogs';
import { loadFoxVm } from '../../../wasm/foxvm/loader';

type Phase = 'loading' | 'running' | 'error';

export function PlayerApp() {
  // the player has no theme setting: a built application always looks the same
  const [phase, setPhase] = useState<Phase>('loading');
  /** The parse failure to show; null in the `error` phase means "there was no bundle at all". */
  const [problem, setProblem] = useState<string | null>(null);
  const started = useRef(false);

  useEffect(() => {
    if (started.current) return;
    started.current = true;
    void (async () => {
      await loadFoxVm();
      const loaded = await getApi().player.getBundle();
      if (!loaded) {
        setProblem(null);
        setPhase('error');
        return;
      }
      const parsed = parseBundle(loaded.text);
      if (!parsed.ok) {
        setProblem(parsed.error);
        setPhase('error');
        return;
      }
      const bundle = parsed.doc;
      document.title = bundle.name;
      setPhase('running');
      const source = createBundleSource(bundle);
      const session = useSessionStore.getState();
      // never awaited: a program that reaches READ EVENTS only settles at CLEAR EVENTS
      void (bundle.main.kind === 'program' ? session.runProgram(source, bundle.main.name) : session.runForm(source, bundle.main.name));
    })();
  }, []);

  return (
    <FluentProvider theme={webLightTheme} style={{ height: '100%' }}>
      <div style={{ position: 'relative', height: '100%', display: 'flex', flexDirection: 'column', overflow: 'hidden' }}>
        {phase === 'error' ? (
          <Panel title={problem ?? 'No application bundle was loaded.'}>
            <Text size={200}>{problem ? 'The file is not a FoxDev application bundle.' : 'Run one with FoxDevRuntime.exe --play app.fxa'}</Text>
          </Panel>
        ) : (
          <>
            <RuntimeDesktopDocument />
            <EndedPanel active={phase === 'running'} />
          </>
        )}
        <WaitWindowToast />
        <RuntimeDialog />
      <ReadDialog />
        {/* no onEdit: the player has no editor to jump to */}
        <ProgramErrorDialog />
      </div>
    </FluentProvider>
  );
}

/**
 * VFP's runtime exits once the last form closes and no READ EVENTS is outstanding. We show
 * that rather than closing the window on our own, so the user (and a test) can see it happen.
 */
function EndedPanel({ active }: { active: boolean }) {
  const status = useSessionStore((s) => s.status);
  const desktop = useSessionStore((s) => s.desktop);
  // form open/close mutates the desktop in place, so re-render on the store's revision counter
  useSessionStore((s) => s.revision);
  const [ran, setRan] = useState(false);

  // "ended" only means anything once a session has actually started
  useEffect(() => useSessionStore.subscribe((s) => setRan((was) => was || s.desktop !== null)), []);

  if (!active || !ran || status !== 'idle' || (desktop?.forms.length ?? 0) > 0) return null;
  return (
    <Panel title="The application has ended.">
      <Button appearance="primary" onClick={() => window.close()}>
        Quit
      </Button>
    </Panel>
  );
}

function Panel({ title, children }: { title: string; children: ReactNode }) {
  return (
    <div style={{ position: 'absolute', inset: 0, display: 'flex', alignItems: 'center', justifyContent: 'center', zIndex: 50 }}>
      <div
        style={{
          display: 'flex',
          flexDirection: 'column',
          alignItems: 'center',
          gap: 12,
          padding: 24,
          background: 'var(--colorNeutralBackground1)',
          border: '1px solid var(--colorNeutralStroke1)',
          borderRadius: 6,
          boxShadow: 'var(--shadow16)',
        }}
      >
        <Text weight="semibold">{title}</Text>
        {children}
      </div>
    </div>
  );
}
