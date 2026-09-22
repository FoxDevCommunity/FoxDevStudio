import { useEffect, useRef } from 'react';
import { EditorState, type Extension } from '@codemirror/state';
import { EditorView, keymap, lineNumbers, highlightActiveLine, drawSelection } from '@codemirror/view';
import { defaultKeymap, history, historyKeymap, indentWithTab } from '@codemirror/commands';
import { searchKeymap, highlightSelectionMatches } from '@codemirror/search';
import { defaultHighlightStyle, syntaxHighlighting, indentOnInput, bracketMatching } from '@codemirror/language';
import { getLanguageService, type LanguageContext } from './LanguageService';
import './editor.css';

export interface CodeEditorProps {
  value: string;
  onChange(value: string): void;
  onBlur?(): void;
  readOnly?: boolean;
  ariaLabel?: string;
  extensions?: Extension[];
  autoFocus?: boolean;
  /** Tells the language service what is being edited; defaults to a program. */
  languageContext?: LanguageContext;
}

const PROGRAM_CONTEXT: LanguageContext = { kind: 'program' };

/**
 * CodeMirror 6 wrapper. The view is created once; `value` changes from outside (undo in the
 * designer, switching methods) replace the document only when they differ from what is shown.
 * Tests reach the view with `EditorView.findFromDOM(container)`.
 */
export function CodeEditor({
  value,
  onChange,
  onBlur,
  readOnly = false,
  ariaLabel,
  extensions = [],
  autoFocus = false,
  languageContext = PROGRAM_CONTEXT,
}: CodeEditorProps) {
  const hostRef = useRef<HTMLDivElement>(null);
  const viewRef = useRef<EditorView | null>(null);
  const callbacks = useRef({ onChange, onBlur });
  useEffect(() => {
    callbacks.current = { onChange, onBlur };
  });

  useEffect(() => {
    const view = new EditorView({
      parent: hostRef.current!,
      state: EditorState.create({
        doc: value,
        extensions: [
          lineNumbers(),
          history(),
          drawSelection(),
          highlightActiveLine(),
          highlightSelectionMatches(),
          bracketMatching(),
          indentOnInput(),
          syntaxHighlighting(defaultHighlightStyle, { fallback: true }),
          ...getLanguageService().editorExtensions(languageContext),
          keymap.of([...defaultKeymap, ...historyKeymap, ...searchKeymap, indentWithTab]),
          EditorState.readOnly.of(readOnly),
          EditorView.contentAttributes.of({ 'aria-label': ariaLabel ?? 'Code' }),
          EditorView.updateListener.of((u) => {
            if (u.docChanged) callbacks.current.onChange(u.state.doc.toString());
          }),
          EditorView.domEventHandlers({ blur: () => callbacks.current.onBlur?.() }),
          ...extensions,
        ],
      }),
    });
    viewRef.current = view;
    if (autoFocus) view.focus();
    return () => {
      view.destroy();
      viewRef.current = null;
    };
    // the editor is created once; later prop changes are applied through dispatch below
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  useEffect(() => {
    const view = viewRef.current;
    if (!view) return;
    const current = view.state.doc.toString();
    // CodeMirror stores newlines only, so a value that differs from the buffer in line endings
    // alone is not a change: dispatching it would report an edit nobody made.
    if (current === value || current === value.replace(/\r\n?/g, '\n')) return;
    view.dispatch({ changes: { from: 0, to: current.length, insert: value }, selection: { anchor: Math.min(view.state.selection.main.anchor, value.length) } });
  }, [value]);

  return <div ref={hostRef} className="fx-code-editor" data-testid="code-editor" />;
}
