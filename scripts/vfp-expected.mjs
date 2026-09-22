// Asks Visual FoxPro 9 what a golden program prints, and writes the `.expected` beside it.
//
// A golden program is only ground truth if Visual FoxPro produced its expected output. Written
// by hand it is ground truth about what someone believed, which is a different thing: three of
// the bugs found in this codebase were places where the reference page and the product disagree.
//
//   node scripts/vfp-expected.mjs crates/foxvm/tests/programs/ref_numbers.prg
//   node scripts/vfp-expected.mjs --check crates/foxvm/tests/programs/*.prg
//   node scripts/vfp-expected.mjs --all
//   node scripts/vfp-expected.mjs --probe scripts/probes/reports/p0-behaviour.prg
//
// `--probe` runs one program and prints what it printed together with every `fdv-*.txt` file it
// wrote, and compares nothing: a probe is a question put to the product whose answer is written
// into a specification (`docs/reports.md`), not a golden the runtime is expected to reproduce.
// A probe is given longer than a golden, because one probe runs every shipped report.
//
// `--check` writes no `.expected` and reports which files disagree with the product, which is
// what `npm run goldens:check` runs before anyone trusts a file that was written by hand. A
// full sweep - with or without `--check` - also rewrites `not-measured.txt`, because only a
// sweep has seen every golden.
//
// How it works: the program is copied to a scratch directory as `main.prg`, so `PROGRAM()` and
// `LINENO()` read exactly as they do in the test, and a wrapper runs it with `SET ALTERNATE`
// pointed at a file - VFP's own way of capturing what `?` prints. An unhandled error is written
// in the same words the golden runner expects: `ERROR <code> line <n> in <program>`.
//
// Beside the program the wrapper sets the same stage the golden runner does - the HelloWorld
// object tree in `oForm`, `other.prg` to `DO`, the report file - so a golden that leans on the
// mock host is asked a fair question.
//
// Some goldens stop to ask a person. Visual FoxPro would sit there for ever, so a Timer inside
// the wrapper watches: it flushes the capture, answers what is on screen the way the mock host
// does (as a cancel) and, when even that stops moving the program along, ends the run. Those
// programs go into `not-measured.txt` with the reason, because their `.expected` is still only
// what we believe.
//
// Visual FoxPro must be installed. Nothing here runs in CI; it is a tool for making the files
// that CI then checks.

import { execFile, execFileSync } from 'node:child_process';
import {
  existsSync,
  mkdirSync,
  mkdtempSync,
  readdirSync,
  readFileSync,
  rmSync,
  writeFileSync,
} from 'node:fs';
import { basename, dirname, join, resolve } from 'node:path';
import { tmpdir } from 'node:os';
import { fileURLToPath } from 'node:url';
// imported rather than taken as globals, because the lint config gives these scripts only the
// globals a build script needs
import { clearTimeout, setTimeout } from 'node:timers';

const VFP = 'C:/Program Files (x86)/Microsoft Visual FoxPro 9/vfp9.exe';
/** How long to let one program run before giving up on it and killing the process. */
const TIMEOUT_MS = 25_000;
/**
 * How long a probe gets. One of them runs every shipped report through a listener and then
 * dozens of small reports of its own, which is minutes rather than seconds, and the last thing
 * each probe tries is the thing that stops Visual FoxPro for good.
 */
const PROBE_TIMEOUT_MS = 300_000;
const ROOT = resolve(import.meta.dirname, '..');
const PROGRAMS = join(ROOT, 'crates/foxvm/tests/programs');
/** The files the golden runner hands the mock host, kept where both it and this script read them. */
const FIXTURES = join(ROOT, 'crates/foxvm/tests/fixtures/golden');
const MANIFEST = join(PROGRAMS, 'not-measured.txt');

