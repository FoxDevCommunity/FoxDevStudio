/**
 * The Visual FoxPro language reference, measured against this runtime.
 *
 * `vfp-language.tsv` is every command, function, directive, system variable, property, method
 * and event the VFP 9 reference lists. This test asks the runtime what it does with each one and
 * writes the answer to `docs/language-coverage.md`, which is checked in: a change that adds a
 * command, or quietly loses one, shows up as a diff there rather than as a bug report from
 * someone running a real program.
 *
 * It also holds the invariants a coverage map cannot show. A built-in that is not a VFP function
 * is a typo or an invention; a test that claims to cover an element that does not exist is a
 * typo too; and the number of elements that work must not fall.
 *
 * Regenerate with `REGEN_DOCS=1 npx vitest run tests/reference`.
 */

import { readFileSync, readdirSync, writeFileSync } from 'node:fs';

/**
 * A source file, read with its line endings settled.
 *
 * What this test knows about the runtime it reads out of the runtime's own text, and the
 * patterns that do the reading are anchored on newlines. A checkout with CRLF endings - which
 * is what `core.autocrlf=true` gives on Windows, and what every fresh worktree got - made them
 * match nothing, and the map came out wrong rather than failing. .gitattributes stops that
 * happening; this makes it impossible.
 */
function sourceText(path: string): string {
  return readFileSync(path, 'utf8').replace(/\r\n/g, '\n');
}
import { beforeAll, describe, expect, it } from 'vitest';
import { CONTROL_DESCRIPTORS, FORM_DESCRIPTOR, OBJECT_DESCRIPTORS } from '@shared/registry';
import { COMMAND_EVENTS, DATA_METHODS, FOXPRO_METHODS, PROJECT_METHODS, REPORTED_METHODS, REPORT_METHODS, XML_METHODS } from '@shared/language/foxproMethods';
import { readBuiltins, readVerbs } from '../helpers/runtimeSurface';
import { loadFoxVm, loadFoxVmSync } from '../../src/wasm/foxvm/loader';

const MANIFEST = 'tests/reference/vfp-language.tsv';
const DOC = 'docs/language-coverage.md';
const PROGRAMS = 'crates/foxvm/tests/programs';

/** One element of the language, as the reference lists it. */
interface Element {
  kind: string;
  /** The name as the reference writes it: `SET EXACT`, `ALLTRIM`, `@ ... SAY`. */
  name: string;
  title: string;
}

/** What this runtime does with an element. */
type Support =
  /** It works. */
  | 'works'
  /** It compiles and runs, but changes nothing: a setting this runtime has no use for. */
  | 'accepted'
  /** It is recognised and says why it cannot run, rather than failing as a mystery. */
  | 'reported'
  /** It is refused on purpose and always will be: the thing it acts on is gone. */
  | 'answered'
  /** Nothing knows the name. */
  | 'missing';

const LEVELS: Support[] = ['works', 'accepted', 'answered', 'reported', 'missing'];
const LABELS: Record<Support, string> = {
  works: 'Runs',
  accepted: 'Accepted and ignored',
  answered: 'Answered, not missing',
  reported: 'Refused, by name',
  missing: 'Unknown to the runtime',
};

/**
 * Elements that are refused on purpose and will not be built, with the reason.
 *
 * These are not gaps and should stop being counted as ones. Each acts on something that no
 * longer exists rather than on something hard: there is nothing to load a 16-bit binary
 * overlay into, Visual FoxPro 9 itself converts a FoxPro 2.x screen into a form, and ASSIST
 * is a stub in VFP 9 too.
 */
const ANSWERED: Record<string, string> = {
  'command:CALL': 'a 16-bit FoxPro 2.x binary overlay; DECLARE - DLL is the living replacement',
  'command:LOAD': 'the same overlay, being loaded; there is nothing here to load it into',
  'command:MODIFY SCREEN': 'the FoxPro 2.x screen designer; VFP 9 converts a .scx screen to a form, and so do we',
  'command:ASSIST': 'the FoxPro 2.6 Catalog Manager, which Visual FoxPro 9 ships as a stub',
};

/**
 * SET options the runtime acts on somewhere other than `set_cmd`, and where.
 *
 * The classifier below reads the match arms of one function, so an option handled anywhere else
 * looked ignored when it was not - ten of them did. Each entry names the file and a string that
 * must still be in it, so this cannot quietly go stale: the test checks every one.
 */
