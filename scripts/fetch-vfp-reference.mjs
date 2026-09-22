// Rebuilds tests/reference/vfp-language.tsv from the Visual FoxPro 9 language reference.
//
// The reference is the list of everything the language has; this project's coverage is measured
// against it rather than against whatever a sample happened to use. The pages are the index
// pages of vfphelp.com, which is the community-maintained copy of the VFP 9 SP2 help file
// (Creative Commons BY 3.0, github.com/VFPX/HelpFile).
//
// Run: node scripts/fetch-vfp-reference.mjs
/* global fetch */
import { writeFileSync, mkdirSync } from 'node:fs';

const BASE = 'https://www.vfphelp.com/help/';

/** Index page per kind, and how a title on it reads when it names an element of that kind. */
const PAGES = [
  ['command', '_5WN12PCJD.htm', /^(.*?)\s+Commands?$/],
  ['function', '_5WN12PP3Y.htm', /^(.*?)\(\s*\)\s+Functions?$/],
  ['directive', '_5WN12PTWQ.htm', /^(.*?)\s+Preprocessor Directive$/],
  ['systemvar', '_5WN12PCH1.htm', /^(.*?)\s+System Variable$/],
  ['property', '_5WN12PIMC.htm', /^(.*?)\s+Property$/],
  ['method', '_5WN12PMRX.htm', /^(.*?)\s+Method$/],
  ['event', '_5WN12PNS3.htm', /^(.*?)\s+Event$/],
  ['object', '_5WN12PFWX.htm', /^(.*?)\s+(?:Object|Control|Class)$/],
];

/** Strips the qualifier VFP adds when a name means different things in different places. */
function withoutQualifier(title) {
  return title.replace(/\s*\((?:Visual FoxPro|VFP)[^)]*\)\s*$/i, '').trim();
}

/** Markup as the text it stands for. */
function plain(html) {
  return html
    .replace(/<[^>]+>/g, '')
    .replace(/&amp;/g, '&')
    .replace(/&lt;/g, '<')
    .replace(/&gt;/g, '>')
    .replace(/&nbsp;/g, ' ')
    .replace(/\s+/g, ' ')
    .trim();
}

/**
 * The operators, which the reference lists in tables rather than one page each: the first
 * cell of a row is the operator, or several separated by commas when one page covers them all.
 */
async function operators() {
  const index = await fetch(BASE + '_5WN12PTUQ.htm').then((r) => r.text());
  const out = [];
  const seen = new Set();
  for (const link of index.matchAll(/<a[^>]+href="(_5WN12[^"]+)\.htm"[^>]*>([^<]*Operators)<\/a>/gi)) {
    const [, href, group] = link;
    const page = (await fetch(BASE + href + '.htm').then((r) => r.text())).replace(/\s+/g, ' ');
    for (const row of page.matchAll(/<tr[^>]*>(.*?)<\/tr>/gi)) {
      const cells = [...row[1].matchAll(/<td[^>]*>(.*?)<\/td>/gi)].map((c) => plain(c[1]));
      if (cells.length < 2 || !cells[0] || cells[0].length > 12) continue;
      for (const name of cells[0].split(',').map((n) => n.trim()).filter(Boolean)) {
        // one operator is documented once per type it works on: + adds numbers, joins text
        // and moves a date on, and is one operator of the language either way
        if (seen.has(name)) continue;
        seen.add(name);
        out.push(['operator', name, `${name} - ${cells[1]} (${group.trim()})`, href]);
      }
    }
  }
  return out;
}

const rows = [];
for (const [kind, page, shape] of PAGES) {
  const html = await fetch(BASE + page).then((r) => r.text());
  const seen = new Set();
  let kept = 0;
  for (const link of html.matchAll(/<a[^>]+href="(_5WN12[^"]+)\.htm"[^>]*>([\s\S]*?)<\/a>/gi)) {
    const href = link[1];
    const title = link[2]
      .replace(/<[^>]+>/g, '')
      .replace(/&amp;/g, '&')
      .replace(/&lt;/g, '<')
      .replace(/&gt;/g, '>')
      .replace(/&nbsp;/g, ' ')
      .replace(/\s+/g, ' ')
      .trim();
    if (!title || seen.has(href)) continue;

    // an index page also links to its neighbours and to category pages; only titles that read
    // as "<name> <kind>" name an element
    const match = shape.exec(withoutQualifier(title));
    if (!match) continue;
    // One page may document several names: "PADL( ) | PADR( ) | PADC( ) Functions", and
    // "CD | CHDIR Command". Each name is an element of its own.
    const names = match[1]
      .split('|')
      .map((part) => part.replace(/\(\s*\)/g, '').trim())
      .filter((part) => part !== '');
    // "SET Command Overview", "SYS( ) Functions Overview": pages about a family, not an element
    if (names.length === 0 || names.some((n) => /\bOverview$/i.test(n))) continue;

    seen.add(href);
    for (const name of names) rows.push([kind, name, title, href]);
    kept += names.length;
  }
  console.log(`${kind}: ${kept}`);
}

const ops = await operators();
rows.push(...ops);
console.log(`operator: ${ops.length}`);

rows.sort((a, b) => a[0].localeCompare(b[0]) || a[1].localeCompare(b[1]));
mkdirSync('tests/reference', { recursive: true });
writeFileSync(
  'tests/reference/vfp-language.tsv',
  ['kind\tname\ttitle\thref', ...rows.map((r) => r.join('\t'))].join('\n') + '\n',
);
console.log(`total ${rows.length}`);
