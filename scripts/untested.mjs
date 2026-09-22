// Every element of the Visual FoxPro language that no golden program claims to exercise.
//
// A golden claims an element with a `* COVERS: NAME, NAME` line, and that claim is what makes
// the element tested rather than merely present. "Present" is a weak thing to know: for a
// command it means a one-line probe of it compiles without a syntax error, and for a property
// that its name is in the registry. Neither says the behaviour is right.
//
//   node scripts/untested.mjs              # counts by kind
//   node scripts/untested.mjs command      # the names of one kind, one per line
//   node scripts/untested.mjs --all        # every kind, as markdown

import { readdirSync, readFileSync } from 'node:fs';

const PROGRAMS = 'crates/foxvm/tests/programs';
const REFERENCE = 'tests/reference/vfp-language.tsv';

/**
 * Everything a golden says it covers, upper-cased.
 *
 * A name on its own covers every element of that name, which is what almost all of them mean.
 * Where the reference has one name twice - `Help` is a method of the application and `HELP` is a
 * command that opens a window - a claim may say which by writing `method:Help`, and then it
 * covers that one and leaves the other still to do.
 */
function claimed() {
  const out = new Set();
  for (const file of readdirSync(PROGRAMS).filter((f) => f.endsWith('.prg'))) {
    const source = readFileSync(`${PROGRAMS}/${file}`, 'latin1');
    for (const header of source.matchAll(/^\*\s*COVERS:\s*(.+)$/gim)) {
      for (const name of header[1].split(',')) {
        const key = name.trim().toUpperCase();
        if (key !== '') out.add(key);
      }
    }
  }
  return out;
}

/** Whether a golden claims this element, by name alone or by kind and name. */
function covers(claims, kind, name) {
  return claims.has(name.toUpperCase()) || claims.has(`${kind}:${name}`.toUpperCase());
}

const covered = claimed();
const byKind = new Map();
let total = 0;
for (const row of readFileSync(REFERENCE, 'utf8').split(/\r?\n/).slice(1)) {
  const [kind, name] = row.split('\t');
  if (!kind || !name) continue;
  total += 1;
  if (covers(covered, kind, name)) continue;
  if (!byKind.has(kind)) byKind.set(kind, []);
  byKind.get(kind).push(name);
}

const want = process.argv[2];
if (want === '--all') {
  for (const [kind, names] of [...byKind].sort((a, b) => b[1].length - a[1].length)) {
    console.log(`## ${kind} (${names.length})\n\n${names.join(', ')}\n`);
  }
} else if (want) {
  for (const name of byKind.get(want) ?? []) console.log(name);
} else {
  let missing = 0;
  for (const [kind, names] of [...byKind].sort((a, b) => b[1].length - a[1].length)) {
    missing += names.length;
    console.log(`${kind.padEnd(10)} ${String(names.length).padStart(4)} with no golden`);
  }
  console.log(`\n${total - missing} of ${total} element(s) are exercised by a golden.`);
}