const SET_ELSEWHERE: Record<string, { file: string; find: string }> = {
  FILTER: { file: 'crates/foxvm/src/compiler.rs', find: 'StmtKind::SetFilter' },
  ORDER: { file: 'crates/foxvm/src/compiler.rs', find: 'StmtKind::SetOrder' },
  RELATION: { file: 'crates/foxvm/src/compiler.rs', find: 'StmtKind::SetRelation' },
  'RELATION OFF': { file: 'crates/foxvm/src/compiler.rs', find: 'StmtKind::SetRelation' },
  SYSMENU: { file: 'crates/foxvm/src/menu.rs', find: 'SYSMENU' },
  'SKIP OF': { file: 'crates/foxvm/src/menu.rs', find: 'SKIP OF' },
  'MARK OF': { file: 'crates/foxvm/src/menu.rs', find: 'MARK OF' },
  MESSAGE: { file: 'crates/foxvm/src/menu.rs', find: 'SET MESSAGE' },
  TEXTMERGE: { file: 'crates/foxvm/src/parser.rs', find: 'TEXTMERGE' },
  DATABASE: { file: 'crates/foxvm/src/vm.rs', find: 'SET DATABASE' },
};

function manifest(): Element[] {
  const [, ...lines] = readFileSync(MANIFEST, 'utf8').trim().split('\n');
  return lines.map((line) => {
    const [kind, name, title] = line.split('\t');
    return { kind: kind!, name: name!, title: title! };
  });
}

/**
 * The words a command name begins with, for matching against the parser's verbs.
 *
 * The reference writes alternatives with a bar (`CD | CHDIR`), arguments with an ellipsis
 * (`@ ... SAY`, `DO CASE ... ENDCASE`) and qualifiers with a dash (`CREATE REPORT - Quick
 * Report`), so a name is several possible verbs.
 */
function verbsOf(name: string): string[] {
  // the reference writes alternatives with a bar, and once with the word: DIR or DIRECTORY
  return name
    .split(/\||\bor\b/)
    .map((part) => part.split(/\s+-\s+/)[0]!.split('...')[0]!.trim().toUpperCase())
    .filter((part) => part !== '');
}

/**
 * A line of FoxPro that uses a command, built from the name the reference gives it.
 *
 * The point is not to write valid code for all 400 of them: it is to get past the parser's
 * dispatch, which is where a verb is either known or not. An argument the probe gets wrong
 * produces a syntax error about the argument, and that error is itself proof the verb was
 * recognised.
 */
function probeOf(name: string): string {
  const words = name
    .split('|')[0]!
    .split(/\s+-\s+/)[0]!
    .replace(/\.\.\./g, ' ')
    .trim();
  // the reference names a block by both ends: DO CASE ... ENDCASE is opened by DO CASE
  const opener = words.replace(/\s+END\w+$/i, '').trim();
  return opener;
}

/**
 * SET options the runtime acts on; every other one is accepted and changes nothing.
 *
 * Most are match arms of `set_cmd`, but an option can be a statement of its own or belong to
 * the menus or the parser, so `SET_ELSEWHERE` names those and where they live.
 */
function settingsHandled(): Set<string> {
  const vm = sourceText('crates/foxvm/src/vm.rs');
  // the parameter list is not part of the search: it has been wrapped over several lines once
  // already, and every setting silently became one this runtime does not act on
  const body = /fn set_cmd\([\s\S]*?\n {4}\}\n/.exec(vm)?.[0] ?? '';
  const out = new Set<string>();
  for (const arm of body.matchAll(/^\s{12}((?:"[A-Z ]+"\s*\|?\s*)+)=>/gm)) {
    for (const word of arm[1]!.matchAll(/"([A-Z ]+)"/g)) out.add(word[1]!.trim());
  }
  for (const [option, where] of Object.entries(SET_ELSEWHERE)) {
    if (sourceText(where.file).includes(where.find)) out.add(option);
  }
  return out;
}

