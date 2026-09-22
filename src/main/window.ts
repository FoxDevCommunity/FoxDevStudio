import { BrowserWindow, shell } from 'electron';
import { join } from 'node:path';
import { writeFile } from 'node:fs/promises';

export interface MainWindow {
  win: BrowserWindow;
  /** Renderer answered the close request. */
  confirmClose(ok: boolean): void;
}

export interface WindowOptions {
  /** Renderer entry: the IDE by default, or the runtime player. */
  page?: 'index.html' | 'player.html';
  title?: string;
}

export function createMainWindow(options: WindowOptions = {}): MainWindow {
  const page = options.page ?? 'index.html';
  const win = new BrowserWindow({
    width: 1400,
    height: 900,
    minWidth: 900,
    minHeight: 600,
    show: false,
    title: options.title ?? 'FoxDev Studio',
    icon: join(__dirname, '../../resources/icon.png'),
    autoHideMenuBar: false,
    webPreferences: {
      preload: join(__dirname, '../preload/index.cjs'),
      contextIsolation: true,
      nodeIntegration: false,
      sandbox: true,
    },
  });

  let closeConfirmed = false;
  win.on('close', (e) => {
    if (closeConfirmed || process.env['FOXDEV_SCREENSHOT']) return;
    e.preventDefault();
    win.webContents.send('window:closeRequested');
  });

  // ready-to-show can fail to fire on some Wayland/WSLg setups; show after a fallback delay regardless
  let shown = false;
  const show = (reason: string) => {
    if (shown) return;
    shown = true;
    win.show();
    console.log(`[foxdev] window shown (${reason}), visible=${win.isVisible()}`);
    void maybeScreenshot(win);
  };
  win.on('ready-to-show', () => show('ready-to-show'));
  win.webContents.on('did-finish-load', () => console.log('[foxdev] renderer loaded'));
  win.webContents.on('did-fail-load', (_e, code, desc) => console.error(`[foxdev] renderer failed to load: ${code} ${desc}`));
  win.webContents.on('render-process-gone', (_e, details) => console.error(`[foxdev] renderer gone: ${details.reason}`));
  win.webContents.on('console-message', (_e, level, message) => {
    if (level >= 2) console.error(`[renderer] ${message}`);
  });
  setTimeout(() => show('fallback timer'), 1500);

  win.webContents.setWindowOpenHandler(({ url }) => {
    void shell.openExternal(url);
    return { action: 'deny' };
  });

  if (process.env['ELECTRON_RENDERER_URL']) {
    void win.loadURL(page === 'index.html' ? process.env['ELECTRON_RENDERER_URL'] : `${process.env['ELECTRON_RENDERER_URL']}/${page}`);
  } else {
    void win.loadFile(join(__dirname, '../renderer', page));
  }

  return {
    win,
    confirmClose(ok) {
      if (!ok) return;
      closeConfirmed = true;
      win.close();
    },
  };
}

/**
 * Dev aid: when FOXDEV_SCREENSHOT=/path/file.png is set, capture the window shortly
 * after it renders, write the PNG and quit. Lets an agent verify the real app without a screen.
 */
async function maybeScreenshot(win: BrowserWindow): Promise<void> {
  const target = process.env['FOXDEV_SCREENSHOT'];
  if (!target) return;
  const delay = Number(process.env['FOXDEV_SCREENSHOT_DELAY'] ?? 2500);
  await new Promise((r) => setTimeout(r, delay));
  const image = await win.webContents.capturePage();
  await writeFile(target, image.toPNG());
  win.close();
}
