import { useMemo } from 'react';
import { basename } from '@shared/paths';
import { baseName } from '@shared/runtime/programSource';
import { useDocumentsStore } from '../stores/documentsStore';
import { breakpointGutter } from './breakpointGutter';
import { currentLine } from './currentLine';
import { CodeEditor } from './CodeEditor';

/** The name a program's frames answer to, which is the name it is compiled under: its stem. */
function programName(docId: string): string {
  const doc = useDocumentsStore.getState().docs[docId];
  return doc?.kind === 'program' && doc.path ? baseName(basename(doc.path)) : '';
}

/** Source editing for .prg files, with a gutter to set breakpoints in and the stopped line. */
export function ProgramDocument({ docId }: { docId: string }) {
  const doc = useDocumentsStore((s) => s.docs[docId]);
  const setText = useDocumentsStore((s) => s.setProgramText);
  // read when it is needed, not captured: a new program is named only once it has been saved
  const extensions = useMemo(
    () => [breakpointGutter(() => programName(docId)), currentLine(() => programName(docId))],
    [docId],
  );
  if (!doc || doc.kind !== 'program') return null;
  return <CodeEditor value={doc.text} onChange={(v) => setText(docId, v)} ariaLabel="Program source" extensions={extensions} />;
}
