import type { MenuHotkey } from '@shared/menu/schema';

/** Parses VFP-style shortcut text ("CTRL+SHIFT+S", "F5", "Alt+X"). Returns null when no key is present. */
export function parseHotkey(text: string): MenuHotkey | null {
  const parts = text.split('+').map((p) => p.trim()).filter(Boolean);
  if (parts.length === 0) return null;
  const key = parts[parts.length - 1]!;
  const mods = parts.slice(0, -1).map((m) => m.toUpperCase());
  const known = new Set(['CTRL', 'CONTROL', 'SHIFT', 'ALT']);
  if (mods.some((m) => !known.has(m))) return null;
  const out: MenuHotkey = { key: key.length === 1 ? key.toUpperCase() : key };
  if (mods.includes('CTRL') || mods.includes('CONTROL')) out.ctrl = true;
  if (mods.includes('SHIFT')) out.shift = true;
  if (mods.includes('ALT')) out.alt = true;
  return out;
}

export function formatHotkey(h: MenuHotkey | undefined): string {
  if (!h) return '';
  if (h.label) return h.label;
  return [h.ctrl ? 'Ctrl' : '', h.alt ? 'Alt' : '', h.shift ? 'Shift' : '', h.key].filter(Boolean).join('+');
}

/** Builds a hotkey from a keyboard event, or null for a bare modifier. */
export function hotkeyFromEvent(e: { key: string; ctrlKey: boolean; shiftKey: boolean; altKey: boolean }): MenuHotkey | null {
  if (['Control', 'Shift', 'Alt', 'Meta'].includes(e.key)) return null;
  const key = e.key.length === 1 ? e.key.toUpperCase() : e.key;
  const out: MenuHotkey = { key };
  if (e.ctrlKey) out.ctrl = true;
  if (e.shiftKey) out.shift = true;
  if (e.altKey) out.alt = true;
  return out;
}

export interface PromptLabel {
  label: string;
  /** Index of the mnemonic character in `label`, or -1. */
  index: number;
  mnemonic: string | null;
}

/** "\<File" -> { label: "File", index: 0, mnemonic: "F" }. "\-" is a separator. */
export function promptToLabel(prompt: string): PromptLabel {
  if (prompt === '\\-') return { label: '', index: -1, mnemonic: null };
  const idx = prompt.indexOf('\\<');
  if (idx < 0) return { label: prompt, index: -1, mnemonic: null };
  const label = prompt.slice(0, idx) + prompt.slice(idx + 2);
  const mnemonic = label[idx] ?? null;
  return { label, index: mnemonic ? idx : -1, mnemonic };
}
