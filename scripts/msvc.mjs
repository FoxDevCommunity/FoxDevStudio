// Finds a Visual C++ command line. The .fll host has to be built for x86 whatever this machine
// is, because every Visual FoxPro library is a 32-bit image, so the environment is asked for by
// architecture rather than taken from whatever shell happens to be running.
import { spawnSync } from 'node:child_process';
import { existsSync } from 'node:fs';
import { join } from 'node:path';

const VSWHERE = join(
  process.env['ProgramFiles(x86)'] ?? 'C:/Program Files (x86)',
  'Microsoft Visual Studio',
  'Installer',
  'vswhere.exe',
);

/** Where Visual Studio is installed, or null when it is not. */
export function visualStudio() {
  if (process.platform !== 'win32' || !existsSync(VSWHERE)) return null;
  const r = spawnSync(
    VSWHERE,
    ['-latest', '-products', '*', '-requires', 'Microsoft.VisualStudio.Component.VC.Tools.x86.x64', '-property', 'installationPath'],
    { encoding: 'utf8' },
  );
  const path = (r.stdout ?? '').trim().split(/\r?\n/)[0] ?? '';
  return path !== '' && existsSync(path) ? path : null;
}

/**
 * The environment `cl.exe` needs to build for `arch`, or null when there is no Visual C++ here.
 * vcvarsall.bat only speaks to a command processor, so it is run in one and asked what it set.
 */
export function msvcEnv(arch = 'x86') {
  const vs = visualStudio();
  if (!vs) return null;
  const vcvars = join(vs, 'VC', 'Auxiliary', 'Build', 'vcvarsall.bat');
  if (!existsSync(vcvars)) return null;
  const r = spawnSync(`"${vcvars}" ${arch} >nul && set`, { shell: true, encoding: 'utf8' });
  if (r.status !== 0) return null;
  const env = { ...process.env };
  for (const line of (r.stdout ?? '').split(/\r?\n/)) {
    const at = line.indexOf('=');
    if (at > 0) env[line.slice(0, at)] = line.slice(at + 1);
  }
  return env.INCLUDE ? env : null;
}
