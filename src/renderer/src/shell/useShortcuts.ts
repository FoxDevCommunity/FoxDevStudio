import { useEffect } from 'react';
import { findCommandForKey } from './commands/registry';

/** Window-level keyboard shortcuts routed through the command registry. Focused editors stop propagation for keys they own. */
export function useShortcuts(): void {
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.defaultPrevented) return;
      const target = e.target as HTMLElement | null;
      const editing =
        !!target &&
        (target.tagName === 'INPUT' || target.tagName === 'TEXTAREA' || target.isContentEditable || !!target.closest?.('[contenteditable]:not([contenteditable="false"])'));
      // plain keys (Delete, F-keys) belong to text fields; modifier chords are always commands
      if (editing && !e.ctrlKey && !e.metaKey && !e.altKey && !/^F\d+$/.test(e.key)) return;
      const cmd = findCommandForKey(e);
      if (!cmd) return;
      // text editors own their clipboard/undo chords (Ctrl+Z in CodeMirror must not undo the designer)
      if (editing && cmd.id.startsWith('edit.')) return;
      e.preventDefault();
      void cmd.run();
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, []);
}
