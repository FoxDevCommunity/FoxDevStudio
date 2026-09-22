// Generates src/data/instructions.json from the VM's own `Instr` enum, so the instruction
// reference on the site cannot drift from the bytecode the runtime runs.
//
//   node scripts/instructions.mjs          writes the file
//   node scripts/instructions.mjs --check  exits 1 when the file is out of date
//
// It reads `pub enum Instr { ... }` in crates/foxvm/src/bytecode.rs: the `// ---- name` lines
// that divide the enum become the groups, each variant's `///` comment becomes its
// description, and a struct variant's fields (with their own comments) become its operands.
import { readFileSync, writeFileSync, existsSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const here = dirname(fileURLToPath(import.meta.url));
const source = join(here, '..', '..', 'crates', 'foxvm', 'src', 'bytecode.rs');
const target = join(here, '..', 'src', 'data', 'instructions.json');

const text = readFileSync(source, 'utf8');
const start = text.indexOf('pub enum Instr {');
if (start < 0) throw new Error('pub enum Instr not found');
const end = text.indexOf('\n}\n', start);
const lines = text.slice(start, end).split('\n').slice(1);

const constants = {
  MAGIC: (text.match(/pub const MAGIC: &\[u8; 4\] = b"([A-Z]+)"/) ?? [])[1],
  FORMAT_VERSION: Number((text.match(/pub const FORMAT_VERSION: u16 = (\d+)/) ?? [])[1]),
};

/** `[a, b] -> [r]` at the front of a doc line is the stack effect; the rest is the prose. */
function splitEffect(doc) {
  const m = doc.match(/^(\[[^\]]*\]\s*->\s*\[[^\]]*\])\s*(.*)$/s);
  return m ? { effect: m[1].replace(/\s+/g, ' '), doc: m[2].trim() } : { effect: '', doc };
}

// A variant is encoded by its position in the enum, so one that was added after its family
// cannot be moved next to it; these are shown under the group they belong to instead.
const homes = {
  ReturnTo: 'control flow',
  Retry: 'exceptions',
  Throw: 'exceptions',
  CatchObject: 'exceptions',
  EndFinally: 'exceptions',
  OnError: 'exceptions',
  LoadField: 'data',
  Blank: 'data',
  Nop: 'stack',
};

const groups = [];
let group = { name: 'Ungrouped', note: '', instructions: [] };
let doc = [];
let i = 0;
while (i < lines.length) {
  const line = lines[i];
  const trimmed = line.trim();
  const section = trimmed.match(/^\/\/ ---- (.*)$/);
  if (section) {
    if (group.instructions.length) groups.push(group);
    const [name, ...rest] = section[1].split(/[:.] /);
    group = { name: name.trim(), note: rest.join('. ').trim(), instructions: [] };
    doc = [];
    i++;
    continue;
  }
  if (trimmed.startsWith('///')) {
    doc.push(trimmed.replace(/^\/\/\/ ?/, ''));
    i++;
    continue;
  }
  if (trimmed === '' || trimmed.startsWith('//') || trimmed.startsWith('#[')) {
    i++;
    continue;
  }
  // a variant: `Name,`  `Name(T, U),`  `Name { a: T, b: U },`  or `Name {` ... `},`
  const unit = trimmed.match(/^([A-Z][A-Za-z0-9]*),$/);
  const tuple = trimmed.match(/^([A-Z][A-Za-z0-9]*)\((.*)\),$/);
  const oneLine = trimmed.match(/^([A-Z][A-Za-z0-9]*) \{ (.*) \},$/);
  const struct = trimmed.match(/^([A-Z][A-Za-z0-9]*) \{$/);
  if (unit || tuple || oneLine) {
    const name = (unit ?? tuple ?? oneLine)[1];
    const operands = tuple
      ? tuple[2].split(',').map((t) => ({ name: '', type: t.trim(), doc: '' }))
      : oneLine
        ? oneLine[2].split(',').map((f) => {
            const [n, t] = f.split(':').map((s) => s.trim());
            return { name: n, type: t, doc: '' };
          })
        : [];
    const shape = tuple ? 'tuple' : oneLine ? 'struct' : 'unit';
    group.instructions.push({ name, shape, operands, ...splitEffect(doc.join(' ')) });
    doc = [];
    i++;
    continue;
  }
  if (struct) {
    const name = struct[1];
    const operands = [];
    let fieldDoc = [];
    i++;
    while (i < lines.length && lines[i].trim() !== '},') {
      const t = lines[i].trim();
      if (t.startsWith('///')) fieldDoc.push(t.replace(/^\/\/\/ ?/, ''));
      else if (t.startsWith('#[')) {
        // serde attributes say nothing about the instruction
      } else {
        const f = t.match(/^([a-z_0-9]+): (.*),$/);
        if (f) operands.push({ name: f[1], type: f[2], doc: fieldDoc.join(' ') });
        fieldDoc = [];
      }
      i++;
    }
    group.instructions.push({ name, shape: 'struct', operands, ...splitEffect(doc.join(' ')) });
    doc = [];
    i++;
    continue;
  }
  throw new Error(`cannot read line: ${line}`);
}
if (group.instructions.length) groups.push(group);

for (const g of groups) {
  for (const ins of [...g.instructions]) {
    const home = homes[ins.name];
    if (!home || home === g.name) continue;
    const to = groups.find((x) => x.name.startsWith(home));
    if (!to) throw new Error(`no group starts with ${home} for ${ins.name}`);
    g.instructions.splice(g.instructions.indexOf(ins), 1);
    to.instructions.push({ ...ins, moved: true });
  }
}

const out = {
  generated: 'by marketing/scripts/instructions.mjs from crates/foxvm/src/bytecode.rs; do not edit',
  ...constants,
  count: groups.reduce((n, g) => n + g.instructions.length, 0),
  groups,
};
const json = `${JSON.stringify(out, null, 2)}\n`;

if (process.argv.includes('--check')) {
  const current = existsSync(target) ? readFileSync(target, 'utf8') : '';
  if (current !== json) {
    console.error(`[instructions] ${target} is out of date: run node scripts/instructions.mjs`);
    process.exit(1);
  }
  console.log(`[instructions] up to date: ${out.count} instructions in ${groups.length} groups`);
} else {
  writeFileSync(target, json);
  console.log(`[instructions] wrote ${out.count} instructions in ${groups.length} groups to ${target}`);
}
