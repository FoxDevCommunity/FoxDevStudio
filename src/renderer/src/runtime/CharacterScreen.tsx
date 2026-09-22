/**
 * The character screen `@ ... SAY` draws on, and the windows a program opened over it.
 *
 * Visual FoxPro still lets a program address the screen by row and column, and a great many
 * working programs do nothing else. The VM keeps that surface as a grid of characters and hands
 * it over whole; this draws it in a fixed-width font, sizing the cell so the whole width fits.
 */

import { useEffect, useRef, useState } from 'react';
import type { ScreenDoc, WindowDoc } from '@shared/runtime/host';
import { useSessionStore } from './session';

/** How wide a character is as a fraction of its height, for the font stack below. */
const ASPECT = 0.6;
const FONT = 'Consolas, "Cascadia Mono", "Courier New", monospace';

export function CharacterScreen({ screen }: { screen: ScreenDoc }) {
  const box = useRef<HTMLDivElement>(null);
  const [cell, setCell] = useState(14);

  // the whole width of the screen has to fit, however wide the pane is
  useEffect(() => {
    const el = box.current;
    if (!el) return;
    const measure = (): void => {
      const width = el.clientWidth;
      if (width > 0) setCell(Math.max(6, Math.min(24, width / screen.cols / ASPECT)));
    };
    measure();
    const observer = new ResizeObserver(measure);
    observer.observe(el);
    return () => observer.disconnect();
  }, [screen.cols]);

  const pointer = (event: React.MouseEvent<HTMLDivElement>, down?: boolean): void => {
    const rect = event.currentTarget.getBoundingClientRect();
    useSessionStore.getState().setPointer({
      row: Math.floor((event.clientY - rect.top) / cell),
      col: Math.floor((event.clientX - rect.left) / (cell * ASPECT)),
      down: down ?? event.buttons !== 0,
    });
  };

  return (
    <div ref={box} style={{ width: '100%', padding: 8, boxSizing: 'border-box' }}>
      <div
        data-testid="character-screen"
        onMouseMove={pointer}
        onMouseDown={(e) => pointer(e, true)}
        onMouseUp={(e) => pointer(e, false)}
        style={{
          position: 'relative',
          width: screen.cols * cell * ASPECT,
          height: screen.rows * cell,
          fontFamily: FONT,
          fontSize: cell * 0.85,
          lineHeight: `${cell}px`,
          whiteSpace: 'pre',
          background: 'var(--colorNeutralBackground1)',
          color: 'var(--colorNeutralForeground1)',
          border: '1px solid var(--colorNeutralStroke2)',
          overflow: 'hidden',
        }}
      >
        {screen.lines.map((line, row) => (
          // the rows never move, so their place in the grid is the only identity they have
          <div key={row} style={{ height: cell }}>
            {line}
          </div>
        ))}
        {screen.windows.map((window) => (
          <CharacterWindow key={window.name} window={window} cell={cell} />
        ))}
      </div>
    </div>
  );
}

function CharacterWindow({ window, cell }: { window: WindowDoc; cell: number }): React.JSX.Element {
  const height = window.minimized ? 1 : window.height;
  return (
    <div
      data-testid={`screen-window-${window.name}`}
      style={{
        position: 'absolute',
        top: window.row * cell,
        left: window.col * cell * ASPECT,
        width: window.width * cell * ASPECT,
        height: (height + (window.title ? 1 : 0)) * cell,
        background: 'var(--colorNeutralBackground1)',
        border: window.border ? '1px solid var(--colorNeutralStroke1)' : 'none',
        boxShadow: 'var(--shadow8)',
        boxSizing: 'border-box',
        overflow: 'hidden',
      }}
    >
      {window.title && (
        <div
          style={{
            height: cell,
            background: 'var(--colorNeutralBackground3)',
            textAlign: 'center',
            fontWeight: 600,
          }}
        >
          {window.title}
        </div>
      )}
      {!window.minimized &&
        window.lines.map((line, row) => (
          <div key={row} style={{ height: cell }}>
            {line}
          </div>
        ))}
    </div>
  );
}
