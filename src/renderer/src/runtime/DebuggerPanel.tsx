/**
 * The debugger window: where the program stopped, the call stack, what the chosen frame can
 * see, and the watch expressions.
 *
 * Everything here is read from a parked fiber while the VM is off the stack, so the panel is a
 * plain synchronous view of the store - and the rest of the IDE stays live while a program
 * waits at a breakpoint, which is the whole point of stopping on a fiber rather than a thread.
 */

import { useState } from 'react';
import { Button, Input, Text } from '@fluentui/react-components';
import { displayValue } from '@shared/runtime/values';
import { reasonLabel, useDebugStore } from './debugSession';
import { useSessionStore } from './session';

const SECTION: React.CSSProperties = {
  flex: 1,
  minWidth: 0,
  minHeight: 0,
  display: 'flex',
  flexDirection: 'column',
  borderLeft: '1px solid var(--colorNeutralStroke2)',
};

const LIST: React.CSSProperties = {
  flex: 1,
  minHeight: 0,
  overflow: 'auto',
  margin: 0,
  padding: '2px 6px',
  listStyle: 'none',
  fontFamily: 'var(--fontFamilyMonospace)',
  fontSize: 12,
};

function Heading({ children }: { children: React.ReactNode }) {
  return (
    <Text size={200} weight="semibold" style={{ padding: '2px 6px', borderBottom: '1px solid var(--colorNeutralStroke2)' }}>
      {children}
    </Text>
  );
}

export function DebuggerPanel() {
  const stop = useDebugStore((s) => s.stop);
  const frames = useDebugStore((s) => s.frames);
  const level = useDebugStore((s) => s.level);
  const locals = useDebugStore((s) => s.locals);
  const watches = useDebugStore((s) => s.watches);
  const breakpoints = useDebugStore((s) => s.breakpoints);
  const [expression, setExpression] = useState('');
  const running = useSessionStore((s) => s.status) !== 'idle';

  const letGo = useDebugStore.getState().letGo;
  const button = (label: string, run: () => void, disabled = !stop) => (
    <Button size="small" appearance="subtle" disabled={disabled} onClick={run}>
      {label}
    </Button>
  );

  return (
    <div style={{ display: 'flex', flexDirection: 'column', height: '100%', minHeight: 0 }} data-testid="debugger-panel">
      <div style={{ display: 'flex', alignItems: 'center', gap: 4, padding: '2px 8px', borderBottom: '1px solid var(--colorNeutralStroke2)' }}>
        <Text size={200} weight="semibold">
          Debugger
        </Text>
        {button('Continue', () => letGo('go'))}
        {button('Step Into', () => letGo('into'))}
        {button('Step Over', () => letGo('over'))}
        {button('Step Out', () => letGo('out'))}
        {button('Stop', () => useSessionStore.getState().cancel(), !running)}
        <Text size={200} style={{ flex: 1, textAlign: 'right', color: 'var(--colorNeutralForeground3)' }}>
          {stop ? `${reasonLabel(stop)} in ${stop.program}, line ${stop.line}` : 'Running - nothing is stopped'}
        </Text>
      </div>

      <div style={{ flex: 1, minHeight: 0, display: 'flex' }}>
        <div style={{ ...SECTION, borderLeft: 'none' }}>
          <Heading>Call Stack</Heading>
          <ul aria-label="Call stack" style={LIST}>
            {frames.map((frame, i) => (
              <li key={i}>
                <button
                  type="button"
                  aria-current={i === level}
                  onClick={() => useDebugStore.getState().selectFrame(i)}
                  style={{
                    all: 'unset',
                    cursor: 'pointer',
                    display: 'block',
                    width: '100%',
                    fontWeight: i === level ? 600 : 400,
                  }}
                >
                  {frame.program} ({frame.line})
                </button>
              </li>
            ))}
          </ul>
        </div>

        <div style={SECTION}>
          <Heading>Locals</Heading>
          <ul aria-label="Locals" style={LIST}>
            {locals.map((v) => (
              <li key={v.name}>
                {v.name}
                {v.private ? ' (private)' : ''} = {displayValue(v.value)}
              </li>
            ))}
          </ul>
        </div>

        <div style={SECTION}>
          <Heading>Watch</Heading>
          <ul aria-label="Watch expressions" style={LIST}>
            {watches.map((w, i) => (
              <li key={`${w.text}-${i}`} style={{ color: w.failed ? 'var(--colorPaletteRedForeground1)' : undefined }}>
                {w.text} = {w.value}
                <Button size="small" appearance="transparent" aria-label={`Remove watch ${w.text}`} onClick={() => useDebugStore.getState().removeWatch(i)}>
                  ×
                </Button>
              </li>
            ))}
          </ul>
          <form
            style={{ display: 'flex', gap: 4, padding: 4, borderTop: '1px solid var(--colorNeutralStroke2)' }}
            onSubmit={(e) => {
              e.preventDefault();
              useDebugStore.getState().addWatch(expression);
              setExpression('');
            }}
          >
            <Input
              size="small"
              style={{ flex: 1 }}
              aria-label="Watch expression"
              placeholder="Expression to watch"
              value={expression}
              onChange={(_e, d) => setExpression(d.value)}
            />
            <Button size="small" type="submit">
              Watch
            </Button>
          </form>
        </div>

        <div style={SECTION}>
          <Heading>Breakpoints</Heading>
          <ul aria-label="Breakpoints" style={LIST}>
            {breakpoints.map((b) => (
              <li key={`${b.program}:${b.line}`}>
                <button
                  type="button"
                  aria-label={`Remove breakpoint at ${b.program} line ${b.line}`}
                  onClick={() => useDebugStore.getState().toggleBreakpoint(b.program, b.line)}
                  style={{ all: 'unset', cursor: 'pointer', display: 'block', width: '100%' }}
                >
                  {b.program} ({b.line})
                </button>
              </li>
            ))}
          </ul>
        </div>
      </div>
    </div>
  );
}