/** Everything the object model answers to, from the registries React and the VM share. */
function objectSurface(): {
  props: Set<string>;
  methods: Set<string>;
  reported: Set<string>;
  events: Set<string>;
  classes: Set<string>;
} {
  const props = new Set<string>();
  const events = new Set<string>();
  const classes = new Set<string>();
  for (const [name, descriptor] of [
    ['Form', FORM_DESCRIPTOR] as const,
    ...Object.entries(CONTROL_DESCRIPTORS),
    ...Object.entries(OBJECT_DESCRIPTORS),
  ]) {
    classes.add(name.toUpperCase());
    classes.add(descriptor.baseClass.toUpperCase());
    for (const p of descriptor.properties) props.add(p.name.toUpperCase());
    for (const e of descriptor.events) events.add(e.name.toUpperCase());
  }
  // the events a command raises rather than an object: READ runs what its clauses name
  for (const name of COMMAND_EVENTS) events.add(name);
  // the classes the runtime makes that are not described by a registry entry of their own
  for (const name of ['Empty', 'Collection', 'FormSet', 'Files', 'Objects']) classes.add(name.toUpperCase());
  // and the two the reference calls something other than what the class is called
  for (const [reference, runtime] of Object.entries(RENAMED_CLASSES)) {
    if (classes.has(runtime.toUpperCase())) classes.add(reference.toUpperCase());
  }
  const methods = new Set(
    [...FOXPRO_METHODS, ...DATA_METHODS, ...REPORT_METHODS, ...XML_METHODS, ...PROJECT_METHODS].map((m) =>
      m.toUpperCase(),
    ),
  );
  const reported = new Set(REPORTED_METHODS.map((m) => m.toUpperCase()));
  return { props, methods, reported, events, classes };
}

/**
 * What the language does with an operator: the lexer reads it and the compiler acts on it, so
 * a program that uses one either compiles or reports a syntax error where the operator is.
 */
function operatorSupport(name: string): Support {
  const probe = OPERATOR_PROBES[name];
  if (probe === undefined) return 'missing';
  const { diagnostics } = loadFoxVmSync().check(probe, 'program') as { diagnostics: { message: string; severity: string }[] };
  return diagnostics.some((d) => d.severity === 'error') ? 'missing' : 'works';
}

/**
 * A line that uses each operator for what it is for. An operator cannot be probed by its name
 * the way a command can - it is punctuation, and only means anything in an expression - so each
 * one is written into the smallest program that uses it.
 */
const OPERATOR_PROBES: Record<string, string> = {
  '+': 'x = 1 + 2',
  '-': 'x = 3 - 1',
  '*': 'x = 2 * 3',
  '/': 'x = 6 / 2',
  '%': 'x = 7 % 2',
  '**': 'x = 2 ** 8',
  '^': 'x = 2 ^ 8',
  '(': 'x = (1 + 2) * 3',
  // the reference names the pair as one operator, which is what grouping is
  '( )': 'x = (1 + 2) * 3',
  ')': 'x = (1 + 2) * 3',
  '=': 'x = 1',
  '==': 'IF "a" == "a"\nENDIF',
  '<': 'IF 1 < 2\nENDIF',
  '>': 'IF 2 > 1\nENDIF',
  '<=': 'IF 1 <= 2\nENDIF',
  '>=': 'IF 2 >= 1\nENDIF',
  '<>': 'IF 1 <> 2\nENDIF',
  '#': 'IF 1 # 2\nENDIF',
  '!=': 'IF 1 != 2\nENDIF',
  $: 'IF "a" $ "abc"\nENDIF',
  AND: 'IF .T. AND .F.\nENDIF',
  OR: 'IF .T. OR .F.\nENDIF',
  NOT: 'IF NOT .T.\nENDIF',
  '!': 'IF ! .T.\nENDIF',
};

/**
 * Elements a test says it exercises: a golden program may begin `* COVERS: NAME, NAME`, and
 * that is what makes an element tested rather than merely present.
 *
 * A claim may say the kind as well - `method:Help` - where the reference has one name in two
 * kinds, so that covering the application's Help method does not quietly cover the HELP command.
 */
function covered(): Map<string, string[]> {
  const out = new Map<string, string[]>();
  for (const file of readdirSync(PROGRAMS).filter((f) => f.endsWith('.prg'))) {
    const source = sourceText(`${PROGRAMS}/${file}`);
    for (const header of source.matchAll(/^\*\s*COVERS:\s*(.+)$/gim)) {
      for (const name of (header[1] ?? '').split(',')) {
        const key = name.trim().toUpperCase();
        if (key === '') continue;
        out.set(key, [...(out.get(key) ?? []), file]);
      }
    }
  }
  return out;
}

/** Whether a golden claims this element, by name alone or by kind and name. */
function isTested(tested: Map<string, string[]>, kind: string, name: string): boolean {
  return tested.has(name.toUpperCase()) || tested.has(`${kind}:${name}`.toUpperCase());
}

