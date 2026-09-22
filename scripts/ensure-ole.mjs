// Rebuilds the COM addon only when its Rust sources are newer than the built one.
import { spawnSync } from 'node:child_process';
import { existsSync, readdirSync, statSync } from 'node:fs';
import { join, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = dirname(dirname(fileURLToPath(import.meta.url)));
const output = join(root, 'resources', 'native', 'foxole.node');

if (process.platform !== 'win32') {
  console.log('[ensure-ole] not Windows: no COM addon to build');
  process.exit(0);
}

function newest(path) {
  if (!existsSync(path)) return 0;
  const st = statSync(path);
  if (!st.isDirectory()) return st.mtimeMs;
  let max = st.mtimeMs;
  for (const entry of readdirSync(path)) max = Math.max(max, newest(join(path, entry)));
  return max;
}

const inputs = [join(root, 'crates', 'foxole'), join(root, 'Cargo.lock')];
const sourceTime = Math.max(...inputs.map(newest));
const outputTime = existsSync(output) ? statSync(output).mtimeMs : 0;

if (outputTime >= sourceTime) {
  console.log('[ensure-ole] up to date');
} else {
  console.log('[ensure-ole] building the COM addon');
  const r = spawnSync(process.execPath, [join(root, 'scripts', 'build-ole.mjs')], { cwd: root, stdio: 'inherit' });
  process.exit(r.status ?? 1);
}