/**
 * The wrapper that runs the program and writes down what it printed.
 *
 * `SET ALTERNATE` is how Visual FoxPro sends `?` output to a file, so the capture is the
 * product's own and not a transcription. The error handler writes the golden runner's own
 * wording, then stops: a golden asserts the first error, as the runner does.
 *
 * `ON ERROR` rather than a `TRY` around `DO main`, because a `TRY` in the caller takes
 * precedence over an `ON ERROR` the program sets for itself - measured - and a golden about
 * `ON ERROR` would then never see its own handler run.
 */
const WRAPPER = [
  'ON ERROR DO fdvErr WITH ERROR(), LINENO(), PROGRAM()',
  'SET SAFETY OFF',
  'SET TALK OFF',
  'SET CONSOLE OFF',
  'SET ESCAPE OFF',
  '* With AutoYield off Visual FoxPro fires a timer only while it is waiting for a person, so',
  '* the keeper below never interrupts a program that is merely busy.',
  '_VFP.AutoYield = .F.',
  'PUBLIC oForm, goFdvKeeper',
  'DO fdvStage',
  'SET ALTERNATE TO "out.txt"',
  'SET ALTERNATE ON',
  'goFdvKeeper = CREATEOBJECT("fdvKeeper")',
  'DO main',
  'DO fdvEnd WITH "done"',
  'QUIT',
  '',
  'PROCEDURE fdvErr',
  'LPARAMETERS nCode, nLine, cProg',
  'DO fdvEnd WITH "error"',
  'STRTOFILE("ERROR " + LTRIM(STR(nCode)) + " line " + LTRIM(STR(nLine)) + " in " + ALLTRIM(cProg) + CHR(13) + CHR(10), "err.txt")',
  'QUIT',
  '',
  '* Closes the capture, which is what writes the buffer out, and says how the run ended. No',
  '* how.txt at all means Visual FoxPro was killed before it got this far. Nothing in here may',
  '* read a variable: a program is free to CLEAR ALL, and the ending still has to be written.',
  'PROCEDURE fdvEnd',
  'LPARAMETERS cHow',
  'SET ALTERNATE OFF',
  'SET ALTERNATE TO',
  'STRTOFILE(cHow + CHR(13) + CHR(10), "how.txt")',
  'ENDPROC',
  '',
  '* The stage the golden runner sets before it runs a program: the same object tree in oForm.',
  '* The second program to DO and the report file sit on disk beside main.prg.',
  'PROCEDURE fdvStage',
  'oForm = CREATEOBJECT("Form")',
  'oForm.Caption = "Hello, World"',
  'oForm.AddObject("lblName", "Label")',
  'oForm.lblName.Caption = "Your name:"',
  'oForm.AddObject("txtName", "TextBox")',
  'oForm.txtName.Value = ""',
  'oForm.AddObject("chkLoud", "CheckBox")',
  'oForm.chkLoud.Value = .F.',
  'oForm.AddObject("pgfMain", "PageFrame")',
  'oForm.pgfMain.PageCount = 2',
  'oForm.pgfMain.Page1.Caption = "Greeting"',
  'oForm.pgfMain.Page1.AddObject("lblGreeting", "Label")',
  'oForm.pgfMain.Page1.lblGreeting.Caption = "(nothing yet)"',
  'oForm.pgfMain.Page2.Caption = "About"',
  'oForm.AddObject("cmdSayHi", "CommandButton")',
  'oForm.cmdSayHi.Caption = "Say Hi"',
  'oForm.AddObject("cmdClose", "CommandButton")',
  'oForm.cmdClose.Caption = "Close"',
  'ENDPROC',
  '',
  '* Closing and reopening the capture writes out what has been printed so far, so a run that',
  '* has to be ended still shows how far it got. A tick also means the program has stopped to',
  '* ask something, so the keeper answers as a cancel - what the mock host answers a dialog',
  '* with - and gives up when several answers in a row leave the program printing nothing new.',
  'DEFINE CLASS fdvKeeper AS Timer',
  '  Interval = 400',
  '  nSeen = 0',
  '  nQuiet = 0',
  '  PROCEDURE Timer',
  '    LOCAL nNow',
  '    SET ALTERNATE OFF',
  '    SET ALTERNATE TO',
  '    nNow = IIF(FILE("out.txt"), FSIZE("out.txt"), 0)',
  '    SET ALTERNATE TO "out.txt" ADDITIVE',
  '    SET ALTERNATE ON',
  '    IF nNow > THIS.nSeen',
  '      THIS.nSeen = nNow',
  '      THIS.nQuiet = 0',
  '    ELSE',
  '      THIS.nQuiet = THIS.nQuiet + 1',
  '    ENDIF',
  '    STRTOFILE("asked" + CHR(13) + CHR(10), "asked.txt")',
  '    KEYBOARD "{ESC}"',
  '    IF THIS.nQuiet > 10',
  '      DO fdvEnd WITH "stalled"',
  '      QUIT',
  '    ENDIF',
  '  ENDPROC',
  'ENDDEFINE',
  '',
].join('\r\n');

