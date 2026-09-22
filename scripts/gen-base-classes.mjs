// Turns what Visual FoxPro said about its base classes into the two tables the runtime reads.
//
// `tests/reference/vfp-base-classes.tsv` is the measurement (see `scripts/vfp-base-classes.mjs`).
// It is the product's own answer, in the product's own upper case, so this spells each name the
// way the language reference does and writes:
//
//   src/shared/registry/baseClassMembers.ts   what the object model builds an object from
//   crates/foxvm/src/base_classes.tsv         the same, for the VM's own test host
//
//   node scripts/gen-base-classes.mjs           # rewrites both
//   node scripts/gen-base-classes.mjs --check    # writes nothing, fails when they are stale
//
// A property whose value is not a constant - `Parent`, `Application`, a container's `Controls`,
// a window handle - is written down as a member with no default. Which properties those are is
// the product's answer too: they are the ones two fresh objects did not agree about, or that
// raised rather than answering.

import { readdirSync, readFileSync, writeFileSync } from 'node:fs';
import { join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const ROOT = resolve(import.meta.dirname, '..');
const MEASURED = join(ROOT, 'tests/reference/vfp-base-classes.tsv');
const LANGUAGE = join(ROOT, 'tests/reference/vfp-language.tsv');
const TS_OUT = join(ROOT, 'src/shared/registry/baseClassMembers.ts');
const RS_OUT = join(ROOT, 'crates/foxvm/src/base_classes.tsv');

/**
 * The errors the product raises when a write is refused because the property is read-only.
 *
 * 1743 and 2134 are FoxPro refusing one of its own. An object it hands over out of a type
 * library - the application, a project, a file of one - refuses through OLE instead, and says so
 * as 1429 ("This property is read-only") or 1426; the type library agrees, listing every one of
 * them for reading only.
 */
const READ_ONLY_ERRORS = new Set(['1743', '2134', '1426', '1429']);

/**
 * The name as the language reference spells it, by its upper case.
 *
 * The reference names a few properties two at a time ("BackColor, ForeColor Properties"), so
 * only one of the pair is a row of its own; the names our descriptors already write are read as
 * well to fill those in. What neither knows stays as the product wrote it, in upper case, which
 * at least does not invent a spelling.
 */
function spellings() {
  const out = new Map();
  for (const row of readFileSync(LANGUAGE, 'utf8').split(/\r?\n/).slice(1)) {
    const [kind, name] = row.split('\t');
    if (!kind || !name) continue;
    if (kind !== 'property' && kind !== 'event' && kind !== 'method') continue;
    if (!out.has(name.toUpperCase())) out.set(name.toUpperCase(), name);
  }
  const dir = join(ROOT, 'src/shared/registry');
  const sources = [join(dir, 'common.ts')].concat(
    readdirSync(join(dir, 'descriptors')).map((f) => join(dir, 'descriptors', f)),
  );
  for (const file of sources) {
    for (const call of readFileSync(file, 'utf8').matchAll(/\b(?:prop|enumProp|ev)\(\s*'([A-Za-z][A-Za-z0-9_]*)'/g)) {
      if (!out.has(call[1].toUpperCase())) out.set(call[1].toUpperCase(), call[1]);
    }
  }
  return out;
}

/** The measurement, class by class. */
function measured() {
  const classes = new Map();
  for (const line of readFileSync(MEASURED, 'latin1').split(/\r?\n/)) {
    if (line.startsWith('#') || line.trim() === '' || line.startsWith('class\t')) continue;
    const [cls, kind, name, type, value = '', onwrite = ''] = line.split('\t');
    if (!classes.has(cls)) classes.set(cls, { properties: [], computed: [], readOnly: [], events: [], methods: [], contained: new Map() });
    const entry = classes.get(cls);
    // the "class" row only says the class exists, which matters for one whose answer is that it
    // has no members at all
    if (kind === 'class') continue;
    // 1743 and 2134 are the product refusing a write because the property is read-only; any
    // other answer is about the state that object happened to be in, not about the property
    // what the property holds once the object is in a container, which is the answer a form is
    // drawn from: every control on a form is contained, and the colours a control takes from the
    // system's 3D palette are resolved against the container it is in
    if (kind === 'contained') {
      entry.contained.set(name, type === 'C' ? value : type === 'N' ? Number(value) : type === 'L' ? value === '.T.' : null);
      continue;
    }
    if (kind === 'property' && READ_ONLY_ERRORS.has(onwrite)) entry.readOnly.push([name, Number(onwrite)]);
    if (kind === 'event') entry.events.push(name);
    else if (kind === 'method') entry.methods.push(name);
    else if (type === 'C') entry.properties.push([name, value]);
    else if (type === 'N') entry.properties.push([name, Number(value)]);
    else if (type === 'L') entry.properties.push([name, value === '.T.']);
    else if (type === 'X') entry.properties.push([name, null]);
    else entry.computed.push(name);
  }
  return classes;
}

function literal(value) {
  if (value === null) return 'null';
  if (typeof value === 'string') return JSON.stringify(value);
  return String(value);
}

function typescript(classes, spell) {
  const say = (n) => spell.get(n.toUpperCase()) ?? n;
  const lines = [
    '/**',
    ' * Every Visual FoxPro base class, as the product itself answers about it: which properties an',
    ' * object of it holds and what they start out at, and which events and methods it answers to.',
    ' *',
    ' * Generated by `node scripts/gen-base-classes.mjs` from `tests/reference/vfp-base-classes.tsv`,',
    ' * which `scripts/vfp-base-classes.mjs` measures out of vfp9.exe itself. Do not edit by hand:',
    ' * a property this file does not list is one the product does not have.',
    ' *',
    ' * `computed` names the properties an object answers to whose value is not a constant - its',
    ' * `Parent`, the application it belongs to, a container\'s `Controls`, a window handle. The',
    ' * object model works those out as they are asked for; the descriptor beside this file is what',
    ' * says how the designer shows them.',
    ' */',
    '',
    "import type { PropValue } from '../form/schema';",
    '',
    'export interface BaseClassMembers {',
    '  /** Property name as the reference spells it, and what a new object of the class holds. */',
    '  properties: Record<string, PropValue>;',
    '  /** Properties the object answers to whose value is worked out rather than stored. */',
    '  computed: string[];',
    '  /** Properties the product refuses to have written to, and the error it raises for each. */',
    '  readOnly: Record<string, number>;',
    '  /**',
    '   * What a property holds once the object is in a container, where that is not what it holds',
    '   * on its own. A Label answers white until it is placed and 15790320 once it is: the colours',
    '   * a control takes from the system' + "'" + 's 3-D palette are resolved against the container',
    '   * it is in. Every control on every form is contained, so this is the answer a form is drawn',
    '   * from, where `properties` is what a bare CREATEOBJECT gets.',
    '   */',
    '  contained: Record<string, PropValue>;',
    '  events: string[];',
    '  methods: string[];',
    '}',
    '',
    'export const BASE_CLASS_MEMBERS: Record<string, BaseClassMembers> = {',
  ];
  for (const [cls, m] of classes) {
    lines.push(`  ${cls}: {`);
    lines.push('    properties: {');
    for (const [name, value] of m.properties) lines.push(`      ${say(name)}: ${literal(value)},`);
    lines.push('    },');
    lines.push(`    computed: [${m.computed.map((n) => `'${say(n)}'`).join(', ')}],`);
    lines.push(`    readOnly: { ${m.readOnly.map(([n, code]) => `${say(n)}: ${code}`).join(', ')} },`);
    lines.push(`    contained: { ${[...m.contained].map(([n, v]) => `${say(n)}: ${literal(v)}`).join(', ')} },`);
    lines.push(`    events: [${m.events.map((n) => `'${say(n)}'`).join(', ')}],`);
    lines.push(`    methods: [${m.methods.map((n) => `'${say(n)}'`).join(', ')}],`);
    lines.push('  },');
  }
  lines.push('};', '');
  return lines.join('\n');
}

function tsv(classes, spell) {
  const say = (n) => spell.get(n.toUpperCase()) ?? n;
  const rows = [
    '# Every Visual FoxPro base class, as the product itself answers about it: what an object of',
    '# it holds, and what it answers to. Read by crates/foxvm/src/base_classes.rs.',
    '#',
    '# Generated by `node scripts/gen-base-classes.mjs`; the measurement it comes from is',
    '# tests/reference/vfp-base-classes.tsv. A property with type "-" is one whose value is not a',
    '# constant, so it is a member with nothing stored behind it, and readonly is the error the',
    '# product raises for one it refuses to have written to.',
    'class\tkind\tname\ttype\tdefault\treadonly',
  ];
  for (const [cls, m] of classes) {
    const readOnly = new Map(m.readOnly);
    const refuses = (name) => String(readOnly.get(name) ?? '');
    rows.push(`${cls}\tclass\t\t\t\t`);
    for (const [name, value] of m.properties) {
      const type = value === null ? 'X' : typeof value === 'string' ? 'C' : typeof value === 'number' ? 'N' : 'L';
      const text = value === null ? '' : typeof value === 'boolean' ? (value ? '.T.' : '.F.') : String(value);
      rows.push(`${cls}\tproperty\t${say(name)}\t${type}\t${text}\t${refuses(name)}`);
    }
    for (const name of m.computed) rows.push(`${cls}\tproperty\t${say(name)}\t-\t\t${refuses(name)}`);
    for (const name of m.events) rows.push(`${cls}\tevent\t${say(name)}\t\t\t`);
    for (const name of m.methods) rows.push(`${cls}\tmethod\t${say(name)}\t\t\t`);
  }
  return rows.join('\n') + '\n';
}

/** What the two generated files should hold right now. */
export function generate() {
  const spell = spellings();
  const classes = measured();
  return { [TS_OUT]: typescript(classes, spell), [RS_OUT]: tsv(classes, spell) };
}

function main() {
  const check = process.argv.includes('--check');
  let stale = 0;
  for (const [path, want] of Object.entries(generate())) {
    const had = (() => {
      try {
        return readFileSync(path, 'utf8').replace(/\r\n/g, '\n');
      } catch {
        return null;
      }
    })();
    if (had === want) {
      console.log(`${path}: current`);
      continue;
    }
    stale += 1;
    if (check) console.log(`${path}: STALE`);
    else {
      writeFileSync(path, want);
      console.log(`${path}: written`);
    }
  }
  if (check && stale > 0) process.exit(1);
}

// run when it is the program, stay quiet when the test imports `generate` to check the files
if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) main();
