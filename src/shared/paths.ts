/**
 * Small, pure path helpers that work for both '/' and '\' separated paths, so the renderer
 * never needs an IPC round trip for path arithmetic. Output always uses forward slashes
 * for relative paths (the project file format) and preserves the input style otherwise.
 */
const SEP_RE = /[\\/]+/;

export function isAbsolute(p: string): boolean {
  return p.startsWith('/') || /^[A-Za-z]:[\\/]/.test(p) || p.startsWith('\\\\');
}

function sepOf(p: string): string {
  return p.includes('\\') && !p.includes('/') ? '\\' : '/';
}

function splitParts(p: string): string[] {
  return p.split(SEP_RE).filter((s) => s.length > 0);
}

export function normalize(p: string): string {
  const sep = sepOf(p);
  const abs = isAbsolute(p);
  const drive = /^[A-Za-z]:/.exec(p)?.[0];
  const parts: string[] = [];
  for (const part of splitParts(drive ? p.slice(2) : p)) {
    if (part === '.') continue;
    if (part === '..') {
      if (parts.length && parts[parts.length - 1] !== '..') parts.pop();
      else if (!abs) parts.push('..');
      continue;
    }
    parts.push(part);
  }
  const body = parts.join(sep);
  if (drive) return drive + sep + body;
  if (abs) return sep + body;
  return body || '.';
}

export function dirname(p: string): string {
  const n = normalize(p);
  const idx = Math.max(n.lastIndexOf('/'), n.lastIndexOf('\\'));
  if (idx < 0) return '.';
  if (idx === 0) return n.slice(0, 1);
  if (/^[A-Za-z]:$/.test(n.slice(0, idx))) return n.slice(0, idx + 1);
  return n.slice(0, idx);
}

export function basename(p: string, stripExt = false): string {
  const parts = splitParts(p);
  const last = parts[parts.length - 1] ?? '';
  if (!stripExt) return last;
  const dot = last.lastIndexOf('.');
  return dot > 0 ? last.slice(0, dot) : last;
}

export function extname(p: string): string {
  const b = basename(p);
  const dot = b.lastIndexOf('.');
  return dot > 0 ? b.slice(dot) : '';
}

export function join(dir: string, ...rest: string[]): string {
  const sep = sepOf(dir);
  return normalize([dir, ...rest].join(sep));
}

/** Relative path from directory `fromDir` to `to`, with forward slashes. Falls back to `to` when on different roots. */
export function relative(fromDir: string, to: string): string {
  const a = splitParts(normalize(fromDir));
  const b = splitParts(normalize(to));
  const rootA = isAbsolute(fromDir) ? (/^[A-Za-z]:/.exec(fromDir)?.[0]?.toLowerCase() ?? '/') : '';
  const rootB = isAbsolute(to) ? (/^[A-Za-z]:/.exec(to)?.[0]?.toLowerCase() ?? '/') : '';
  if (rootA !== rootB) return to.replace(/\\/g, '/');
  if (rootA && rootA !== '/') {
    a.shift();
    b.shift();
  }
  let i = 0;
  while (i < a.length && i < b.length && a[i]!.toLowerCase() === b[i]!.toLowerCase()) i++;
  const ups = a.slice(i).map(() => '..');
  return [...ups, ...b.slice(i)].join('/') || '.';
}

/** Resolves `rel` against `dir` unless it is already absolute. */
export function resolveFrom(dir: string, rel: string): string {
  return isAbsolute(rel) ? normalize(rel) : join(dir, rel);
}

/** True when `p` equals `root` or lives under it (case-insensitive on drive letters). */
export function isInside(root: string, p: string): boolean {
  const r = normalize(root).replace(/\\/g, '/').replace(/\/$/, '');
  const q = normalize(p).replace(/\\/g, '/');
  const rl = r.toLowerCase();
  const ql = q.toLowerCase();
  return ql === rl || ql.startsWith(rl + '/');
}
