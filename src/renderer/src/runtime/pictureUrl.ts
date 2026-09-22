/**
 * Turning a VFP `Picture` path into something the renderer can display.
 *
 * VFP paths are Windows paths, often relative to the project, and often built at runtime from
 * `HOME()`, which points at Visual FoxPro's own installation. That last case cannot resolve
 * here at all, so a missing picture has to fail visibly rather than silently.
 */

import { isAbsolute, join } from '@shared/paths';
import { useProjectStore } from '../stores/projectStore';

/**
 * A URL the renderer can show, or null when there is nothing to show. Local files go through
 * the `foxpic://` scheme rather than `file://`, because Chromium blocks file subresources on
 * the http page the dev server serves, which would work when packaged and fail while developing.
 */
export function pictureUrl(picture: unknown): string | null {
  const path = typeof picture === 'string' ? picture.trim() : '';
  if (!path) return null;
  // a data URI or an http(s) source is already a URL
  if (/^(data:|https?:|file:)/i.test(path)) return path;

  const absolute = isAbsolute(path) ? path : resolveAgainstProject(path);
  if (!absolute) return null;
  const normalised = absolute.replace(/\\/g, '/').replace(/^\/+/, '');
  return `foxpic://local/${encodeURIComponent(normalised)}`;
}

function resolveAgainstProject(path: string): string | null {
  const dir = useProjectStore.getState().dir();
  return dir ? join(dir, path) : null;
}

/** What to tell the user when a picture cannot be loaded. */
export function pictureProblem(picture: unknown): string {
  const path = typeof picture === 'string' ? picture.trim() : '';
  if (!path) return 'no picture set';
  if (!isAbsolute(path) && !useProjectStore.getState().dir()) {
    return `cannot resolve "${path}": it is a relative path and no project is open`;
  }
  return `cannot load "${path}": the file does not exist`;
}
