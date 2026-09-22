import { mkdtemp, mkdir, readFile, rm, writeFile } from 'node:fs/promises';
import { existsSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { buildExecutable, embeddedBundlePath, resolvePlayTarget } from '@main/services/buildService';

let root: string;
let appDir: string;
let outDir: string;

/** A stand-in for an installed FoxDev Studio: an exe, resources and an asar. */
async function fakeInstall(withBundle = false): Promise<void> {
  await mkdir(join(appDir, 'resources'), { recursive: true });
  await writeFile(join(appDir, 'FoxDev Studio.exe'), 'MZ fake');
  await writeFile(join(appDir, 'resources', 'app.asar'), 'asar');
  await writeFile(join(appDir, 'chrome_100_percent.pak'), 'pak');
  if (withBundle) await writeFile(join(appDir, 'resources', 'app.fxa'), '{"old":true}');
}

const deps = (packaged = true) => ({ appDir, exeName: 'FoxDev Studio.exe', packaged });

beforeEach(async () => {
  root = await mkdtemp(join(tmpdir(), 'foxdev-build-'));
  appDir = join(root, 'install');
  outDir = join(root, 'out');
  await mkdir(outDir, { recursive: true });
});

afterEach(async () => {
  await rm(root, { recursive: true, force: true });
});

describe('Build Executable', () => {
  it('copies the runtime, embeds the bundle and renames the executable', async () => {
    await fakeInstall();
    const result = await buildExecutable({ name: 'Hello World', outDir, bundle: '{"$schema":"foxdev-app"}' }, deps());

    expect(result.ok).toBe(true);
    const target = join(outDir, 'Hello World');
    expect(result.exePath).toBe(join(target, 'Hello World.exe'));
    expect(existsSync(result.exePath!)).toBe(true);
    // the original executable name is gone, and the runtime files came along
    expect(existsSync(join(target, 'FoxDev Studio.exe'))).toBe(false);
    expect(existsSync(join(target, 'resources', 'app.asar'))).toBe(true);
    expect(existsSync(join(target, 'chrome_100_percent.pak'))).toBe(true);
    expect(await readFile(join(target, 'resources', 'app.fxa'), 'utf8')).toBe('{"$schema":"foxdev-app"}');
  });

  it('does not carry the source application\'s own bundle into the copy', async () => {
    await fakeInstall(true);
    await buildExecutable({ name: 'App2', outDir, bundle: '{"new":true}' }, deps());
    expect(await readFile(join(outDir, 'App2', 'resources', 'app.fxa'), 'utf8')).toBe('{"new":true}');
  });

  it('replaces an earlier build of the same name', async () => {
    await fakeInstall();
    await mkdir(join(outDir, 'HelloWorld'), { recursive: true });
    await writeFile(join(outDir, 'HelloWorld', 'stale.txt'), 'old');

    await buildExecutable({ name: 'HelloWorld', outDir, bundle: '{}' }, deps());
    expect(existsSync(join(outDir, 'HelloWorld', 'stale.txt'))).toBe(false);
  });

  it('refuses to build from a development run, where there is no installed app', async () => {
    await fakeInstall();
    const result = await buildExecutable({ name: 'X', outDir, bundle: '{}' }, deps(false));
    expect(result).toMatchObject({ ok: false });
    expect(result.error).toMatch(/installed FoxDev Studio/);
  });

  it('reports an error instead of throwing when the copy fails', async () => {
    // no install directory at all
    const result = await buildExecutable({ name: 'X', outDir, bundle: '{}' }, deps());
    expect(result.ok).toBe(false);
    expect(result.error).toBeTruthy();
  });
});

describe('player target resolution', () => {
  it('prefers FOXDEV_PLAY, then --play, then a positional bundle, then the embedded one', async () => {
    const bundle = join(root, 'app.fxa');
    await writeFile(bundle, '{}');
    const resources = join(root, 'resources');
    await mkdir(resources, { recursive: true });
    const embedded = join(resources, 'app.fxa');
    await writeFile(embedded, '{}');

    expect(resolvePlayTarget([], { FOXDEV_PLAY: bundle }, resources)).toBe(bundle);
    expect(resolvePlayTarget(['--play', bundle], {}, resources)).toBe(bundle);
    expect(resolvePlayTarget([bundle], {}, resources)).toBe(bundle);
    expect(resolvePlayTarget([], {}, resources)).toBe(embedded);
  });

  it('ignores paths that do not exist and reports no bundle for the IDE', () => {
    expect(resolvePlayTarget(['--play', join(root, 'missing.fxa')], {}, root)).toBeNull();
    expect(resolvePlayTarget([], { FOXDEV_PLAY: join(root, 'missing.fxa') }, root)).toBeNull();
    expect(embeddedBundlePath(root)).toBeNull();
  });
});

describe('application names', () => {
  it('drops characters Windows refuses but keeps spaces', async () => {
    await fakeInstall();
    const result = await buildExecutable({ name: 'My:App?<v1> ', outDir, bundle: '{}' }, deps());
    expect(result.exePath).toBe(join(outDir, 'MyAppv1', 'MyAppv1.exe'));

    // a path separator in the name must not create a nested folder
    const slashed = await buildExecutable({ name: String.raw`a\b/c`, outDir, bundle: '{}' }, deps());
    expect(slashed.exePath).toBe(join(outDir, 'abc', 'abc.exe'));
  });
});
