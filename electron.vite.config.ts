import { resolve } from 'node:path';
import { defineConfig, externalizeDepsPlugin } from 'electron-vite';
import react from '@vitejs/plugin-react';

const alias = {
  '@shared': resolve('src/shared'),
  '@renderer': resolve('src/renderer/src'),
  '@main': resolve('src/main'),
};

export default defineConfig({
  main: {
    plugins: [externalizeDepsPlugin()],
    resolve: { alias },
  },
  preload: {
    plugins: [externalizeDepsPlugin()],
    resolve: { alias },
    // sandboxed preload scripts cannot be ES modules
    build: { rollupOptions: { output: { format: 'cjs' } } },
  },
  renderer: {
    resolve: { alias },
    plugins: [react()],
    // two pages: the IDE window and the standalone runtime player
    build: {
      rollupOptions: {
        input: {
          index: resolve('src/renderer/index.html'),
          player: resolve('src/renderer/player.html'),
        },
      },
    },
  },
});
