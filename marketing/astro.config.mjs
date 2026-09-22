// @ts-check
import { defineConfig } from 'astro/config';
import { satteri } from '@astrojs/markdown-satteri';
import { readFileSync } from 'node:fs';

// Shiki has no FoxPro grammar, so the site carries a small one of its own: comments, strings,
// dates, the block keywords, the SQL words, the two FoxScript keywords and the namespace.
const foxpro = JSON.parse(readFileSync(new URL('./src/grammars/foxpro.tmLanguage.json', import.meta.url), 'utf8'));

// The site is static: every page is HTML the build writes once. The docs are Markdown pages
// under src/pages/docs, each naming the layout that wraps it.
export default defineConfig({
  site: 'https://foxscript.org',
  trailingSlash: 'never',
  build: {
    format: 'file',
  },
  markdown: {
    // the site is plain ASCII punctuation throughout: no curly quotes, dashes or ellipses,
    // so the processor's smart punctuation is off
    processor: satteri({ features: { smartPunctuation: false } }),
    shikiConfig: {
      // one theme per side; the listing ground is always the dark one, so the dark token
      // colours are what the page shows in both themes (see DocsLayout.astro)
      themes: { light: 'github-light', dark: 'github-dark' },
      langs: [foxpro],
      wrap: false,
    },
  },
});
