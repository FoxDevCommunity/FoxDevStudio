/**
 * Where Visual FoxPro is installed, for the directories `HOME()` answers about.
 *
 * `HOME(2)` is the samples directory and `_SAMPLES` is the same thing, so a program that opens
 * `_samples + "\Data\customer.dbf"` - which the samples that ship with Visual FoxPro all do -
 * needs a real path to work from. FoxDev has no samples of its own of that kind, so the only
 * thing those names can mean is the installation's, which Windows records where the installer
 * put it.
 */

import { execFile } from 'node:child_process';
import { promisify } from 'node:util';
import { join } from 'node:path';

const run = promisify(execFile);

/** Where the Visual FoxPro 9 installer records the product directory. */
const KEYS = [
  'HKLM\\SOFTWARE\\WOW6432Node\\Microsoft\\VisualFoxPro\\9.0\\Setup\\VFP',
  'HKLM\\SOFTWARE\\Microsoft\\VisualFoxPro\\9.0\\Setup\\VFP',
];

let found: string | null | undefined;

/** The installation directory, or `null` when Visual FoxPro is not installed. */
export async function foxproHome(): Promise<string | null> {
  if (found !== undefined) return found;
  found = null;
  if (process.platform === 'win32') {
    for (const key of KEYS) {
      try {
        const { stdout } = await run('reg', ['query', key, '/v', 'ProductDir'], { windowsHide: true });
        const value = /ProductDir\s+REG_[A-Z_]+\s+(.+)/i.exec(stdout)?.[1]?.trim();
        if (value) {
          found = value.replace(/\\+$/, '');
          break;
        }
      } catch {
        // no such key: try the next one, and answer nothing when none of them is there
      }
    }
  }
  return found;
}

/**
 * The directory `HOME(n)` names, or "" when there is none.
 *
 * The numbers are Visual FoxPro's: 0 the product directory, 1 the directory the running
 * application was started from, 2 the samples, and the rest the folders the product keeps
 * beside itself.
 */
export async function homeDir(which: number, appDir: string): Promise<string> {
  if (which === 1) return withSlash(appDir);
  const home = await foxproHome();
  if (home === null) return which === 0 ? withSlash(appDir) : '';
  const under = (...parts: string[]) => withSlash(join(home, ...parts));
  switch (which) {
    case 0:
      return under();
    case 2:
      return under('Samples');
    case 3:
      return under('Wizards');
    case 4:
      return under('Ffc');
    case 5:
      return under('Gallery');
    case 6:
      return under('Tools');
    default:
      return '';
  }
}

/** Visual FoxPro's directory names end with a separator, and programs join onto them. */
function withSlash(dir: string): string {
  if (dir === '') return dir;
  return dir.endsWith('\\') || dir.endsWith('/') ? dir : `${dir}\\`;
}
