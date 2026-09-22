import { StrictMode } from 'react';
import { createRoot } from 'react-dom/client';
import { App } from './App';
import { setLanguageService } from './editor/LanguageService';
import { foxproService } from './editor/foxproService';
import './styles/global.css';

// diagnostics and completions from the FoxVM compiler; editors created later pick it up
setLanguageService(foxproService);

createRoot(document.getElementById('root')!).render(
  <StrictMode>
    <App />
  </StrictMode>,
);
