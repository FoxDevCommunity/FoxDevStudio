import type { Extension } from '@codemirror/state';
import { foxproLanguage } from './foxproMode';

/** What the editor is showing, so diagnostics can be tailored (implicit method parameters etc.). */
export interface LanguageContext {
  kind: 'program' | 'method';
  /** VFP parameter list of the event a method body belongs to, e.g. "nKeyCode, nShiftAltCtrl". */
  params?: string;
}

/**
 * Editor language support. The placeholder ships syntax colouring only; the real service backed
 * by the FoxVM compiler adds diagnostics and completions (see `foxproService.ts`).
 */
export interface LanguageService {
  id: string;
  editorExtensions(context: LanguageContext): Extension[];
}

export const placeholderFoxPro: LanguageService = {
  id: 'foxpro-placeholder',
  editorExtensions: () => [foxproLanguage],
};

let current: LanguageService = placeholderFoxPro;
export function getLanguageService(): LanguageService {
  return current;
}
export function setLanguageService(service: LanguageService): void {
  current = service;
}
