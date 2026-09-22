/**
 * Jumps to the code a program named: where an error was raised, and where the debugger has
 * stopped. The VM reports `PROGRAM()` as `cmdSayHi.Click` for a form method or the program's
 * own name for a `.prg`, which is enough to find the right tab.
 */

import { findByName } from '@shared/form/tree';
import { baseName } from '@shared/runtime/programSource';
import { FORM_ID } from '../designer/store/createFormDesignerStore';
import { useDocumentsStore } from '../stores/documentsStore';

/** Opens the method or program tab a runtime error came from. Returns false when not found. */
export function openMethodAt(program: string, _line: number): boolean {
  const docs = Object.values(useDocumentsStore.getState().docs);
  const dot = program.lastIndexOf('.');

  if (dot > 0) {
    const objectPath = program.slice(0, dot);
    const event = program.slice(dot + 1);
    const leaf = objectPath.split('.').pop()!;
    for (const doc of docs) {
      if (doc.kind !== 'form') continue;
      const form = doc.store.getState().doc.form;
      const control = findByName(form, leaf);
      if (control) {
        useDocumentsStore.getState().openMethod(doc.id, control.id, event);
        return true;
      }
      if (form.name.toLowerCase() === objectPath.toLowerCase()) {
        useDocumentsStore.getState().openMethod(doc.id, FORM_ID, event);
        return true;
      }
    }
  }

  // a .prg: focus its tab if it is open
  const wanted = baseName(program).toLowerCase();
  const open = docs.find((d) => d.kind === 'program' && d.path && baseName(d.path).toLowerCase().endsWith(wanted));
  if (open) {
    useDocumentsStore.getState().activate(open.id);
    return true;
  }
  return false;
}