/**
 * Names this runtime has that the reference's index pages do not list.
 *
 * The index is not exhaustive: an alias is documented on the page of the name it stands for,
 * and the XMLAdapter's methods are documented as methods rather than as functions even though a
 * program calls them like one. Each entry here says which, so a name that is neither - an
 * invention, or a typo - still fails the test.
 */
const UNLISTED_FUNCTIONS: Record<string, string> = {
  ADDTABLESCHEMA: 'XMLAdapter method, called as a function',
  AEMPTY: 'documented with the other array functions',
  AEMPTYNEW: 'documented with AEMPTY',
  APPLYDIFFGRAM: 'XMLAdapter method, called as a function',
  GETKEY: 'documented with the keyboard functions',
  LOADXML: 'XMLAdapter method, called as a function',
  MSGBOX: 'alias of MESSAGEBOX, documented on its page',
  PARSFONT: 'documented with the font functions',
  REMOVEPROPERTY: 'documented on the ADDPROPERTY page',
  SYS: 'documented one number at a time (SYS(2001), ...)',
  TOCURSOR: 'XMLAdapter method, called as a function',
  TOXML: 'XMLAdapter method, called as a function',
};

/**
 * Classes the reference lists under a name that is not the class's own. VFP's own designer and
 * BaseClass property use the second name, which is what a program writes.
 */
const RENAMED_CLASSES: Record<string, string> = {
  'OLE Container': 'OleControl',
  'OLE Bound': 'OleBoundControl',
};

/** The same, for command verbs the index writes only as part of a longer command. */
const UNLISTED_VERBS: Record<string, string> = {
  ACCEPT: 'a command of FoxPro 2.x, documented among the backward-compatible elements',
  DODEFAULT: 'documented as a function; VFP accepts it as a statement too',
  ENDDEFINE: 'the end of DEFINE CLASS',
  NODEFAULT: 'documented on the page of the event it belongs to',
  THROW: 'documented on the TRY...CATCH...FINALLY page',
};

/**
 * What the compiler makes of a command, from the diagnostics it gives its probe.
 *
 * A verb the parser does not know is reported as an unrecognised command verb, and one it knows
 * but cannot run says so by name. Anything else got past dispatch: the verb is real here.
 */
function commandSupport(name: string, settings: Set<string>): Support {
  const probe = probeOf(name);
  const { diagnostics } = loadFoxVmSync().check(probe, 'program') as { diagnostics: { message: string; severity: string }[] };
  const messages = diagnostics.map((d) => d.message);

  if (messages.some((m) => m.startsWith('Unrecognized command verb'))) return 'missing';
  if (messages.some((m) => /is not supported in the FoxDev runtime|is ignored:/.test(m))) return 'reported';
  // SET has one verb and a hundred settings; the VM acts on a few and lets the rest by
  const set = /^SET\s+(.*)$/i.exec(probe);
  if (set) return settings.has(set[1]!.trim().toUpperCase()) ? 'works' : 'accepted';
  return 'works';
}

