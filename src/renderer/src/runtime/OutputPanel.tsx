/**
 * The IDE-wide Output window: `?` output, runtime errors, echoed Command Window lines and,
 * when enabled, an event trace. Unlike the Milestone-1 preview log it outlives a single form.
 */

import { useEffect, useRef } from 'react';
import { Button, Checkbox, Text } from '@fluentui/react-components';
import { formatOutput, useSessionStore, type OutputLine } from './session';
import { CommandWindow } from './CommandWindow';

const COLORS: Record<OutputLine['kind'], string> = {
  output: 'var(--colorNeutralForeground1)',
  error: 'var(--colorPaletteRedForeground1)',
  trace: 'var(--colorNeutralForeground3)',
  echo: 'var(--colorBrandForeground1)',
};

export function OutputPanel() {
  const output = useSessionStore((s) => s.output);
  const trace = useSessionStore((s) => s.traceEvents);
  const listRef = useRef<HTMLUListElement>(null);

  useEffect(() => {
    const el = listRef.current;
    if (el) el.scrollTop = el.scrollHeight;
  }, [output.length]);

  return (
    <div style={{ display: 'flex', flexDirection: 'column', height: '100%', minHeight: 0 }} data-testid="output-panel">
      <div style={{ display: 'flex', alignItems: 'center', gap: 12, padding: '2px 8px', borderBottom: '1px solid var(--colorNeutralStroke2)' }}>
        <Text size={200} weight="semibold" style={{ flex: 1 }}>
          Output
        </Text>
        <Checkbox
          label="Trace events"
          checked={trace}
          onChange={(_e, d) => useSessionStore.getState().setTraceEvents(d.checked === true)}
        />
        <Button size="small" appearance="subtle" onClick={() => useSessionStore.getState().clearOutput()}>
          Clear
        </Button>
      </div>
      <ul
        ref={listRef}
        aria-label="Output"
        style={{ flex: 1, minHeight: 0, overflow: 'auto', margin: 0, padding: '4px 8px', listStyle: 'none', fontFamily: 'var(--fontFamilyMonospace)', fontSize: 12 }}
      >
        {output.map((line, i) => (
          <li key={i} style={{ color: COLORS[line.kind], whiteSpace: 'pre-wrap' }}>
            {formatOutput(line)}
          </li>
        ))}
      </ul>
      <CommandWindow />
    </div>
  );
}
