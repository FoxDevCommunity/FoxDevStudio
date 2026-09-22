import { act, screen, waitFor } from '@testing-library/react';
import { afterAll, beforeAll, beforeEach, describe, expect, it } from 'vitest';
import { EditorView } from '@codemirror/view';
import { useDocumentsStore } from '@renderer/stores/documentsStore';
import { DocumentArea } from '@renderer/shell/DocumentArea';
import { placeholderFoxPro, setLanguageService } from '@renderer/editor/LanguageService';
import { foxproService } from '@renderer/editor/foxproService';
import { loadFoxVm } from '../../src/wasm/foxvm/loader';
import { renderWithProviders } from '../helpers/render';

const view = () => EditorView.findFromDOM(screen.getByTestId('code-editor'))!;
const lintMarks = () => document.querySelectorAll('.cm-lintRange');

beforeAll(() => setLanguageService(foxproService));
afterAll(() => setLanguageService(placeholderFoxPro));
beforeEach(() => useDocumentsStore.getState().closeAll());

describe('FoxPro language service', () => {
  it('reports parser diagnostics with editor offsets', async () => {
    const vm = await loadFoxVm();
    const result = vm.check('x = 1\nIF x\n', 'program') as { diagnostics: { line: number; col: number; start: number; message: string; severity: string }[] };
    expect(result.diagnostics).toHaveLength(1);
    expect(result.diagnostics[0]).toMatchObject({ line: 2, col: 1, start: 6, severity: 'error' });
    expect(result.diagnostics[0]!.message).toMatch(/ENDIF/);
    expect((vm.check('x = 1\n', 'program') as { diagnostics: unknown[] }).diagnostics).toHaveLength(0);
  });

  it('underlines a syntax error in the program editor and clears it when fixed', async () => {
    useDocumentsStore.getState().openProgram('IF x\n', '/p/main.prg');
    renderWithProviders(<DocumentArea />);
    await screen.findByTestId('code-editor');
    await waitFor(() => expect(lintMarks().length).toBeGreaterThan(0), { timeout: 4000 });

    act(() => view().dispatch({ changes: { from: 0, to: view().state.doc.length, insert: 'IF x\nENDIF\n' } }));
    await waitFor(() => expect(lintMarks().length).toBe(0), { timeout: 4000 });
  });
});
