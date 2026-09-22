import js from '@eslint/js';
import tseslint from 'typescript-eslint';
import reactHooks from 'eslint-plugin-react-hooks';

export default tseslint.config(
  // .claude/worktrees holds checkouts of this same repo while an agent works in one; they are
  // not this checkout's source
  // marketing is the website: a package of its own, with its own toolchain
  { ignores: ['node_modules', 'out', 'dist', 'release', 'coverage', 'target', 'src/wasm/foxvm/generated', '.claude', 'marketing'] },
  js.configs.recommended,
  {
    // build scripts run under Node
    files: ['scripts/**/*.mjs'],
    languageOptions: {
      sourceType: 'module',
      globals: { console: 'readonly', process: 'readonly', setTimeout: 'readonly', clearTimeout: 'readonly' },
    },
  },
  ...tseslint.configs.recommended,
  {
    files: ['**/*.{ts,tsx}'],
    plugins: { 'react-hooks': reactHooks },
    rules: {
      ...reactHooks.configs.recommended.rules,
      '@typescript-eslint/no-unused-vars': ['error', { argsIgnorePattern: '^_', varsIgnorePattern: '^_' }],
    },
  },
  {
    // shared code must stay free of electron and react so it runs in main, preload, renderer and tests
    files: ['src/shared/**/*.ts'],
    rules: {
      'no-restricted-imports': [
        'error',
        { patterns: ['electron', 'electron/*', 'react', 'react-dom', 'react/*', '@renderer/*', '@main/*', '@fluentui/*'] },
      ],
    },
  },
);
