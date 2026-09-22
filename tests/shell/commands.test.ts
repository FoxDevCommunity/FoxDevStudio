import { describe, expect, it } from 'vitest';
import { findCommandForKey, matchesShortcut, parseShortcut, registerCommands, runCommand } from '@renderer/shell/commands/registry';

describe('command registry', () => {
  it('parses and matches shortcuts', () => {
    expect(parseShortcut('Ctrl+Shift+S')).toEqual({ key: 's', ctrl: true, shift: true, alt: false });
    expect(parseShortcut('F4')).toEqual({ key: 'f4', ctrl: false, shift: false, alt: false });
    expect(matchesShortcut(new KeyboardEvent('keydown', { key: 'S', ctrlKey: true, shiftKey: true }), 'Ctrl+Shift+S')).toBe(true);
    expect(matchesShortcut(new KeyboardEvent('keydown', { key: 's', ctrlKey: true }), 'Ctrl+Shift+S')).toBe(false);
    expect(matchesShortcut(new KeyboardEvent('keydown', { key: 's', metaKey: true }), 'Ctrl+S')).toBe(true);
  });

  it('runs enabled commands and resolves keys to commands', async () => {
    let ran = 0;
    let enabled = false;
    registerCommands([{ id: 'test.x', label: 'X', shortcut: 'Ctrl+Alt+X', isEnabled: () => enabled, run: () => void ran++ }]);
    expect(await runCommand('test.x')).toBe(false);
    expect(findCommandForKey(new KeyboardEvent('keydown', { key: 'x', ctrlKey: true, altKey: true }))).toBeUndefined();
    enabled = true;
    expect(await runCommand('test.x')).toBe(true);
    expect(ran).toBe(1);
    expect(findCommandForKey(new KeyboardEvent('keydown', { key: 'x', ctrlKey: true, altKey: true }))?.id).toBe('test.x');
    expect(await runCommand('nope')).toBe(false);
  });
});
