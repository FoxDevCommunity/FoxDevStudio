// A second pair of eyes on the codebase, from a model with room to read it.
//
// Kimi Code holds 256K of context, which is enough to take in a whole crate at once - every
// caller of a function, every place a base class is touched, the shape of a system rather than
// the file in front of you. That is work this session cannot do cheaply: reading forty files to
// answer one question costs a context window we need for the work itself. So the division is
// deliberate - Kimi reads widely and answers in a page, and that page comes back here.
//
// It is an outside opinion, not an oracle. Nothing it says about Visual FoxPro is measured; the
// product is still the only authority on behaviour, and `scripts/vfp-expected.mjs` is still how
// we ask it. Use this for "where is X done", "what else would this break", "summarise how this
// system hangs together" - questions about our own code, where being wrong is visible.
//
//   node scripts/kimi.mjs "how does the scheduler avoid re-entering the wasm?"
//   node scripts/kimi.mjs --dir crates/foxvm "list every place Value::Ref is dereferenced"
//   node scripts/kimi.mjs --file docs/brief.md --out answer.md
//   node scripts/kimi.mjs --resume session_a3d8ee26-... "now show the error paths"
//   echo "..." | node scripts/kimi.mjs
//
// The working directory it reads is the repository root unless --dir says otherwise; narrowing
// it to one crate is usually both faster and a better answer. Every run prints the session id,
// and --resume continues that conversation with its context intact, which is much cheaper than
// asking the same wide question twice.
//
// Requires Kimi Code (https://code.kimi.com) and `kimi login`. Set KIMI_BIN to override where
// this looks for it.

import { spawn } from 'node:child_process';
import { existsSync, readFileSync, writeFileSync } from 'node:fs';
import { homedir } from 'node:os';
import { join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

/** Where Kimi Code puts itself, in the order worth trying. KIMI_BIN wins over all of them. */
function kimiBinary() {
  const candidates = process.env.KIMI_BIN
    ? [process.env.KIMI_BIN]
    : [
        join(homedir(), '.kimi-code', 'bin', process.platform === 'win32' ? 'kimi.exe' : 'kimi'),
        join(homedir(), '.local', 'bin', process.platform === 'win32' ? 'kimi.exe' : 'kimi'),
      ];
  const found = candidates.find((path) => existsSync(path));
  if (found) return found;
  // Not where we expect it, but it may still be on PATH; let spawn decide.
  return 'kimi';
}

/** The command line, in the shape this script takes it. */
function parseArgs(argv) {
  const opts = { dir: process.cwd(), words: [] };
  for (let i = 0; i < argv.length; i += 1) {
    const arg = argv[i];
    if (arg === '--dir') opts.dir = resolve(argv[(i += 1)]);
    else if (arg === '--file') opts.file = argv[(i += 1)];
    else if (arg === '--out') opts.out = argv[(i += 1)];
    else if (arg === '--resume') opts.resume = argv[(i += 1)];
    else if (arg === '--timeout') opts.timeoutMs = Number(argv[(i += 1)]) * 1000;
    else if (arg === '--json') opts.json = true;
    else if (arg === '--help' || arg === '-h') opts.help = true;
    else opts.words.push(arg);
  }
  return opts;
}

/** Everything on stdin, for `echo ... | node scripts/kimi.mjs`. */
async function readStdin() {
  if (process.stdin.isTTY) return '';
  process.stdin.setEncoding('utf8');
  let text = '';
  for await (const chunk of process.stdin) text += chunk;
  return text;
}

/**
 * One run of Kimi, as its JSONL events.
 *
 * `--output-format stream-json` gives one JSON object per line: the assistant's answer, and a
 * `session.resume_hint` naming the session. Anything else it prints is progress, and belongs on
 * stderr rather than in the answer.
 */
function runKimi({ bin, prompt, dir, resume, timeoutMs }) {
  const args = ['-p', prompt, '--output-format', 'stream-json'];
  if (resume) args.unshift('-r', resume);
  return new Promise((done, fail) => {
    const proc = spawn(bin, args, { cwd: dir, stdio: ['ignore', 'pipe', 'pipe'] });
    let out = '';
    let err = '';
    const timer = setTimeout(() => {
      proc.kill();
      fail(new Error(`kimi did not answer within ${Math.round(timeoutMs / 1000)}s`));
    }, timeoutMs);
    proc.stdout.on('data', (chunk) => {
      out += chunk.toString();
    });
    proc.stderr.on('data', (chunk) => {
      err += chunk.toString();
    });
    proc.on('error', (cause) => {
      clearTimeout(timer);
      fail(new Error(`could not run ${bin}: ${cause.message}`));
    });
    proc.on('close', (code) => {
      clearTimeout(timer);
      if (code !== 0) fail(new Error(`kimi exited ${code}\n${err.trim() || out.trim()}`));
      else done(out);
    });
  });
}

/** The answer and the session id, pulled out of the JSONL. */
function readEvents(jsonl) {
  const answer = [];
  let session = '';
  for (const line of jsonl.split(/\r?\n/)) {
    if (line.trim() === '') continue;
    let event;
    try {
      event = JSON.parse(line);
    } catch {
      continue; // progress chatter, not an event
    }
    if (event.role === 'assistant' && typeof event.content === 'string') answer.push(event.content);
    if (event.type === 'session.resume_hint') session = event.session_id ?? '';
  }
  return { answer: answer.join('\n').trim(), session };
}

const opts = parseArgs(process.argv.slice(2));

if (opts.help) {
  const source = readFileSync(fileURLToPath(import.meta.url), 'utf8');
  console.log(source.split('\n\nimport')[0].replace(/^\/\/ ?/gm, ''));
  process.exit(0);
}

const prompt = opts.file
  ? readFileSync(opts.file, 'utf8')
  : opts.words.length > 0
    ? opts.words.join(' ')
    : (await readStdin()).trim();

if (prompt.trim() === '') {
  console.error('Nothing to ask. Give a prompt, --file, or pipe one in; --help explains the rest.');
  process.exit(2);
}

const bin = kimiBinary();
const timeoutMs = opts.timeoutMs ?? 900_000;

let jsonl;
try {
  jsonl = await runKimi({ bin, prompt, dir: opts.dir, resume: opts.resume, timeoutMs });
} catch (cause) {
  console.error(String(cause.message));
  if (bin === 'kimi') {
    console.error('Kimi Code was not where this looks for it. Install it from https://code.kimi.com,');
    console.error('then `kimi login`, or set KIMI_BIN to the binary.');
  }
  process.exit(1);
}

if (opts.json) {
  process.stdout.write(jsonl);
  process.exit(0);
}

const { answer, session } = readEvents(jsonl);

if (opts.out) {
  writeFileSync(opts.out, `${answer}\n`);
  console.log(`${opts.out}: ${answer.length} characters`);
} else {
  console.log(answer);
}
if (session !== '') console.error(`\nresume with: node scripts/kimi.mjs --resume ${session} "..."`);
