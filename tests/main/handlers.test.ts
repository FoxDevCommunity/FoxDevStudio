import { mkdtemp, readFile, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { createHandlers, type HandlerDeps } from '@main/ipc/handlers';
import { nodeFileService } from '@main/services/fileService';
import { createRecentFiles } from '@main/services/recentFiles';
import { PathGuard } from '@main/ipc/pathGuard';

let dir: string;
let deps: HandlerDeps;
let guard: PathGuard;

beforeEach(async () => {
  dir = await mkdtemp(join(tmpdir(), 'foxdev-'));
  guard = new PathGuard();
  deps = {
    getVersion: () => '1.2.3',
    fs: nodeFileService,
    classLibraryDir: join(dir, 'ffc'),
    recent: createRecentFiles(nodeFileService, join(dir, 'userData', 'recent.json')),
    guard,
    dialogs: { openFile: vi.fn(async () => null), saveFile: vi.fn(async () => null), pickFolder: vi.fn(async () => null), message: vi.fn(async () => 1) },
    window: { setTitle: vi.fn(), setDocumentEdited: vi.fn(), confirmClose: vi.fn() },
  };
});
afterEach(() => rm(dir, { recursive: true, force: true }));

describe('main ipc handlers', () => {
  it('returns the injected version and forwards window calls', async () => {
    const h = createHandlers(deps);
    expect(await h['app:getVersion']()).toBe('1.2.3');
    await h['app:setTitle']('x');
    expect(deps.window.setTitle).toHaveBeenCalledWith('x');
    await h['app:confirmClose'](true);
    expect(deps.window.confirmClose).toHaveBeenCalledWith(true);
  });

  it('grants a path near one already reachable, and refuses one that is not', async () => {
    const h = createHandlers(deps);
    const install = join(dir, 'vfp9');
    const project = join(install, 'samples', 'solution', 'solution.pjx');
    const beside = join(install, 'samples', 'classes', 'samples.vcx');
    const atRoot = join(install, 'ffc', '_hyperlink.vcx');

    // nothing is reachable to begin with, so there is nothing to reach out from
    expect(await h['project:allowNear'](beside)).toBe(false);

    guard.allowFile(project);
    expect(guard.isAllowed(beside)).toBe(false);

    // the classes folder beside the project, and the ffc one at the product root, are both
    // inside the tree the project sits in
    expect(await h['project:allowNear'](beside)).toBe(true);
    expect(guard.isAllowed(beside)).toBe(true);
    expect(await h['project:allowNear'](atRoot)).toBe(true);
    expect(guard.isAllowed(atRoot)).toBe(true);

    // and it is bounded: far enough above the project is another tree, and stays refused
    const faraway = join(dir, 'elsewhere', 'id_rsa');
    expect(await h['project:allowNear'](faraway)).toBe(false);
    expect(guard.isAllowed(faraway)).toBe(false);
  });

  it('creates, saves and reopens a project inside a folder chosen through a dialog', async () => {
    const h = createHandlers(deps);
    const projDir = join(dir, 'MyApp');
    await expect(h['project:create'](projDir, 'MyApp')).rejects.toThrow('Access denied');

    (deps.dialogs.pickFolder as ReturnType<typeof vi.fn>).mockResolvedValueOnce(projDir);
    expect(await h['dialog:pickFolder'](undefined)).toBe(projDir);
    const { path, doc } = await h['project:create'](projDir, 'MyApp');
    expect(path).toBe(join(projDir, 'MyApp.fxproject'));
    expect(doc.name).toBe('MyApp');

    await h['files:writeText'](join(projDir, 'Form1.fxf'), '{}');
    expect(await h['files:exists'](join(projDir, 'Form1.fxf'))).toBe(true);
    expect(await h['files:readText'](join(projDir, 'Form1.fxf'))).toBe('{}');

    await h['project:save'](path, { ...doc, items: [{ kind: 'form', path: 'Form1.fxf' }] });
    expect(JSON.parse(await readFile(path, 'utf8')).items).toHaveLength(1);

    const reopened = await h['project:open'](path);
    expect(reopened.doc.items[0]!.path).toBe('Form1.fxf');
    await expect(h['project:open'](join(projDir, 'Form1.fxf'))).rejects.toThrow('Cannot open project');
  });

  it('refuses paths outside allowed roots, but trusts recent projects', async () => {
    const h = createHandlers(deps);
    const outside = join(dir, 'elsewhere', 'x.txt');
    await expect(h['files:readText'](outside)).rejects.toThrow('Access denied');
    await expect(h['files:writeText'](outside, 'x')).rejects.toThrow('Access denied');

    const projPath = join(dir, 'Recent', 'R.fxproject');
    await nodeFileService.writeText(projPath, '{"$schema":"foxdev-project","version":1,"name":"R","items":[]}');
    await expect(h['project:open'](projPath)).rejects.toThrow('Access denied');
    await h['app:addRecentProject'](projPath);
    expect(await h['app:getRecentProjects']()).toEqual([projPath]);
    expect((await h['project:open'](projPath)).doc.name).toBe('R');
    // once opened, the project directory is readable
    expect(await h['files:exists'](join(dir, 'Recent', 'nothing.prg'))).toBe(false);
  });

  it('allows files returned by open/save dialogs', async () => {
    const h = createHandlers(deps);
    const file = join(dir, 'picked', 'a.prg');
    (deps.dialogs.saveFile as ReturnType<typeof vi.fn>).mockResolvedValueOnce(file);
    expect(await h['dialog:saveFile']({})).toBe(file);
    await h['files:writeText'](file, 'RETURN');
    expect(await h['files:readText'](file)).toBe('RETURN');
    expect(await h['dialog:message']({ type: 'info', message: 'm', buttons: ['a', 'b'] })).toBe(1);
  });
});
