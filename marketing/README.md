# The FoxDev Studio site

The marketing page and the documentation, built with [Astro](https://astro.build). It is a
separate package from the IDE and shares nothing with it but the facts.

```bash
cd marketing
npm install
npm run dev        # http://localhost:4321 with hot reload
npm run build      # static HTML into dist/
npm run preview    # serve dist/ to check the build
```

## Deploying

The site is a Cloudflare Worker serving the static files in `dist/`, which is what Cloudflare
Pages has become; `wrangler.jsonc` is the whole configuration. A push to `main` that touches
`marketing/` (or `crates/foxvm/src/bytecode.rs`, which the instruction reference is generated
from) deploys it through `.github/workflows/site.yml`. To deploy by hand:

```bash
npx wrangler login     # once
npm run deploy         # astro build, then wrangler deploy
```

The workflow uses an API token instead of a login: `CLOUDFLARE_API_TOKEN` with
Account > Workers Scripts > Edit, and `CLOUDFLARE_ACCOUNT_ID`, both repository secrets.

## Layout

```text
src/pages/index.astro        the landing page
src/pages/docs/*.md          the documentation: one Markdown file per page, each naming its layout
src/pages/docs/instructions.astro  the instruction reference, rendered from generated data
src/data/instructions.json   generated at build time from the VM source (not committed)
scripts/instructions.mjs     the generator, run by prebuild and predev
src/data/nav.ts              the sidebar: every page, in order, with a one-line summary
src/layouts/BaseLayout.astro the head every page shares (fonts, title, description)
src/layouts/DocsLayout.astro sidebar, prose styles, the on-this-page list and the pager
src/components               masthead and footer
src/styles/global.css        the design tokens, light and dark
src/grammars                 the FoxPro grammar the code blocks are highlighted with
public/mark.png              the mark
```

## Generated content

`src/data/instructions.json` is generated, not committed: `npm run build` and `npm run dev` write it
first (`prebuild`/`predev` run `scripts/instructions.mjs`) from `crates/foxvm/src/bytecode.rs`, so a
change to the VM's `Instr` enum reaches the instruction reference on the next build with
nothing to regenerate or commit. `src/pages/docs/instructions.astro` renders it.

## Diagrams

The diagrams are inline SVG written straight into the Markdown pages, styled by the `dg-*`
classes in `DocsLayout.astro` so they follow the theme. Two rules: every colour is a token
(`var(--brand)`, never a literal), and **no blank line inside a diagram block** - Markdown ends
a raw HTML block at the first blank line and renders the rest as prose.

## Adding a page

1. Write `src/pages/docs/<name>.md` with the frontmatter the others have: `layout`, `title`,
   `kicker`, `description`.
2. Add it to `src/data/nav.ts` in the group it belongs to. That is what puts it in the
   sidebar, on the landing page's documentation map, and in the previous/next pager.

## House style

The site is written the way the repository's own documents are: plain ASCII punctuation, no
em-dashes, no typographic quotes; claims that were measured against Visual FoxPro say so;
what is not built yet is said by name rather than left out.
