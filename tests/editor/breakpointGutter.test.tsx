/**
 * The breakpoint gutter, clicked.
 *
 * The debugger's own tests drive the store directly, so nothing covered the one thing a
 * developer actually does - click beside a line - and the gutter shipped with no element to
 * click on at all. Which line a click lands on depends on the editor's layout, which jsdom does
 * not do, so that part is only exercised in a real browser; what is asserted here is that there
 * is something to click on every line, and that clicking it toggles the breakpoint.
 */

import { render, cleanup } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { CodeEditor } from '@renderer/editor/CodeEditor';
import { breakpointGutter } from '@renderer/editor/breakpointGutter';
import { useDebugStore } from '@renderer/runtime/debugSession';

const SOURCE = '? 1\n? 2\n? 3\n';

function editor() {
  return render(<CodeEditor value={SOURCE} onChange={() => {}} extensions={[breakpointGutter(() => 'main')]} />);
}

function cells(container: HTMLElement): HTMLElement[] {
  return [...container.querySelectorAll('.fx-breakpoint-gutter .cm-gutterElement')] as HTMLElement[];
}

describe('the breakpoint gutter', () => {
  beforeEach(() => {
    useDebugStore.setState({ breakpoints: [] });
  });
  afterEach(cleanup);

  it('gives every line something to click on, not only the lines that already have one', () => {
    const { container } = editor();
    expect(container.querySelector('.fx-breakpoint-gutter')).not.toBeNull();
    // one element per line of the document, and the spacer that holds the column open
    expect(cells(container).length).toBeGreaterThanOrEqual(container.querySelectorAll('.cm-line').length);
  });

  it('sets a breakpoint where it was clicked, and a second click takes it away', () => {
    const { container } = editor();
    const cell = cells(container).at(-1)!;

    cell.dispatchEvent(new MouseEvent('mousedown', { bubbles: true }));
    expect(useDebugStore.getState().breakpoints).toHaveLength(1);
    expect(useDebugStore.getState().breakpoints[0]?.program.toUpperCase()).toBe('MAIN');

    cell.dispatchEvent(new MouseEvent('mousedown', { bubbles: true }));
    expect(useDebugStore.getState().breakpoints).toEqual([]);
  });

  it('shows a dot on a line the store already has a breakpoint on', async () => {
    useDebugStore.getState().toggleBreakpoint('main', 2);
    const { container } = editor();
    // the store is pushed into the editor once the stack unwinds
    await new Promise((resolve) => setTimeout(resolve, 0));
    expect(container.querySelector('.fx-breakpoint')).not.toBeNull();
  });

  it('finds the breakpoints of its own source, whatever case the name is written in', () => {
    useDebugStore.getState().toggleBreakpoint('MAIN', 2);
    expect(useDebugStore.getState().linesIn('main')).toEqual([2]);
    expect(useDebugStore.getState().linesIn('other')).toEqual([]);
  });
});

describe('the dot, without reopening the file', () => {
  beforeEach(() => {
    useDebugStore.setState({ breakpoints: [] });
  });
  afterEach(cleanup);

  it('appears in the editor that was clicked, there and then', async () => {
    const { container } = editor();
    await new Promise((resolve) => setTimeout(resolve, 0));
    expect(container.querySelector('.fx-breakpoint')).toBeNull();

    cells(container).at(-1)!.dispatchEvent(new MouseEvent('mousedown', { bubbles: true }));
    await new Promise((resolve) => setTimeout(resolve, 0));

    // this is what closing and reopening the file used to be needed for
    expect(container.querySelector('.fx-breakpoint')).not.toBeNull();
  });

  it('goes away again on the second click, in the same editor', async () => {
    const { container } = editor();
    const cell = cells(container).at(-1)!;
    cell.dispatchEvent(new MouseEvent('mousedown', { bubbles: true }));
    await new Promise((resolve) => setTimeout(resolve, 0));
    expect(container.querySelector('.fx-breakpoint')).not.toBeNull();

    cell.dispatchEvent(new MouseEvent('mousedown', { bubbles: true }));
    await new Promise((resolve) => setTimeout(resolve, 0));
    expect(container.querySelector('.fx-breakpoint')).toBeNull();
  });
});
