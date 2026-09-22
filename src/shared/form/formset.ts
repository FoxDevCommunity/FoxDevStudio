/**
 * Where the forms of a formset live once a `.scx` has been imported.
 *
 * A formset holds several forms and a form is what the designer draws, so one file becomes
 * several documents. The first of them takes the file's own name, because that is the name
 * `DO FORM anim` asks for; the rest are told apart by the form's name after it. Both the
 * importer, which writes them, and the runtime, which opens them, work the names out here so
 * there is one rule rather than two.
 */

import type { FormDocument, VfpFormset } from './schema';

/** The document name the member form at `index` of a formset imported from `fileStem` takes. */
export function formsetDocumentName(fileStem: string, formName: string, index: number): string {
  return index === 0 ? fileStem : `${fileStem}.${formName}`;
}

/**
 * The file the formset came from, worked out from one of its member documents: its own name,
 * less the form name appended to it. The first member kept the file's name unchanged.
 */
export function formsetFileStem(documentName: string, formName: string): string {
  const suffix = `.${formName}`;
  return documentName.toLowerCase().endsWith(suffix.toLowerCase()) ? documentName.slice(0, -suffix.length) : documentName;
}

/**
 * Every member form of a formset, as the name of the document each was written to, given any one
 * of them. The document that was asked for is in the list at the place its form holds.
 */
export function formsetDocumentNames(documentName: string, doc: FormDocument, formset: VfpFormset): string[] {
  const stem = formsetFileStem(documentName, doc.form.name);
  return formset.forms.map((name, i) => formsetDocumentName(stem, name, i));
}
