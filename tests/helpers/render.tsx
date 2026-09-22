import type { ReactElement } from 'react';
import { render, type RenderOptions } from '@testing-library/react';
import { FluentProvider, webLightTheme } from '@fluentui/react-components';

export function renderWithProviders(ui: ReactElement, options?: RenderOptions) {
  return render(<FluentProvider theme={webLightTheme}>{ui}</FluentProvider>, options);
}
