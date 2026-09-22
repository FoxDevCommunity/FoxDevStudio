// Rebuilds the Visual FoxPro library host only when its source is newer than the built one.
//
// The host is a build-time artefact: `resources/native/win32/fllhost.exe` is produced here and shipped
// inside the application, exactly as `resources/native/foxole.node` is. Nothing at run time ever
// looks for a compiler.
import { spawnSync } from 'node:child_process';
import { existsSync, statSync } from 'node:fs';
import { join, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = dirname(dirname(fileURLToPath(import.meta.url)));
const output = join(root, 'resources', 'native', 'win32', 'fllhost.exe');

if (process.platform !== 'win32') {
  console.log('[ensure-fllhost] not Windows: there are no FoxPro libraries to host');
  process.exit(0);
}

const when = (path) => (existsSync(path) ? statSync(path).mtimeMs : 0);
const sourceTime = Math.max(
  when(join(root, 'native', 'fllhost', 'fllhost.c')),
  when(join(root, 'scripts', 'build-fllhost.mjs')),
);

if (when(output) >= sourceTime) {
  console.log('[ensure-fllhost] up to date');
} else {
  console.log('[ensure-fllhost] building the library host');
  const r = spawnSync(process.execPath, [join(root, 'scripts', 'build-fllhost.mjs')], { cwd: root, stdio: 'inherit' });
  process.exit(r.status ?? 1);
}
