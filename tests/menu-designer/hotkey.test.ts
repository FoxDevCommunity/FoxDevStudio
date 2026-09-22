import { describe, expect, it } from 'vitest';
import { formatHotkey, hotkeyFromEvent, parseHotkey, promptToLabel } from '@renderer/menu-designer/hotkey';

describe('menu hotkeys and prompts', () => {
  it('parses and formats shortcuts', () => {
    expect(parseHotkey('CTRL+SHIFT+s')).toEqual({ key: 'S', ctrl: true, shift: true });
    expect(parseHotkey('F5')).toEqual({ key: 'F5' });
    expect(parseHotkey('Alt+X')).toEqual({ key: 'X', alt: true });
    expect(parseHotkey('')).toBeNull();
    expect(parseHotkey('Bogus+X')).toBeNull();
    expect(formatHotkey({ key: 'S', ctrl: true, shift: true })).toBe('Ctrl+Shift+S');
    expect(formatHotkey({ key: 'N', ctrl: true, label: 'Ctrl-N' })).toBe('Ctrl-N');
    expect(formatHotkey(undefined)).toBe('');
    expect(hotkeyFromEvent({ key: 'Control', ctrlKey: true, shiftKey: false, altKey: false })).toBeNull();
    expect(hotkeyFromEvent({ key: 'o', ctrlKey: true, shiftKey: false, altKey: false })).toEqual({ key: 'O', ctrl: true });
  });
  it('extracts VFP prompt mnemonics', () => {
    expect(promptToLabel('\\<File')).toEqual({ label: 'File', index: 0, mnemonic: 'F' });
    expect(promptToLabel('E\\<xit')).toEqual({ label: 'Exit', index: 1, mnemonic: 'x' });
    expect(promptToLabel('Plain')).toEqual({ label: 'Plain', index: -1, mnemonic: null });
    expect(promptToLabel('\\-')).toEqual({ label: '', index: -1, mnemonic: null });
    expect(promptToLabel('Trailing\\<')).toEqual({ label: 'Trailing', index: -1, mnemonic: null });
  });
});
