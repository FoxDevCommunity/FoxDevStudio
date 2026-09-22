/**
 * Entry point for the runtime player window: a compiled `.fxa` application and nothing else.
 * No language service is installed - the player never edits code, it only runs it.
 */

import { StrictMode } from 'react';
import { createRoot } from 'react-dom/client';
import { PlayerApp } from './PlayerApp';
import '../styles/global.css';

createRoot(document.getElementById('root')!).render(
  <StrictMode>
    <PlayerApp />
  </StrictMode>,
);
