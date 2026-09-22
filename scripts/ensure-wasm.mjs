// Rebuilds the wasm module only when the Rust sources are newer than the generated output.
import { spawnSync } from 'node:child_process';
import { existsSync, readdirSync, statSync } from 'node:fs';
import { join, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = dirname(dirname(fileURLToPath(import.meta.url)));
const output = join(root, 'src', 'wasm', 'foxvm', 'generated', 'foxvm_wasm.ts');

function newest(path) {
  if (!existsSync(path)) return 0;
  const st = statSync(path);
  if (!st.isDirectory()) return st.mtimeMs;
  let max = st.mtimeMs;
  for (const entry of readdirSync(path)) {
    if (entry === 'target') continue;
    max = Math.max(max, newest(join(path, entry)));
  }
  return max;
}

const inputs = [join(root, 'crates'), join(root, 'Cargo.toml'), join(root, 'Cargo.lock')];
const sourceTime = Math.max(...inputs.map(newest));
const outputTime = existsSync(output) ? statSync(output).mtimeMs : 0;

if (outputTime >= sourceTime) {
  console.log('[ensure-wasm] up to date');
} else {
  console.log('[ensure-wasm] Rust sources changed, rebuilding wasm');
  const r = spawnSync(process.execPath, [join(root, 'scripts', 'build-wasm.mjs')], { cwd: root, stdio: 'inherit' });
  process.exit(r.status ?? 1);
}
