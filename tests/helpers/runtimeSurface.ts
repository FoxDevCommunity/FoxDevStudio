/**
 * What the runtime says it supports, read from the sources that decide it.
 *
 * The built-ins live in a table in the Rust crate and the command verbs in the parser's keyword
 * lists, so both are read from there rather than restated here: a list that has to be kept in
 * step by hand is a list that will not be.
 */

import { readdirSync, readFileSync } from 'node:fs';

/**
 * A source file with its line endings settled. What this reads out of the runtime is matched
 * with patterns anchored on newlines, and a CRLF checkout - which is what core.autocrlf gives
 * on Windows - would make them find nothing at all rather than fail.
 */
function sourceText(path: string): string {
  return readFileSync(path, 'utf8').replace(/\r\n/g, '\n');
}

const BUILTINS_DIR = 'crates/foxvm/src/builtins';
const PARSER = 'crates/foxvm/src/parser.rs';

export interface BuiltinEntry {
  name: string;
  min: number;
  max: number;
  /** Why it is unavailable, or null when it works. */
  unavailable: string | null;
}

/** Reads every `spec("NAME", min, max, f_fn)` and every `not_available!` message. */
export function readBuiltins(): BuiltinEntry[] {
  const sources = readdirSync(BUILTINS_DIR)
    .filter((f) => f.endsWith('.rs'))
    .map((f) => sourceText(`${BUILTINS_DIR}/${f}`))
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

/** How the parser treats a command verb. */
export type VerbKind = 'statement' | 'unsupported' | 'screen' | 'ide' | 'ignored';

/** The verb lists the parser dispatches on, by the name of the list in `parser.rs`. */
export function readVerbs(): Map<string, VerbKind> {
  const source = sourceText(PARSER);
  const lists: [string, VerbKind][] = [
    ['STATEMENT_KEYWORDS', 'statement'],
    ['UNSUPPORTED', 'unsupported'],
    ['SCREEN_ONLY', 'screen'],
    ['IDE_ONLY', 'ide'],
    ['IGNORED', 'ignored'],
  ];
  const out = new Map<string, VerbKind>();
  for (const [list, kind] of lists) {
    const block = new RegExp(`const ${list}: &\\[&str\\] = &\\[([\\s\\S]*?)\\];`).exec(source);
    if (!block) throw new Error(`${PARSER}: no ${list} list`);
    for (const word of (block[1] ?? '').matchAll(/"([^"]+)"/g)) {
      // a verb in more than one list is dispatched by the first that matches
      if (!out.has(word[1]!)) out.set(word[1]!, kind);
    }
  }
  return out;
}

/**
 * Verbs the parser handles inside another statement rather than from its own keyword list:
 * the second word of a two-word command, and the block terminators.
 */
export function readInnerVerbs(): Set<string> {
  const source = sourceText(PARSER);
  const out = new Set<string>();
  for (const kw of source.matchAll(/(?:eat_kw|is_kw)\("([A-Z][A-Z0-9 ]*)"\)/g)) out.add(kw[1]!);
  return out;
}
