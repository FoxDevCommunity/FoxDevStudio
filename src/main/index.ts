import { app, BrowserWindow, dialog, Menu } from 'electron';
import { join } from 'node:path';
import { createMainWindow, type MainWindow } from './window';
import { registerIpc, registerLibrary, registerOle } from './ipc/register';
import { createHandlers } from './ipc/handlers';
import { createOleService } from './services/oleService';
import { createLibraryService } from './services/libraryService';
import { createTableService } from './services/tableService';
import { createHttpService } from '@shared/runtime/httpService';
import { nodeFileService } from './services/fileService';
import { createRecentFiles } from './services/recentFiles';
import { PathGuard } from './ipc/pathGuard';
import { buildExecutable, classLibraryDir, resolvePlayTarget } from './services/buildService';
import { handlePictureRequests, registerPictureScheme } from './services/pictureProtocol';
import { basename, dirname } from 'node:path';

let main: MainWindow | null = null;

// Linux/WSLg: render through native Wayland so HiDPI scaling is done by Chromium (crisp text)
// instead of the compositor upscaling an X11 bitmap. FOXDEV_SCALE forces a device scale factor.
if (process.platform === 'linux' && process.env['WAYLAND_DISPLAY'] && !process.env['FOXDEV_X11']) {
  app.commandLine.appendSwitch('ozone-platform', 'wayland');
  app.commandLine.appendSwitch('enable-features', 'WaylandWindowDecorations');
  // WSLg exposes no DRM render node, so the GPU compositor fails and paints white; use software rendering
  app.disableHardwareAcceleration();
}
if (process.env['FOXDEV_SCALE']) app.commandLine.appendSwitch('force-device-scale-factor', process.env['FOXDEV_SCALE']);

// must happen before the app is ready
registerPictureScheme();

app.whenReady().then(() => {
  // the renderer draws its own Fluent menu bar and handles accelerators
  Menu.setApplicationMenu(null);
  const guard = new PathGuard();
  guard.allowDir(app.getPath('userData'));
  handlePictureRequests(guard);
  const recent = createRecentFiles(nodeFileService, join(app.getPath('userData'), 'recent-projects.json'));

  // a bundle on the command line (or embedded in the installation) turns this into the player
  const playBundle = resolvePlayTarget(process.argv.slice(1), process.env, process.resourcesPath);
  if (playBundle) guard.allowDir(dirname(playBundle));

  const startupProject = playBundle ? null : (process.env['FOXDEV_OPEN'] ?? process.argv.slice(1).find((a) => a.toLowerCase().endsWith('.fxproject')) ?? null);

  // held here so the files are let go of when the app quits, however it is closed
  const tables = createTableService();
  app.on('will-quit', () => void tables.closeAll());

  // COM automation lives in this process: the objects belong to its apartment, not the renderer's
  registerOle(createOleService([join(app.getAppPath(), 'resources', 'native'), join(process.resourcesPath, 'native'), process.resourcesPath]));

  // and so does the process that hosts a .fll: a 32-bit library cannot be loaded by anything here
  registerLibrary(
    createLibraryService([
      join(app.getAppPath(), 'resources', 'native', 'win32'),
      join(process.resourcesPath, 'native', 'win32'),
      join(process.resourcesPath, 'win32'),
    ]),
  );

  // the sockets a FoxScript.Http server listens on; a window that goes leaves no port open
  const http = createHttpService();
  app.on('will-quit', () => void http.closeAll());

  registerIpc(
    createHandlers({
      tables,
      http,
      sendHttpRequest: (id, request) => main?.win.webContents.send('http:request', id, request),
      getVersion: () => app.getVersion(),
      startupProject,
      fs: nodeFileService,
      recent,
      guard,
      classLibraryDir: classLibraryDir(app.getAppPath()),
      playBundle,
      buildExe: (opts) =>
        buildExecutable(opts, {
          appDir: dirname(app.getPath('exe')),
          exeName: basename(app.getPath('exe')),
          packaged: app.isPackaged,
          allowDir: (dir) => guard.allowDir(dir),
        }),
      dialogs: {
        openFile: async (opts) => {
          const r = await dialog.showOpenDialog(main!.win, { ...opts, properties: ['openFile'] });
          return r.canceled ? null : (r.filePaths[0] ?? null);
        },
        saveFile: async (opts) => {
          const r = await dialog.showSaveDialog(main!.win, opts);
          return r.canceled || !r.filePath ? null : r.filePath;
        },
        pickFolder: async (opts) => {
          const r = await dialog.showOpenDialog(main!.win, { ...opts, properties: ['openDirectory', 'createDirectory'] });
          return r.canceled ? null : (r.filePaths[0] ?? null);
        },
        message: async (opts) => (await dialog.showMessageBox(main!.win, opts)).response,
      },
      window: {
        setTitle: (t) => main?.win.setTitle(t),
        setDocumentEdited: (e) => main?.win.setDocumentEdited(e),
        confirmClose: (ok) => main?.confirmClose(ok),
      },
    }),
  );

  const windowOptions = playBundle ? ({ page: 'player.html', title: basename(playBundle, '.fxa') } as const) : {};
  main = createMainWindow(windowOptions);

  app.on('activate', () => {
    if (BrowserWindow.getAllWindows().length === 0) main = createMainWindow(windowOptions);
  });
});

app.on('window-all-closed', () => {
  if (process.platform !== 'darwin') app.quit();
});
