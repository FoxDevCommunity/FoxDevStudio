import { readdirSync, readFileSync, writeFileSync } from 'node:fs';
import { describe, expect, it } from 'vitest';
import { CONTROL_DESCRIPTORS, FORM_DESCRIPTOR, type ObjectDescriptor, type PropertyMeta } from '@shared/registry';
import { CONTROL_TYPES } from '@shared/form/schema';

/**
 * `docs/language-reference.md` is generated from the two registries, so it cannot drift from
 * what the runtime actually supports. Run `REGEN_DOCS=1 npx vitest run tests/shared/languageReference`
 * after adding a built-in or a property.
 */
const DOC = 'docs/language-reference.md';
const BUILTINS_DIR = 'crates/foxvm/src/builtins';

interface BuiltinEntry {
  name: string;
  min: number;
  max: number;
  /** Why it is unavailable, or null when it works. */
  unavailable: string | null;
}

/** Reads every `spec("NAME", min, max, f_fn)` and every `not_available!` message. */
function readBuiltins(): BuiltinEntry[] {
  const sources = readdirSync(BUILTINS_DIR)
    .filter((f) => f.endsWith('.rs'))
    .map((f) => readFileSync(`${BUILTINS_DIR}/${f}`, 'utf8'))
    .join('\n');

  // a not_available! arm reads `f_name => "NAME(): why ..."`, continued over lines with a backslash
  const unavailable = new Map<string, string>();
  for (const block of sources.matchAll(/not_available!\s*\{([\s\S]*?)\n\}/g)) {
    for (const arm of (block[1] ?? '').matchAll(/=>\s*"((?:[^"\\]|\\[\s\S])*)"/g)) {
      const message = (arm[1] ?? '').replace(/\\\s+/g, '').replace(/\s+/g, ' ').trim();
      const named = /^([A-Z0-9_]+)\(\):\s*(.*)$/.exec(message);
      if (named) unavailable.set(named[1]!, named[2]!);
    }
  }

  const entries: BuiltinEntry[] = [];
  for (const spec of sources.matchAll(/spec\("([A-Z0-9_]+)",\s*(\d+),\s*([A-Za-z0-9_]+),/g)) {
    const name = spec[1]!;
    const max = spec[3] === 'VARIADIC' ? Number.POSITIVE_INFINITY : Number(spec[3]);
    entries.push({ name, min: Number(spec[2]), max, unavailable: unavailable.get(name) ?? null });
  }
  return entries.sort((a, b) => a.name.localeCompare(b.name));
}

function args(entry: BuiltinEntry): string {
  if (entry.max === Number.POSITIVE_INFINITY) return `${entry.min}+`;
  return entry.min === entry.max ? String(entry.min) : `${entry.min}-${entry.max}`;
}

function defaultText(meta: PropertyMeta): string {
  if (meta.default === null) return '.NULL.';
  if (typeof meta.default === 'boolean') return meta.default ? '.T.' : '.F.';
  if (typeof meta.default === 'string') return meta.default === '' ? '(empty)' : `"${meta.default}"`;
  return String(meta.default);
}

function describeObject(title: string, descriptor: ObjectDescriptor): string {
  const rows = [...descriptor.properties]
    .sort((a, b) => a.name.localeCompare(b.name))
    .map((p) => `| ${p.name} | ${p.editor} | ${defaultText(p)} |`)
    .join('\n');
  const events = [...descriptor.events].map((e) => (e.params ? `${e.name}(${e.params})` : e.name)).join(', ');
  return [
    `### ${title}`,
    '',
    `Base class \`${descriptor.baseClass}\`. ${descriptor.properties.length} properties, ${descriptor.events.length} events.`,
    '',
    '| Property | Editor | Default |',
    '|---|---|---|',
    rows,
    '',
    `**Events**: ${events}`,
    '',
  ].join('\n');
}

function generate(): string {
  const builtins = readBuiltins();
  const working = builtins.filter((b) => !b.unavailable);
  const missing = builtins.filter((b) => b.unavailable);

  const lines: string[] = [
    '# Language reference',
    '',
    '_Generated from the built-in registry and the control registry. Do not edit by hand; run_',
    '_`REGEN_DOCS=1 npx vitest run tests/shared/languageReference` after changing either._',
    '',
    'Statements the compiler accepts are listed in the README. This file covers the two things',
    'you would otherwise have to discover by trial: which functions exist, and which properties',
    'each control has.',
    '',
    `## Functions (${builtins.length})`,
    '',
    `${working.length} work. ${missing.length} are recognised but report why they cannot run, so a`,
    'call fails with an explanation rather than "procedure not found".',
    '',
    '### Available',
    '',
    '| Function | Arguments |',
    '|---|---|',
    ...working.map((b) => `| ${b.name} | ${args(b)} |`),
    '',
    '### Recognised but not available yet',
    '',
    '| Function | Why |',
    '|---|---|',
    ...missing.map((b) => `| ${b.name} | ${b.unavailable} |`),
    '',
    '## Control properties',
    '',
    'Defaults matter: documents store only values that differ from the default, so what is listed',
    'here is what a saved form leaves out.',
    '',
    describeObject('Form', FORM_DESCRIPTOR),
    ...[...CONTROL_TYPES].sort().map((type) => describeObject(type, CONTROL_DESCRIPTORS[type])),
  ];
  return `${lines.join('\n').replace(/\n{3,}/g, '\n\n')}\n`;
}

describe('language reference', () => {
  it('lists every built-in and control property, and stays in step with the code', () => {
    const generated = generate();
    if (process.env['REGEN_DOCS']) writeFileSync(DOC, generated);

    const onDisk = readFileSync(DOC, 'utf8').replace(/\r\n/g, '\n');
    expect(onDisk, `${DOC} is out of date; regenerate with REGEN_DOCS=1`).toBe(generated);
  });

  it('finds a sensible number of entries, so a broken parse cannot pass quietly', () => {
    const builtins = readBuiltins();
    expect(builtins.length).toBeGreaterThan(150);
    expect(builtins.filter((b) => b.unavailable).length).toBeGreaterThan(10);
    // a few we know by name
    expect(builtins.map((b) => b.name)).toEqual(expect.arrayContaining(['ALLTRIM', 'MESSAGEBOX', 'RGB', 'BINDEVENT', 'SQLEXEC']));
  });

  it('gives every unavailable function a reason, not just its name', () => {
    for (const entry of readBuiltins().filter((b) => b.unavailable)) {
      expect(entry.unavailable!.length, `${entry.name} needs a reason`).toBeGreaterThan(20);
    }
  });
});
