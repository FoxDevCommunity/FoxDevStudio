/**
 * Where the stopped program is: the line the debugger is on, marked in the editor showing it.
 *
 * A developer stepping needs to see the line move. The debugger knows which source it stopped
 * in and at which line - and the call stack lets a frame further out be chosen, in which case
 * that frame's line is the one to show, because that is what Locals and the watches are being
 * read against.
 *
 * The two things CodeMirror needs telling, both of which the breakpoint gutter got wrong first:
 * a gutter builds an element only for a line that already has a marker, and it only recomputes
 * its markers when the document or the viewport changes. Setting a line here is neither, so it
 * has to say that its own state moving is a reason to paint.
 */

import { StateEffect, StateField, type Extension } from '@codemirror/state';
import { Decoration, EditorView, GutterMarker, ViewPlugin, gutter, type DecorationSet } from '@codemirror/view';
import { useDebugStore } from '../runtime/debugSession';

/** The line this editor's source is stopped at, or 0 when it is not the one stopped in. */
const setLine = StateEffect.define<number>();

const stoppedLine = StateField.define<number>({
  create: () => 0,
  update(line, tr) {
    for (const effect of tr.effects) {
      if (effect.is(setLine)) return effect.value;
    }
    return line;
  },
});

const stoppedRow = Decoration.line({ class: 'fx-stopped-line' });

/** The whole line, so it reads as "here" rather than as a selection. */
const decorations = StateField.define<DecorationSet>({
  create: () => Decoration.none,
  update(marks, tr) {
    const line = tr.state.field(stoppedLine, false) ?? 0;
    if (line < 1 || line > tr.state.doc.lines) return Decoration.none;
    return Decoration.set([stoppedRow.range(tr.state.doc.line(line).from)]);
  },
  provide: (field) => EditorView.decorations.from(field),
});

class Arrow extends GutterMarker {
  constructor(private readonly className: string) {
    super();
  }

  override toDOM(): Node {
    const arrow = document.createElement('span');
    arrow.className = this.className;
    arrow.textContent = '▶';
    return arrow;
  }
}

const arrow = new Arrow('fx-stopped-arrow');
/** The same shape, drawn on nothing: CodeMirror keeps this one to hold the column open. */
const spacer = new Arrow('fx-stopped-spacer');

/**
 * `program` is read when it is needed rather than captured, because a method editor can be
 * pointed at another object and event without being rebuilt.
 */
export function currentLine(program: () => string): Extension {
  /** The line to show: the chosen frame's if it is in this source, otherwise none. */
  const lineFor = (name: string): number => {
    const { stop, frames, level } = useDebugStore.getState();
    if (!stop) return 0;
    const frame = frames[level];
    const showing = frame ?? { program: stop.program, line: stop.line };
    return showing.program.toUpperCase() === name.toUpperCase() ? showing.line : 0;
  };

  return [
    stoppedLine,
    decorations,
    ViewPlugin.define((view) => {
      // dispatching from inside a view update or a store write is not allowed, so every push
      // waits for the stack to unwind first
      const push = () =>
        queueMicrotask(() => {
          if (!view.dom.isConnected) return;
          const line = lineFor(program());
          if (line === view.state.field(stoppedLine, false)) return;
          view.dispatch({ effects: setLine.of(line) });
          // bring it into view, the way stepping through a long procedure needs
          if (line >= 1 && line <= view.state.doc.lines) {
            view.dispatch({ effects: EditorView.scrollIntoView(view.state.doc.line(line).from, { y: 'center' }) });
          }
        });
      push();
      const unsubscribe = useDebugStore.subscribe(push);
      return { update: () => {}, destroy: unsubscribe };
    }),
    gutter({
      class: 'fx-stopped-gutter',
      lineMarker: (view, line) =>
        view.state.field(stoppedLine) === view.state.doc.lineAt(line.from).number ? arrow : null,
      lineMarkerChange: (update) => update.startState.field(stoppedLine) !== update.state.field(stoppedLine),
      initialSpacer: () => spacer,
    }),
    EditorView.baseTheme({
      '.fx-stopped-gutter': { width: '12px' },
      '.fx-stopped-arrow': { color: 'var(--colorPaletteYellowForeground1, #b58900)' },
      '.fx-stopped-spacer': { visibility: 'hidden' },
      '.fx-stopped-line': { backgroundColor: 'var(--colorPaletteYellowBackground2, rgba(255, 214, 0, 0.28))' },
    }),
  ];
}
