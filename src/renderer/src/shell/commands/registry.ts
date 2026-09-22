import { create } from 'zustand';

/**
 * Central command registry: menus, shortcuts, the native menu and tests all run the same ids.
 */
export interface Command {
  id: string;
  label: string;
  /** Display and binding form, e.g. "Ctrl+S", "Ctrl+Shift+S", "F5", "Delete". */
  shortcut?: string;
  run(): void | Promise<void>;
  isEnabled?(): boolean;
  isChecked?(): boolean;
}

interface RegistryState {
  commands: Record<string, Command>;
  /** Bumped whenever a command is (re)registered so menus re-render. */
  version: number;
}

export const useCommandRegistry = create<RegistryState>(() => ({ commands: {}, version: 0 }));

export function registerCommands(list: Command[]): void {
  useCommandRegistry.setState((s) => {
    const commands = { ...s.commands };
    for (const c of list) commands[c.id] = c;
    return { commands, version: s.version + 1 };
  });
}

export function getCommand(id: string): Command | undefined {
  return useCommandRegistry.getState().commands[id];
}

export function isCommandEnabled(id: string): boolean {
  const c = getCommand(id);
  return !!c && (c.isEnabled?.() ?? true);
}

/** Runs a command if it exists and is enabled. Resolves to whether it ran. */
export async function runCommand(id: string): Promise<boolean> {
  const c = getCommand(id);
  if (!c || !(c.isEnabled?.() ?? true)) return false;
  await c.run();
  return true;
}

export interface ParsedShortcut {
  key: string;
  ctrl: boolean;
  shift: boolean;
  alt: boolean;
}

export function parseShortcut(text: string): ParsedShortcut {
  const parts = text.split('+').map((p) => p.trim());
  const key = parts[parts.length - 1]!.toLowerCase();
  const mods = parts.slice(0, -1).map((p) => p.toLowerCase());
  return { key, ctrl: mods.includes('ctrl') || mods.includes('cmd'), shift: mods.includes('shift'), alt: mods.includes('alt') };
}

export function matchesShortcut(e: KeyboardEvent, shortcut: string): boolean {
  const s = parseShortcut(shortcut);
  const key = e.key.toLowerCase();
  const eventKey = key === ' ' ? 'space' : key;
  return eventKey === s.key && (e.ctrlKey || e.metaKey) === s.ctrl && e.shiftKey === s.shift && e.altKey === s.alt;
}

/** Finds the enabled command bound to a keyboard event. */
export function findCommandForKey(e: KeyboardEvent): Command | undefined {
  for (const c of Object.values(useCommandRegistry.getState().commands)) {
    if (c.shortcut && matchesShortcut(e, c.shortcut) && (c.isEnabled?.() ?? true)) return c;
  }
  return undefined;
}