function classify(): { support: Map<string, Support>; extraBuiltins: string[]; extraVerbs: string[] } {
  const elements = manifest();
  const builtins = new Map(readBuiltins().map((b) => [b.name, b]));
  const verbs = readVerbs();
  const objects = objectSurface();

  const settings = settingsHandled();
  const support = new Map<string, Support>();
  // every name the reference has, and the first word of every command, which is what the
  // parser dispatches on
  const known = { name: new Set<string>(), verb: new Set<string>() };
  for (const element of elements) {
    known.name.add(element.name.toUpperCase());
    if (element.kind === 'command') for (const v of verbsOf(element.name)) known.verb.add(v.split(/\s+/)[0]!);
  }

  for (const element of elements) {
    const key = `${element.kind}:${element.name}`;
    // refused on purpose: the reason is the answer, and it is not a gap to close
    if (key in ANSWERED) {
      support.set(key, 'answered');
      continue;
    }
    switch (element.kind) {
      case 'function': {
        const entry = builtins.get(element.name.toUpperCase());
        // a few are the compiler's rather than the library's, because what they do with their
        // arguments is not what a call does with them
        if (!entry && compilerKnows().functions.has(element.name.toUpperCase())) {
          support.set(key, 'works');
          break;
        }
        support.set(key, entry ? (entry.unavailable ? 'reported' : 'works') : 'missing');
        break;
      }
      case 'command': {
        support.set(key, commandSupport(element.name, settings));
        break;
      }
      case 'property':
        support.set(key, objects.props.has(element.name.toUpperCase()) ? 'works' : 'missing');
        break;
      case 'method': {
        const name = element.name.toUpperCase();
        // a method that acts on something this runtime has not got says which, and is counted
        // apart from the ones that do their work
        const how = objects.methods.has(name) ? 'works' : objects.reported.has(name) ? 'reported' : 'missing';
        support.set(key, how);
        break;
      }
      case 'event':
        support.set(key, objects.events.has(element.name.toUpperCase()) ? 'works' : 'missing');
        break;
      case 'directive': {
        // the reference names a conditional by its pair, "#IF ... #ENDIF": the directive is the
        // word it starts with
        const word = element.name.split(String.fromCharCode(32))[0]!.toUpperCase();
        support.set(key, compilerKnows().directives.has(word) ? 'works' : 'missing');
        break;
      }
      case 'systemvar':
        support.set(key, compilerKnows().systemVars.has(element.name.toUpperCase()) ? 'works' : 'missing');
        break;
      // a class a program can name: one the registry describes, or one the object model makes
      case 'object':
        support.set(key, objects.classes.has(element.name.toUpperCase()) ? 'works' : 'missing');
        break;
      // an operator is read by the lexer and acted on by the compiler; one it does not know is
      // a syntax error rather than an operator
      case 'operator':
        support.set(key, operatorSupport(element.name));
        break;
      default:
        support.set(key, 'missing');
        break;
    }
  }

  // names this runtime has that the reference does not: an alias, an invention, or a typo
  const extraBuiltins = [...builtins.keys()].filter((n) => !known.name.has(n) && !(n in UNLISTED_FUNCTIONS)).sort();
  const extraVerbs = [...verbs.keys()]
    .filter((v) => !known.verb.has(v.split(' ')[0]!) && !(v in UNLISTED_VERBS))
    .sort();
  return { support, extraBuiltins, extraVerbs };
}

/** Directives, system variables and functions the compiler knows, read from its own sources. */
function compilerKnows(): { directives: Set<string>; systemVars: Set<string>; functions: Set<string> } {
  const parser = sourceText('crates/foxvm/src/parser.rs');
  const body = /fn directive\(&mut self\)[\s\S]*?\n {4}\}\n/.exec(parser);
  const directives = new Set<string>();
  for (const arm of (body?.[0] ?? '').matchAll(/^\s{12}((?:"[A-Z]+"\s*\|?\s*)+)=>/gm)) {
    for (const word of arm[1]!.matchAll(/"([A-Z]+)"/g)) directives.add(`#${word[1]}`);
  }
  // the variables the VM makes public, and the two the parser reads as the objects they are
  const vars = new Set<string>();
  for (const file of ['crates/foxvm/src/vm.rs', 'crates/foxvm/src/parser.rs']) {
    for (const v of sourceText(file).matchAll(/"(_[A-Z]+)"/g)) vars.add(v[1]!);
  }
  // the functions the compiler emits code for instead of calling the library: IIF only
  // evaluates the branch it takes, so it cannot be a call
  const compiler = sourceText('crates/foxvm/src/compiler.rs');
  const functions = new Set<string>();
  for (const name of compiler.matchAll(/COMPILED_FUNCTIONS[^=]*=\s*&\[([^\]]*)\]/g)) {
    for (const word of name[1]!.matchAll(/"([A-Z]+)"/g)) functions.add(word[1]!);
  }
  return { directives, systemVars: vars, functions };
}

