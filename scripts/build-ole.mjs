// Builds crates/foxole into the Node addon that talks to COM, and puts it where the main
// process looks. Windows only: everywhere else there is no COM to talk to, and the runtime
// says so rather than loading anything.
import { spawnSync } from 'node:child_process';
import { copyFileSync, existsSync, mkdirSync, statSync } from 'node:fs';
import { join, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = dirname(dirname(fileURLToPath(import.meta.url)));
const outDir = join(root, 'resources', 'native');

if (process.platform !== 'win32') {
  console.log('[build-ole] not Windows: skipping the COM addon');
  process.exit(0);
}

const r = spawnSync('cargo', ['build', '--release', '-p', 'foxole'], { cwd: root, stdio: 'inherit', shell: true });
if (r.status !== 0) process.exit(r.status ?? 1);

mkdirSync(outDir, { recursive: true });
const built = join(root, 'target', 'release', 'foxole.dll');
const addon = join(outDir, 'foxole.node');
try {
  copyFileSync(built, addon);
  console.log(`[build-ole] ${addon}`);
} catch (e) {
  // A running copy of the application holds the addon open, and Windows will not let anything
  // write over a loaded library. Starting a second one is an ordinary thing to do while the
  // first is still up - and it works, because the addon it would have copied is the same one -
  // so say what happened and carry on rather than refusing to start at all. Only a build that
  // would really have changed the file is worth stopping for.
  const fresh = existsSync(addon) && statSync(addon).mtimeMs >= statSync(built).mtimeMs;
  if (!fresh) throw e;
  console.log(`[build-ole] ${addon} is in use by something already running, and is current: kept`);
}
