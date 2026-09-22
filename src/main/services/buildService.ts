/**
 * "Build Executable": produces a standalone application folder around a compiled bundle.
 *
 * VFP linked a small stub to its runtime DLLs; the equivalent here is a copy of the installed
 * FoxDev application with the project's `.fxa` dropped into its resources and the executable
 * renamed. The copy is the runtime; the bundle is the program. Building an installer for the
 * result belongs to a later milestone.
 */

import { cp, mkdir, readdir, rename, rm, writeFile } from 'node:fs/promises';
import { existsSync } from 'node:fs';
import { basename, dirname, join } from 'node:path';
import type { BuildExeOptions, BuildExeResult } from '@shared/ipc/api';

export interface BuildDeps {
  /** Directory of the running application (where the .exe lives). */
  appDir: string;
  /** File name of the running executable. */
  exeName: string;
  /** False in development, where there is no installed app to copy. */
  packaged: boolean;
  /** Lets the caller allow the output directory for later file access. */
  allowDir?(dir: string): void;
}

/** Punctuation Windows refuses in a file name. Spaces are legal, so they are kept. */
const ILLEGAL_IN_FILENAME = new Set(['<', '>', ':', '"', '/', '\\', '|', '?', '*']);

function safeName(name: string): string {
  const cleaned = [...name]
    // control characters are illegal too, but a regex for them trips no-control-regex
    .filter((ch) => !ILLEGAL_IN_FILENAME.has(ch) && ch.codePointAt(0)! > 0x1f)
    .join('')
    // Windows also refuses a trailing dot or space
    .replace(/[. ]+$/, '');
  return cleaned.length > 0 ? cleaned : 'Application';
}

export async function buildExecutable(opts: BuildExeOptions, deps: BuildDeps): Promise<BuildExeResult> {
  if (!deps.packaged) {
    return { ok: false, error: 'Build Executable needs the installed FoxDev Studio; it cannot run from a development build.' };
  }
  const name = safeName(opts.name);
  const target = join(opts.outDir, name);

  try {
    if (existsSync(target)) await rm(target, { recursive: true, force: true });
    await mkdir(target, { recursive: true });

    // copy the runtime, minus any bundle the source application itself carries
    await cp(deps.appDir, target, {
      recursive: true,
      filter: (src) => basename(src).toLowerCase() !== 'app.fxa',
    });

    const resources = join(target, 'resources');
    await mkdir(resources, { recursive: true });
    await writeFile(join(resources, 'app.fxa'), opts.bundle, 'utf8');

    const copiedExe = join(target, deps.exeName);
    const exePath = join(target, `${name}.exe`);
    if (existsSync(copiedExe) && copiedExe !== exePath) await rename(copiedExe, exePath);

    deps.allowDir?.(target);
    return { ok: existsSync(exePath), exePath, error: existsSync(exePath) ? undefined : 'The application executable was not found in the copy.' };
  } catch (err) {
    return { ok: false, error: err instanceof Error ? err.message : String(err) };
  }
}

/** Where a packaged application keeps its embedded bundle, if it has one. */
export function embeddedBundlePath(resourcesPath: string): string | null {
  const candidate = join(resourcesPath, 'app.fxa');
  return existsSync(candidate) ? candidate : null;
}

/** Resolves the bundle a window was launched to run: env var, `--play <file>`, or embedded. */
export function resolvePlayTarget(argv: string[], env: NodeJS.ProcessEnv, resourcesPath: string): string | null {
  const fromEnv = env['FOXDEV_PLAY'];
  if (fromEnv && existsSync(fromEnv)) return fromEnv;

  const flag = argv.indexOf('--play');
  if (flag >= 0) {
    const path = argv[flag + 1];
    if (path && existsSync(path)) return path;
  }
  const positional = argv.find((a) => a.toLowerCase().endsWith('.fxa') && existsSync(a));
  if (positional) return positional;

  return embeddedBundlePath(resourcesPath);
}

/** Lists an application folder, for tests and diagnostics. */
export async function listFolder(dir: string): Promise<string[]> {
  const entries = await readdir(dir, { withFileTypes: true, recursive: true });
  return entries.filter((e) => e.isFile()).map((e) => join(e.parentPath ?? dirname(dir), e.name));
}

/**
 * Where the bundled Visual FoxPro Foundation Classes live.
 *
 * They sit under `resources/` in the repository and are packed into the app archive, which
 * Electron lets `fs` read straight through, so one path works in development and when packaged.
 */
export function classLibraryDir(appPath: string): string {
  return join(appPath, 'resources', 'ffc');
}