/** A scratch directory: the wrapper above it, the program and its fixtures in `run` inside it. */
function scratch() {
  const dir = mkdtempSync(join(tmpdir(), 'fdv-expected-'));
  mkdirSync(join(dir, 'run'));
  return dir;
}

/**
 * Copies the fixture files the golden runner gives the mock host into the run directory.
 *
 * They are checked in beside the tests so that the two describe the same stage; `cargo test`
 * fails when the report file drifts from the one `dbf_fixture::sample()` builds.
 */
function stage(dir) {
  if (!existsSync(FIXTURES)) return;
  for (const name of readdirSync(FIXTURES))
    writeFileSync(join(dir, name), readFileSync(join(FIXTURES, name)));
}

/**
 * Runs one program under Visual FoxPro and answers with what it printed and how it ended.
 *
 * `extras` holds every `fdv-*.txt` file the program wrote beside itself, by name. A probe writes
 * its measurements there with `STRTOFILE`, which lands on disk at once, where the `?` capture is
 * buffered and written out only when the run ends or the keeper flushes it: a run that has to be
 * killed still hands back everything measured before the statement that hung.
 */
export async function askVfp(source, { timeoutMs = TIMEOUT_MS } = {}) {
  const dir = scratch();
  const run = join(dir, 'run');
  try {
    writeFileSync(join(run, 'main.prg'), source, 'latin1');
    stage(run);
    writeFileSync(join(dir, 'probe.prg'), WRAPPER);
    await new Promise((done) => {
      const child = execFile(VFP, [join(dir, 'probe.prg')], { cwd: run }, () => done());
      const timer = setTimeout(() => {
        try {
          child.kill();
        } catch {
          // it may already be gone; the taskkill below is what actually ends a modal one
        }
        try {
          execFileSync('taskkill', ['/IM', 'vfp9.exe', '/F'], { stdio: 'ignore' });
        } catch {
          // no VFP left to kill, which is the good case
        }
        done();
      }, timeoutMs);
      child.on('exit', () => {
        clearTimeout(timer);
        done();
      });
    });
    const slurp = (name) => (existsSync(join(run, name)) ? readFileSync(join(run, name), 'latin1') : null);
    // closing an alternate file puts the old DOS end-of-file mark on the end of what it wrote
    const out = (slurp('out.txt') ?? '').split('\x1a')[0];
    const err = slurp('err.txt') ?? '';
    const how = slurp('how.txt');
    const extras = new Map();
    for (const name of readdirSync(run).sort()) {
      if (/^fdv-.*\.txt$/i.test(name)) extras.set(name, slurp(name));
    }
    return {
      text: normalize(out + err, true),
      how: how === null ? 'killed' : how.trim(),
      asked: existsSync(join(run, 'asked.txt')),
      extras,
      // Visual FoxPro answers with full paths where the mock host answers with bare names, so a
      // capture can name the scratch directory - which is a different one every run
      where: basename(dir),
    };
  } finally {
    // A print job sent through a printer driver (a probe under REPORTBEHAVIOR 90 does that) can
    // leave the spooler holding a file in the scratch directory for a moment after Visual
    // FoxPro has exited, and Windows answers the removal with EPERM. The answer has been read
    // by now, so a directory that cannot be removed is left behind and said so, not thrown.
    try {
      rmSync(dir, { recursive: true, force: true, maxRetries: 10, retryDelay: 300 });
    } catch (error) {
      console.error(`could not remove ${dir}: ${error.message}`);
    }
  }
}

