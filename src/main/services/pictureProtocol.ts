/**
 * A `foxpic://` scheme for the pictures a running form displays.
 *
 * `file://` looks like the obvious choice and does not work: in development the renderer is
 * served over http, and Chromium refuses `file://` subresources from an http page, so images
 * would load in a packaged build and silently fail while developing. A custom scheme behaves
 * the same in both.
 *
 * URLs are `foxpic://local/<absolute path>`, and the path goes through the same guard as file
 * reads, so a form cannot display something the session was never given access to.
 */

import { net, protocol } from 'electron';
import { pathToFileURL } from 'node:url';
import type { PathGuard } from '../ipc/pathGuard';

export const PICTURE_SCHEME = 'foxpic';

/** Must run before the app is ready: privileged schemes are registered up front. */
export function registerPictureScheme(): void {
  protocol.registerSchemesAsPrivileged([
    { scheme: PICTURE_SCHEME, privileges: { standard: true, secure: true, supportFetchAPI: true, bypassCSP: false } },
  ]);
}

/** Turns the URL back into a path, undoing the `local` authority and percent-encoding. */
export function pathFromPictureUrl(url: string): string {
  const withoutScheme = url.replace(new RegExp(`^${PICTURE_SCHEME}://[^/]*/?`, 'i'), '');
  const decoded = decodeURIComponent(withoutScheme);
  // "C:/pics/fox.gif" arrives without a leading slash; a POSIX path keeps one
  return /^[A-Za-z]:/.test(decoded) ? decoded : `/${decoded}`.replace(/^\/+/, '/');
}

/** Serves picture files, refusing anything outside the directories the guard allows. */
export function handlePictureRequests(guard: PathGuard): void {
  protocol.handle(PICTURE_SCHEME, async (request) => {
    const path = pathFromPictureUrl(request.url);
    if (!guard.isAllowed(path)) {
      return new Response('Not allowed', { status: 403 });
    }
    try {
      return await net.fetch(pathToFileURL(path).toString());
    } catch {
      return new Response('Not found', { status: 404 });
    }
  });
}
