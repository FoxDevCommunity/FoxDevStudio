import { autocompletion, type CompletionContext, type CompletionResult } from '@codemirror/autocomplete';
import { linter, type Diagnostic as EditorDiagnostic } from '@codemirror/lint';
import type { EditorView } from '@codemirror/view';
import { FOXPRO_FUNCTIONS, FOXPRO_KEYWORDS, FOXPRO_OBJECT_REFS } from '@shared/language/foxproKeywords';
import { loadFoxVm } from '../../../wasm/foxvm/loader';
import { foxproLanguage } from './foxproMode';
import type { LanguageContext, LanguageService } from './LanguageService';

/** Shape of `check()` from the wasm module (serde camelCase of the Rust Diagnostic). */
interface VmDiagnostic {
  severity: 'error' | 'warning';
  message: string;
  line: number;
  col: number;
  endLine: number;
  endCol: number;
  start: number;
  end: number;
}
interface CheckResult {
  diagnostics: VmDiagnostic[];
}

/** Diagnostics from the FoxVM parser. Awaits the wasm load so editors created early still lint. */
async function lintSource(view: EditorView, context: LanguageContext): Promise<EditorDiagnostic[]> {
  const vm = await loadFoxVm();
  if (typeof vm.check !== 'function') return []; // stale wasm build (dev only)
  const source = view.state.doc.toString();
  const result = vm.check(source, context.kind) as CheckResult;
  const max = view.state.doc.length;
  return result.diagnostics.map((d) => {
    const from = Math.min(d.start, max);
    const to = Math.min(Math.max(d.end, from + 1), max);
    return { from, to: Math.max(to, from), severity: d.severity, message: d.message };
  });
}

const WORDS = [
  ...FOXPRO_OBJECT_REFS.map((w) => ({ label: w, type: 'variable' })),
  ...FOXPRO_KEYWORDS.map((w) => ({ label: w, type: 'keyword' })),
  ...FOXPRO_FUNCTIONS.map((w) => ({ label: w, type: 'function', apply: `${w}(` })),
].filter((o, i, all) => all.findIndex((x) => x.label === o.label) === i);

function completeWords(ctx: CompletionContext): CompletionResult | null {
  const word = ctx.matchBefore(/[A-Za-z_][A-Za-z0-9_]*/);
  if (!word || (word.from === word.to && !ctx.explicit)) return null;
  if (word.to - word.from < 2 && !ctx.explicit) return null;
  return { from: word.from, options: WORDS, validFor: /^[A-Za-z_][A-Za-z0-9_]*$/ };
}

export const foxproService: LanguageService = {
  id: 'foxpro',
  editorExtensions: (context) => [
    foxproLanguage,
    linter((view) => lintSource(view, context), { delay: 300 }),
    autocompletion({ override: [completeWords], icons: false }),
  ],
};