/**
 * What the golden runner compares: one line per `?`, trailing blanks ignored.
 *
 * `?` ends the line that is open and then prints, so a captured file begins with the newline
 * the first `?` wrote rather than with its text. Dropping exactly one leading empty line
 * recovers the list of printed lines, and it does so even when the first thing printed was
 * itself empty - `? ""` writes a newline and nothing, which is one blank line, not two.
 */
function normalize(text, captured = false) {
  const lines = text.split(/\r\n|\r|\n/).map((x) => x.trimEnd());
  if (captured && lines[0] === '') lines.shift();
  while (lines.length > 0 && lines[lines.length - 1] === '') lines.pop();
  return lines.join('\n');
}

/**
 * Why a run cannot be compared with the product, or the empty string when it can.
 *
 * The rule is the run's own, not a list of names: a program Visual FoxPro had to be prodded
 * through was answered by a timer rather than by a person, so what it printed is not what the
 * golden runner would see.
 */
function whyNotMeasured({ how, asked, where, text }) {
  if (how === 'killed') return 'Visual FoxPro never finished and had to be killed';
  if (how === 'stalled') return 'Visual FoxPro stopped to ask a person, and answering did not move it on';
  if (asked) return 'Visual FoxPro stopped to ask a person, which the harness answered as a cancel';
  if (text.toUpperCase().includes(where.toUpperCase())) {
    return 'what it printed names the directory it ran in, which is a different one every run';
  }
  return '';
}

/** The manifest of goldens the product cannot be asked about, as `name: reason`. */
function readManifest() {
  const entries = new Map();
  if (!existsSync(MANIFEST)) return entries;
  for (const line of readFileSync(MANIFEST, 'latin1').split(/\r?\n/)) {
    if (line.trim() === '' || line.startsWith('#')) continue;
    const at = line.indexOf(':');
    entries.set(line.slice(0, at).trim(), line.slice(at + 1).trim());
  }
  return entries;
}

const MANIFEST_HEAD = [
  '# The goldens Visual FoxPro cannot be asked about, and why. Written by',
  '# `node scripts/vfp-expected.mjs --all`; read by crates/foxvm/tests/golden.rs.',
  '#',
  '# Their .expected files say what we believe the product does, not what it was seen to do.',
  '',
];

/**
 * The notes written under the list by hand: language elements no golden can exercise, and why.
 *
 * A sweep rewrites this file, so anything written into it that is not a golden would be lost.
 * These lines are read back and put down again, which is what lets the file hold the reasons an
 * element cannot be reached as well as the reasons a golden cannot be measured.
 */
function manifestNotes() {
  if (!existsSync(MANIFEST)) return [];
  const lines = readFileSync(MANIFEST, 'latin1').split(/\r?\n/);
  const head = new Set(MANIFEST_HEAD);
  return lines.filter((line) => line.startsWith('#') && !head.has(line));
}

function writeManifest(entries) {
  const notes = manifestNotes();
  const body = [...entries.entries()]
    .sort(([a], [b]) => a.localeCompare(b))
    .map(([name, why]) => `${name}: ${why}`);
  const tail = notes.length === 0 ? [] : [''].concat(notes);
  writeFileSync(MANIFEST, MANIFEST_HEAD.concat(body, tail, '').join('\n'));
}

