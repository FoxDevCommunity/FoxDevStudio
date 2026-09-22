/**
 * The line the debugger is stopped on, shown in the editor of the source it stopped in.
 *
 * Stepping is the whole point: the mark has to move as the program moves, in the editor that is
 * already open, without it being rebuilt.
 */

import { render, cleanup } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { CodeEditor } from '@renderer/editor/CodeEditor';
import { currentLine } from '@renderer/editor/currentLine';
import { useDebugStore } from '@renderer/runtime/debugSession';
import type { BreakStop, DebugFrame } from '@shared/runtime/host';

const SOURCE = '? 1\n? 2\n? 3\n? 4\n';

function editor(program = 'main') {
  return render(<CodeEditor value={SOURCE} onChange={() => {}} extensions={[currentLine(() => program)]} />);
}

function stoppedAt(line: number, program = 'main'): void {
  const stop: BreakStop = { fiber: 1, program, line, reason: 'breakpoint' };
  const frames: DebugFrame[] = [{ program, module: program, line }];
  useDebugStore.setState({ stop, frames, level: 0 });
}

const settle = () => new Promise((resolve) => setTimeout(resolve, 0));

describe('the stopped line', () => {
  beforeEach(() => {
    useDebugStore.setState({ stop: null, frames: [], level: 0 });
  });
  afterEach(cleanup);

  it('is not marked while nothing is stopped', async () => {
    const { container } = editor();
    await settle();
    expect(container.querySelector('.fx-stopped-line')).toBeNull();
    expect(container.querySelector('.fx-stopped-arrow')).toBeNull();
  });

  it('marks the line, and moves as the program is stepped', async () => {
    const { container } = editor();
    stoppedAt(2);
    await settle();
    const marked = () => container.querySelector('.cm-line.fx-stopped-line')?.textContent;
    expect(marked()).toBe('? 2');

    // stepping is the same thing again with another line, in the editor already open
    stoppedAt(3);
    await settle();
    expect(marked()).toBe('? 3');
  });

  it('marks nothing when the program stopped somewhere else', async () => {
    const { container } = editor('main');
    stoppedAt(2, 'other');
    await settle();
    expect(container.querySelector('.fx-stopped-line')).toBeNull();
  });

  it('follows the frame chosen in the call stack, not only the innermost', async () => {
    const { container } = editor('caller');
    useDebugStore.setState({
      stop: { fiber: 1, program: 'inner', line: 9, reason: 'step' },
      frames: [
        { program: 'caller', module: 'caller', line: 4 },
        { program: 'inner', module: 'inner', line: 9 },
      ],
      level: 0,
    });
    await settle();
    expect(container.querySelector('.cm-line.fx-stopped-line')?.textContent).toBe('? 4');
  });

  it('is taken off when the program is let go', async () => {
    const { container } = editor();
    stoppedAt(2);
    await settle();
    expect(container.querySelector('.fx-stopped-line')).not.toBeNull();

    useDebugStore.setState({ stop: null, frames: [], level: 0 });
    await settle();
    expect(container.querySelector('.fx-stopped-line')).toBeNull();
  });
});