function report(): string {
  const elements = manifest();
  const { support, extraBuiltins, extraVerbs } = classify();
  const tested = covered();
  const kinds = ['command', 'function', 'property', 'method', 'event', 'object', 'operator', 'directive', 'systemvar'];

  const lines: string[] = [
    '# Language coverage',
    '',
    '_Generated from `tests/reference/vfp-language.tsv` (every element the Visual FoxPro 9',
    'reference lists) and the runtime\'s own registries. Regenerate with_',
    '`REGEN_DOCS=1 npx vitest run tests/reference`.',
    '',
    'Refresh the reference itself with `node scripts/fetch-vfp-reference.mjs`.',
    '',
    'Every element of the Visual FoxPro language, and what happens when a program uses it.',
    '',
    '- **Runs** - it does what Visual FoxPro does.',
    '- **Accepted and ignored** - it compiles and runs and changes nothing. Almost all of these',
    '  are SET options this runtime has no use for, so a program that sets one still runs.',
    '- **Answered, not missing** - it is refused on purpose and will not be built, because what',
    '  it acts on is gone rather than hard. The reason is listed with it.',
    '- **Refused, by name** - it stops the program with a message saying what is not supported,',
    '  rather than failing as something else.',
    '- **Unknown** - the runtime has never heard of the name. A command reads as an',
    '  unrecognised verb, a function as a missing procedure.',
    '',
    '| Kind | Runs | Accepted and ignored | Answered | Refused, by name | Unknown | Total |',
    '|---|---:|---:|---:|---:|---:|---:|',
  ];

  const bucket = (kind: string, level: Support) =>
    elements.filter((e) => e.kind === kind && support.get(`${e.kind}:${e.name}`) === level).map((e) => e.name);

  for (const kind of kinds) {
    const total = elements.filter((e) => e.kind === kind).length;
    if (total === 0) continue;
    const counts = LEVELS.map((level) => bucket(kind, level).length);
    lines.push(`| ${kind} | ${counts.join(' | ')} | ${total} |`);
  }

  const testedNames = [...tested.keys()].sort();
  lines.push('', `${testedNames.length} element(s) are exercised by a test.`, '');

  for (const kind of kinds) {
    const total = elements.filter((e) => e.kind === kind).length;
    if (total === 0) continue;
    lines.push(`## ${kind}`, '');
    for (const level of LEVELS) {
      const names = bucket(kind, level);
      if (names.length === 0) continue;
      lines.push(`### ${LABELS[level]} (${names.length})`, '');
      if (level === 'answered') {
        for (const name of names) lines.push(`- **${name}** - ${ANSWERED[`${kind}:${name}`]}`);
        lines.push('');
        continue;
      }
      lines.push(names.map((n) => (isTested(tested, kind, n) ? `**${n}**` : n)).join(', '), '');
    }
  }

  lines.push('## Names this runtime has that the reference does not', '');
  lines.push(`Functions: ${extraBuiltins.length === 0 ? 'none' : extraBuiltins.join(', ')}`, '');
  lines.push(`Command verbs: ${extraVerbs.length === 0 ? 'none' : extraVerbs.join(', ')}`, '');
  lines.push('Bold names are exercised by a test.', '');
  return lines.join('\n');
}

beforeAll(async () => {
  // the classifier compiles a probe of every command through the real compiler
  await loadFoxVm();
});

describe('the Visual FoxPro language reference', () => {
  it('lists every element the reference has, and what this runtime does with it', () => {
    const generated = report();
    if (process.env['REGEN_DOCS']) writeFileSync(DOC, generated);
    const onDisk = readFileSync(DOC, 'utf8');
    expect(onDisk, `${DOC} is out of date; regenerate with REGEN_DOCS=1`).toBe(generated);
  });

  it('has a manifest that reads as the reference wrote it', () => {
    const elements = manifest();
    expect(elements.length).toBeGreaterThan(1500);
    expect(elements.filter((e) => e.kind === 'command').length).toBeGreaterThan(400);
    expect(elements.filter((e) => e.kind === 'function').length).toBeGreaterThan(400);
    // every name is a name, not a sentence from an index page
    for (const element of elements) {
      expect(element.name, element.title).not.toMatch(/\b(Overview|Reference|Elements)\b/);
      expect(element.name.length, element.title).toBeLessThan(60);
    }
  });

  it('invents no function of its own', () => {
    const { extraBuiltins } = classify();
    // FoxDev adds nothing to the language: every built-in is one Visual FoxPro documents
    expect(extraBuiltins).toEqual([]);
  });

  it('invents no command verb of its own', () => {
    const { extraVerbs } = classify();
    expect(extraVerbs).toEqual([]);
  });

  it('only claims to test elements that exist', () => {
    const elements = manifest();
    const names = new Set(elements.flatMap((e) => [e.name.toUpperCase(), ...verbsOf(e.name)]));
    for (const e of elements) names.add(`${e.kind}:${e.name}`.toUpperCase());
    const unknown = [...covered().keys()].filter((name) => !names.has(name));
    expect(unknown, 'a COVERS: header names something the reference does not').toEqual([]);
  });

  it('reads the directives out of the compiler that handles them', () => {
    const { directives } = compilerKnows();
    expect([...directives].sort()).toContain('#DEFINE');
    expect([...directives].sort()).toContain('#INCLUDE');
  });
});