/**
 * Runs one probe and prints everything it measured: how the run ended, whether it stopped to
 * ask a person, the `?` capture, then each `fdv-*.txt` file under its own name.
 */
async function probe(file) {
  const answer = await askVfp(readFileSync(resolve(file), 'latin1'), { timeoutMs: PROBE_TIMEOUT_MS });
  console.log(`== ${basename(file)}: ${answer.how}${answer.asked ? ', asked a person' : ''}`);
  console.log(answer.text);
  for (const [name, text] of answer.extras) {
    console.log(`\n== ${name}`);
    console.log(normalize(text));
  }
}

async function main() {
  const args = process.argv.slice(2);
  const check = args.includes('--check');
  const all = args.includes('--all');
  const probeAt = args.indexOf('--probe');
  if (probeAt >= 0) {
    if (!existsSync(VFP)) {
      console.error(`Visual FoxPro 9 is not installed at ${VFP}; nothing to ask.`);
      process.exit(2);
    }
    const file = args[probeAt + 1];
    if (!file) {
      console.error('Usage: node scripts/vfp-expected.mjs --probe <file.prg>');
      process.exit(2);
    }
    await probe(file);
    return;
  }
  const files = all
    ? readdirSync(PROGRAMS)
        .filter((f) => f.endsWith('.prg'))
        .sort()
        .map((f) => join(PROGRAMS, f))
    : args.filter((a) => !a.startsWith('--'));

  if (!existsSync(VFP)) {
    console.error(`Visual FoxPro 9 is not installed at ${VFP}; nothing to ask.`);
    process.exit(2);
  }
  if (files.length === 0) {
    console.error(
      'Usage: node scripts/vfp-expected.mjs [--check] [--all] <file.prg> ... | --probe <file.prg>',
    );
    process.exit(2);
  }

  const manifest = readManifest();
  let differed = 0;
  let unmeasured = 0;
  for (const file of files) {
    const path = resolve(file);
    const source = readFileSync(path, 'latin1');
    const answer = await askVfp(source);
    const name = basename(path);
    const why = whyNotMeasured(answer);
    if (why) {
      unmeasured += 1;
      manifest.set(name, why);
      console.log(`${name}: NOT MEASURED - ${why}`);
      continue;
    }
    manifest.delete(name);
    const got = answer.text;
    const expectedPath = path.replace(/\.prg$/, '.expected');
    const had = existsSync(expectedPath) ? normalize(readFileSync(expectedPath, 'latin1')) : null;
    if (had === got) {
      console.log(`${name}: agrees`);
      continue;
    }
    differed += 1;
    if (check) {
      // the first line that differs, because a whole-file dump hides a trailing space
      const a = (had ?? '').split('\n');
      const b = got.split('\n');
      const at = a.findIndex((l, i) => l !== b[i]);
      const where =
        at < 0
          ? `checked in has ${a.length} line(s), Visual FoxPro ${b.length}`
          : `line ${at + 1}\n  checked in ${JSON.stringify(a[at])}\n  VFP        ${JSON.stringify(b[at])}`;
      console.log(`${name}: DIFFERS at ${where}\n`);
    } else {
      mkdirSync(dirname(expectedPath), { recursive: true });
      writeFileSync(expectedPath, got === '' ? '' : got + '\n');
      console.log(`${name}: written from Visual FoxPro`);
    }
  }
  // only a full sweep knows about every golden, so only a full sweep may rewrite the manifest
  if (all) writeManifest(manifest);
  console.log(`\n${files.length} program(s): ${differed} differed, ${unmeasured} could not be measured.`);
  if (check && differed > 0) process.exit(1);
}

// The sweep runs only when this file is the program that was run: another script imports
// `askVfp` to put its own questions to the product, and must not start a sweep by doing so.
if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) await main();
