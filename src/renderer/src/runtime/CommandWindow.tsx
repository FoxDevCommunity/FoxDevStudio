/**
 * The VFP Command Window: type a line, press Enter, it runs. Works with nothing running (it
 * starts an idle session) and while a program is parked in READ EVENTS, which is how VFP lets
 * you poke at a live application.
 */

import { useRef, useState } from 'react';
import { Input, Text } from '@fluentui/react-components';
import { createProjectSource } from './projectSource';
import { useSessionStore } from './session';

export function CommandWindow() {
  const [text, setText] = useState('');
  const [historyAt, setHistoryAt] = useState<number | null>(null);
  const sourceRef = useRef(createProjectSource());

  const submit = () => {
    const line = text.trim();
    if (!line) return;
    setText('');
    setHistoryAt(null);
    void useSessionStore.getState().execute(sourceRef.current, line);
  };

  /** Up/down walk the history, newest first, like the VFP command window. */
  const recall = (delta: number) => {
    const history = useSessionStore.getState().history;
    if (history.length === 0) return;
    const next = historyAt === null ? (delta < 0 ? history.length - 1 : null) : historyAt + (delta < 0 ? -1 : 1);
    if (next === null || next >= history.length) {
      setHistoryAt(null);
      setText('');
      return;
    }
    const index = Math.max(0, next);
    setHistoryAt(index);
    setText(history[index] ?? '');
  };

  return (
    <div className="fx-command-window" style={{ display: 'flex', alignItems: 'center', gap: 8, padding: '4px 8px', borderTop: '1px solid var(--colorNeutralStroke2)' }}>
      <Text size={200} style={{ color: 'var(--colorNeutralForeground3)' }}>
        Command
      </Text>
      <Input
        aria-label="Command Window"
        size="small"
        appearance="filled-darker"
        style={{ flex: 1, fontFamily: 'var(--fontFamilyMonospace)' }}
        value={text}
        placeholder="? DATE()"
        onChange={(_e, d) => setText(d.value)}
        onKeyDown={(e) => {
          if (e.key === 'Enter') {
            e.preventDefault();
            submit();
          } else if (e.key === 'ArrowUp') {
            e.preventDefault();
            recall(-1);
          } else if (e.key === 'ArrowDown') {
            e.preventDefault();
            recall(1);
          }
        }}
      />
    </div>
  );
}
