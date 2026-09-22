import { screen } from '@testing-library/react';
import type { FormDocument } from '@shared/form/schema';
import { createFormDesignerStore, type FormDesignerStore } from '@renderer/designer/store/createFormDesignerStore';
import { FormDesignerProvider } from '@renderer/designer/store/FormDesignerContext';
import { FormDesignerDocument } from '@renderer/designer/FormDesignerDocument';
import { PropertiesWindow } from '@renderer/designer/properties/PropertiesWindow';
import { useSettingsStore } from '@renderer/stores/settingsStore';
import { renderWithProviders } from './render';
import { sampleForm } from './fixtures';

/** Renders a form designer with its properties window, like the IDE does, and returns the store. */
export function renderDesigner(doc: FormDocument = sampleForm(), opts: { grid?: number; snap?: boolean } = {}) {
  useSettingsStore.setState({ gridSize: opts.grid ?? 8, snapToGrid: opts.snap ?? true, showGrid: true });
  let n = 0;
  const store: FormDesignerStore = createFormDesignerStore(doc, { newId: () => `n${++n}` });
  const utils = renderWithProviders(
    <div style={{ display: 'flex' }}>
      <FormDesignerDocument docId="doc1" store={store} />
      <FormDesignerProvider store={store} docId="doc1">
        <PropertiesWindow />
      </FormDesignerProvider>
    </div>,
  );
  return { store, ...utils };
}

export const control = (id: string) => document.querySelector<HTMLElement>(`[data-control-id="${id}"]`)!;
export const handle = (id: string, h: string) => control(id).querySelector<HTMLElement>(`[data-handle="${h}"]`)!;
export const canvas = () => screen.getByTestId('designer-canvas');
export const client = () => screen.getByTestId('form-client');
