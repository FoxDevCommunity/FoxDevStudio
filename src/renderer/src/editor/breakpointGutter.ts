/**
 * The editor's breakpoint gutter: a click beside a line asks the debugger to stop there.
 *
 * A breakpoint is named by the source it belongs to, exactly as the call stack names it - the
 * program's own name for a .prg, `objPath.Event` for a method - so the gutter has to be told
 * which source it is showing and nothing else. It reads the breakpoints back from the store,
 * which is where they live between runs.
 */

import { StateEffect, StateField, type Extension } from '@codemirror/state';
import { EditorView, GutterMarker, ViewPlugin, gutter } from '@codemirror/view';
import { useDebugStore } from '../runtime/debugSession';

const setLines = StateEffect.define<number[]>();

/** Which lines of this document carry a breakpoint, mirrored from the store. */
const breakLines = StateField.define<Set<number>>({
  create: () => new Set(),
  update(lines, tr) {
    for (const effect of tr.effects) {
      if (effect.is(setLines)) return new Set(effect.value);
    }
    return lines;
  },
});

class BreakpointDot extends GutterMarker {
  constructor(private readonly className: string) {
    super();
  }

  override toDOM(): Node {
    const dot = document.createElement('span');
    dot.className = this.className;
    dot.textContent = '●';
    return dot;
  }
}

const dot = new BreakpointDot('fx-breakpoint');
/** The same shape, drawn on nothing: CodeMirror keeps this one to hold the column open. */
const spacer = new BreakpointDot('fx-breakpoint-spacer');

/**
 * `program` is read when it is needed rather than captured, because a method editor can be
 * pointed at another object and event without being rebuilt.
 */
export function breakpointGutter(program: () => string): Extension {
  return [
    breakLines,
    ViewPlugin.define((view) => {
      // dispatching from inside a view update or a store write is not allowed, so every push
      // of the store's lines into the editor waits for the stack to unwind first
      const push = () =>
        queueMicrotask(() => {
          if (view.dom.isConnected) view.dispatch({ effects: setLines.of(useDebugStore.getState().linesIn(program())) });
        });
      push();
      const unsubscribe = useDebugStore.subscribe(push);
      return { update: () => {}, destroy: unsubscribe };
    }),
    gutter({
      class: 'fx-breakpoint-gutter',
      lineMarker: (view, line) => (view.state.field(breakLines).has(view.state.doc.lineAt(line.from).number) ? dot : null),
      // without this there is nothing to click on: CodeMirror builds a gutter element only for
      // a line that already has a marker, so a file with no breakpoints in it had none at all
      renderEmptyElements: true,
      // and without this the markers are only recomputed when the document or the viewport
      // changes. Setting a breakpoint is neither, so the dot did not appear until the editor
      // was thrown away and built again - which is what closing and reopening the file did.
      lineMarkerChange: (update) => update.startState.field(breakLines) !== update.state.field(breakLines),
      initialSpacer: () => spacer,
      domEventHandlers: {
        mousedown(view, line) {
          useDebugStore.getState().toggleBreakpoint(program(), view.state.doc.lineAt(line.from).number);
          return true;
        },
      },
    }),
    EditorView.baseTheme({
      '.fx-breakpoint-gutter': { width: '14px', cursor: 'pointer' },
      '.fx-breakpoint': { color: 'var(--colorPaletteRedForeground1, #c00)' },
      '.fx-breakpoint-spacer': { visibility: 'hidden' },
      // an empty line's element is there to be clicked, and says so when the pointer is over it
      '.fx-breakpoint-gutter .cm-gutterElement:hover': {
        backgroundColor: 'var(--colorNeutralBackground3Hover, rgba(0, 0, 0, 0.06))',
      },
    }),
  ];
}
