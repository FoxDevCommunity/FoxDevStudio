import { useState, type ReactNode } from 'react';
import { startPointerDrag } from '../designer/canvas/usePointerDrag';

export interface SplitPaneProps {
  left?: ReactNode;
  center: ReactNode;
  right?: ReactNode;
  /** Docked under the centre column (the Output window). */
  bottom?: ReactNode;
  leftWidth?: number;
  rightWidth?: number;
  bottomHeight?: number;
  minSide?: number;
}

/** Three-column layout with draggable splitters. Small and jsdom-safe. */
export function SplitPane({ left, center, right, bottom, leftWidth = 240, rightWidth = 300, bottomHeight = 180, minSide = 120 }: SplitPaneProps) {
  const [lw, setLw] = useState(leftWidth);
  const [rw, setRw] = useState(rightWidth);
  const [bh, setBh] = useState(bottomHeight);

  const splitter = (side: 'left' | 'right') => (
    <div
      role="separator"
      aria-orientation="vertical"
      aria-label={side === 'left' ? 'Resize explorer' : 'Resize properties'}
      style={{ width: 5, cursor: 'col-resize', background: 'var(--colorNeutralStroke2)', flex: 'none' }}
      onPointerDown={(e) => {
        const start = side === 'left' ? lw : rw;
        startPointerDrag(e, {
          onMove: (dx) => (side === 'left' ? setLw(Math.max(minSide, start + dx)) : setRw(Math.max(minSide, start - dx))),
        });
      }}
    />
  );

  return (
    <div style={{ display: 'flex', flex: 1, minHeight: 0, minWidth: 0 }}>
      {left && (
        <>
          <div style={{ width: lw, flex: 'none', minWidth: 0, overflow: 'hidden', display: 'flex', flexDirection: 'column' }}>{left}</div>
          {splitter('left')}
        </>
      )}
      <div style={{ flex: 1, minWidth: 0, minHeight: 0, display: 'flex', flexDirection: 'column' }}>
        <div style={{ flex: 1, minHeight: 0, display: 'flex', flexDirection: 'column' }}>{center}</div>
        {bottom && (
          <>
            <div
              role="separator"
              aria-orientation="horizontal"
              aria-label="Resize output"
              style={{ height: 5, cursor: 'row-resize', background: 'var(--colorNeutralStroke2)', flex: 'none' }}
              onPointerDown={(e) => {
                const start = bh;
                startPointerDrag(e, { onMove: (_dx, dy) => setBh(Math.max(80, start - dy)) });
              }}
            />
            <div style={{ height: bh, flex: 'none', minHeight: 0, display: 'flex', flexDirection: 'column' }}>{bottom}</div>
          </>
        )}
      </div>
      {right && (
        <>
          {splitter('right')}
          <div style={{ width: rw, flex: 'none', minWidth: 0, overflow: 'hidden', display: 'flex', flexDirection: 'column' }}>{right}</div>
        </>
      )}
    </div>
  );
}
