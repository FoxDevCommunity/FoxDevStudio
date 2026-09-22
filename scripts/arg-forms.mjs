// Which functions have an argument form no golden has ever measured.
//
// `scripts/untested.mjs` answers "is this element covered at all"; this answers the question
// that cost us a live bug: is every *form* of it covered. `SET(cSetting)` had a golden and
// `SET(cSetting, nSecondSetting)` did not, so `SET("HELP", 1)` answered the switch instead of
// the help file and a sample's RestoreHelp ran `EVAL("ON")`. The rule that came out of it is in
// `docs/coverage-plan.md`: an element is covered per documented argument form, not per name.
//
// The runtime's own table says how many arguments each function takes - `spec("ACOPY", 2, 5, ..)`
// - so every count between the two is a form, and a form no golden calls is a form nobody has
// measured. What the goldens do exercise is read out of the `.prg` files themselves.
//
//   node scripts/arg-forms.mjs            # the forms no golden calls
//   node scripts/arg-forms.mjs --check    # fails when a form outside the known list appears
//
// `tests/vfp/arg-forms-known.txt` is the list of forms known to be unmeasured; it is what a
// wave of coverage work shrinks, and nothing may be added to it without a reason beside it.

import { readFileSync, readdirSync } from 'node:fs';
import { join, resolve } from 'node:path';

const ROOT = resolve(import.meta.dirname, '..');
const BUILTINS = join(ROOT, 'crates/foxvm/src/builtins');
const PROGRAMS = join(ROOT, 'crates/foxvm/tests/programs');
const KNOWN = join(ROOT, 'tests/vfp/arg-forms-known.txt');

/** Every function the runtime has, with the fewest and the most arguments it takes. */
function arities() {
  const out = new Map();
  for (const file of readdirSync(BUILTINS).filter((f) => f.endsWith('.rs'))) {
    const text = readFileSync(join(BUILTINS, file), 'utf8');
    for (const m of text.matchAll(/spec\("([A-Z0-9_]+)",\s*(\d+),\s*(\d+)/g)) {
      out.set(m[1], { min: Number(m[2]), max: Number(m[3]) });
    }
  }
  return out;
}

/**
 * How many arguments one call was given, reading from just past the opening bracket.
 *
 * Commas inside a nested call, a bracket pair or a string belong to that argument and not to
 * this call, so the depth and the quote have to be tracked; a call whose bracket never closes
 * within the line is left out rather than guessed at.
 */
function argCount(text, start) {
  let depth = 1;
  let args = 1;
  let quote = '';
  let seen = false;
  for (let i = start; i < text.length; i++) {
    const ch = text[i];
    if (quote !== '') {
      if (ch === quote) quote = '';
      continue;
    }
    if (ch === '"' || ch === "'" || ch === '[') {
      // a bracket is a string quote in FoxPro as well as a subscript; either way what is inside
      // it is not this call's commas
      quote = ch === '[' ? ']' : ch;
      seen = true;
      continue;
    }
    if (ch === '(') depth++;
    else if (ch === ')') {
      depth--;
      if (depth === 0) return seen ? args : 0;
    } else if (ch === ',' && depth === 1) args++;
    else if (ch !== ' ' && ch !== '\t') seen = true;
    if (ch === '\n' && depth > 0 && text[i] !== ';') return null;
  }
  return null;
}

/** For every function name, the argument counts some golden calls it with. */
function measured() {
  const out = new Map();
  for (const file of readdirSync(PROGRAMS).filter((f) => f.endsWith('.prg'))) {
    const text = readFileSync(join(PROGRAMS, file), 'utf8');
    for (const m of text.matchAll(/\b([A-Za-z_][A-Za-z0-9_]*)\s*\(/g)) {
      const name = m[1].toUpperCase();
      const n = argCount(text, m.index + m[0].length);
      if (n === null) continue;
      if (!out.has(name)) out.set(name, new Set());
      out.get(name).add(n);
    }
  }
  return out;
}

function knownList() {
  try {
    return new Set(
      readFileSync(KNOWN, 'utf8')
        .split('\n')
        .map((l) => l.replace(/#.*$/, '').trim())
        .filter((l) => l !== ''),
    );
  } catch {
    return new Set();
  }
}

const specs = arities();
const calls = measured();
const missing = [];
for (const [name, { min, max }] of [...specs].sort()) {
  if (max === min) continue;
  const seen = calls.get(name) ?? new Set();
  const gaps = [];
  for (let n = min; n <= max; n++) if (!seen.has(n)) gaps.push(n);
  if (gaps.length > 0) missing.push(`${name}(${gaps.join(',')})`);
}

const check = process.argv.includes('--check');
const known = knownList();
const fresh = missing.filter((m) => !known.has(m));
const gone = [...known].filter((k) => !missing.includes(k));

if (check) {
  if (fresh.length > 0) {
    console.error(`${fresh.length} argument form(s) no golden measures, and not on the known list:`);
    for (const m of fresh) console.error(`  ${m}`);
    process.exit(1);
  }
  if (gone.length > 0) {
    console.error(`${gone.length} form(s) on the known list are measured now; take them off it:`);
    for (const m of gone) console.error(`  ${m}`);
    process.exit(1);
  }
  console.log(`Every argument form outside the known list is measured (${known.size} known).`);
} else {
  for (const m of missing) console.log(m);
  console.log(`\n${missing.length} unmeasured form(s) of ${[...specs].filter(([, s]) => s.max > s.min).length} functions that take an optional argument.`);
}
