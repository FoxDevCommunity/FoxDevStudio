import { fireEvent, render } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import { registerCommands } from '@renderer/shell/commands/registry';
import { useShortcuts } from '@renderer/shell/useShortcuts';

function Host() {
  useShortcuts();
  return (
    <div>
      <input aria-label="field" />
      <div contentEditable aria-label="code" tabIndex={0} />
      <button>btn</button>
    </div>
  );
}

describe('window shortcuts', () => {
  it('routes chords to commands except editing chords inside text editors', () => {
    const ran: string[] = [];
    registerCommands([
      { id: 'edit.undo', label: 'Undo', shortcut: 'Ctrl+Z', run: () => void ran.push('undo') },
      { id: 'file.save', label: 'Save', shortcut: 'Ctrl+S', run: () => void ran.push('save') },
      { id: 'edit.delete', label: 'Delete', shortcut: 'Delete', run: () => void ran.push('delete') },
    ]);
    const { getByLabelText, getByText } = render(<Host />);
    fireEvent.keyDown(getByText('btn'), { key: 'z', ctrlKey: true });
    fireEvent.keyDown(getByText('btn'), { key: 'Delete' });
    fireEvent.keyDown(getByLabelText('field'), { key: 'Delete' }); // typing in a field
    fireEvent.keyDown(getByLabelText('field'), { key: 's', ctrlKey: true });
    fireEvent.keyDown(getByLabelText('code'), { key: 'z', ctrlKey: true }); // CodeMirror owns undo
    fireEvent.keyDown(getByLabelText('code'), { key: 's', ctrlKey: true });
    expect(ran).toEqual(['undo', 'delete', 'save', 'save']);
  });
});
