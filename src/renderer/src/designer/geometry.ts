/**
 * Pure designer math. No DOM access, so it runs identically in jsdom and the app.
 * Coordinates are VFP pixels relative to the parent container.
 */
export interface Rect {
  left: number;
  top: number;
  width: number;
  height: number;
}

export type Handle = 'n' | 'ne' | 'e' | 'se' | 's' | 'sw' | 'w' | 'nw';
export const HANDLES: Handle[] = ['nw', 'n', 'ne', 'e', 'se', 's', 'sw', 'w'];

export type AlignKind = 'left' | 'right' | 'top' | 'bottom' | 'centerH' | 'centerV';
export type SizeKind = 'width' | 'height' | 'both';

export interface SnapOptions {
  grid: number;
  snap: boolean;
}

export function snap(value: number, grid: number, enabled = true): number {
  if (!enabled || grid <= 1) return Math.round(value);
  return Math.round(value / grid) * grid;
}

export function snapRect(r: Rect, grid: number, enabled = true): Rect {
  return {
    left: snap(r.left, grid, enabled),
    top: snap(r.top, grid, enabled),
    width: Math.max(0, snap(r.width, grid, enabled)),
    height: Math.max(0, snap(r.height, grid, enabled)),
  };
}

/** Rect from two corners in any order. */
export function normalizeRect(x1: number, y1: number, x2: number, y2: number): Rect {
  return { left: Math.min(x1, x2), top: Math.min(y1, y2), width: Math.abs(x2 - x1), height: Math.abs(y2 - y1) };
}

export function rectsIntersect(a: Rect, b: Rect): boolean {
  return a.left < b.left + b.width && b.left < a.left + a.width && a.top < b.top + b.height && b.top < a.top + a.height;
}

export function rectContains(outer: Rect, inner: Rect): boolean {
  return (
    inner.left >= outer.left &&
    inner.top >= outer.top &&
    inner.left + inner.width <= outer.left + outer.width &&
    inner.top + inner.height <= outer.top + outer.height
  );
}

export function unionRect(rects: Rect[]): Rect {
  if (rects.length === 0) return { left: 0, top: 0, width: 0, height: 0 };
  const left = Math.min(...rects.map((r) => r.left));
  const top = Math.min(...rects.map((r) => r.top));
  const right = Math.max(...rects.map((r) => r.left + r.width));
  const bottom = Math.max(...rects.map((r) => r.top + r.height));
  return { left, top, width: right - left, height: bottom - top };
}

/**
 * Moves a group by (dx, dy). Snapping is applied to the group's leading edge so that
 * relative offsets between the moved controls never change.
 */
export function computeMove(rects: Rect[], dx: number, dy: number, opts: SnapOptions): Rect[] {
  if (rects.length === 0) return [];
  const anchor = rects[0]!;
  const adx = snap(anchor.left + dx, opts.grid, opts.snap) - anchor.left;
  const ady = snap(anchor.top + dy, opts.grid, opts.snap) - anchor.top;
  return rects.map((r) => ({ ...r, left: r.left + adx, top: r.top + ady }));
}

export interface ResizeOptions extends SnapOptions {
  min?: { width: number; height: number };
  /** 'line' keeps a Line one-dimensional: the smaller dimension collapses to 0. */
  aspect?: 'line';
}

/** Resizes by dragging `handle` by (dx, dy). The opposite edge stays fixed. */
export function computeResize(rect: Rect, handle: Handle, dx: number, dy: number, opts: ResizeOptions): Rect {
  const min = opts.min ?? { width: 1, height: 1 };
  let left = rect.left;
  let top = rect.top;
  let right = rect.left + rect.width;
  let bottom = rect.top + rect.height;

  if (handle.includes('w')) left = snap(left + dx, opts.grid, opts.snap);
  if (handle.includes('e')) right = snap(right + dx, opts.grid, opts.snap);
  if (handle.includes('n')) top = snap(top + dy, opts.grid, opts.snap);
  if (handle.includes('s')) bottom = snap(bottom + dy, opts.grid, opts.snap);

  if (right - left < min.width) {
    if (handle.includes('w')) left = right - min.width;
    else right = left + min.width;
  }
  if (bottom - top < min.height) {
    if (handle.includes('n')) top = bottom - min.height;
    else bottom = top + min.height;
  }
  let out: Rect = { left, top, width: right - left, height: bottom - top };
  if (opts.aspect === 'line') {
    out = out.width >= out.height ? { ...out, height: 0 } : { ...out, width: 0 };
  }
  return out;
}

/** Ids whose rect intersects (default) or is fully inside the marquee. */
export function marqueeHits(items: { id: string; rect: Rect }[], marquee: Rect, mode: 'intersect' | 'contain' = 'intersect'): string[] {
  return items.filter((i) => (mode === 'contain' ? rectContains(marquee, i.rect) : rectsIntersect(marquee, i.rect))).map((i) => i.id);
}

/** Aligns every rect to the first one (the anchor), VFP Format > Align. */
export function alignRects(items: { id: string; rect: Rect }[], kind: AlignKind): Record<string, Rect> {
  const out: Record<string, Rect> = {};
  if (items.length < 2) return out;
  const a = items[0]!.rect;
  for (const { id, rect } of items.slice(1)) {
    let next: Rect;
    switch (kind) {
      case 'left':
        next = { ...rect, left: a.left };
        break;
      case 'right':
        next = { ...rect, left: a.left + a.width - rect.width };
        break;
      case 'top':
        next = { ...rect, top: a.top };
        break;
      case 'bottom':
        next = { ...rect, top: a.top + a.height - rect.height };
        break;
      case 'centerH':
        next = { ...rect, left: Math.round(a.left + a.width / 2 - rect.width / 2) };
        break;
      case 'centerV':
        next = { ...rect, top: Math.round(a.top + a.height / 2 - rect.height / 2) };
        break;
    }
    if (next.left !== rect.left || next.top !== rect.top) out[id] = next;
  }
  return out;
}

/** Makes every rect the same size as the first one. */
export function sameSize(items: { id: string; rect: Rect }[], kind: SizeKind): Record<string, Rect> {
  const out: Record<string, Rect> = {};
  if (items.length < 2) return out;
  const a = items[0]!.rect;
  for (const { id, rect } of items.slice(1)) {
    const next = {
      ...rect,
      width: kind === 'height' ? rect.width : a.width,
      height: kind === 'width' ? rect.height : a.height,
    };
    if (next.width !== rect.width || next.height !== rect.height) out[id] = next;
  }
  return out;
}

/** Distributes rects evenly between the first and last (by position) along an axis. */
export function distribute(items: { id: string; rect: Rect }[], axis: 'horizontal' | 'vertical'): Record<string, Rect> {
  const out: Record<string, Rect> = {};
  if (items.length < 3) return out;
  const key = axis === 'horizontal' ? 'left' : 'top';
  const size = axis === 'horizontal' ? 'width' : 'height';
  const sorted = [...items].sort((x, y) => x.rect[key] - y.rect[key]);
  const first = sorted[0]!.rect;
  const last = sorted[sorted.length - 1]!.rect;
  const total = last[key] + last[size] - first[key];
  const used = sorted.reduce((s, i) => s + i.rect[size], 0);
  const gap = (total - used) / (sorted.length - 1);
  let pos = first[key];
  for (const { id, rect } of sorted) {
    const v = Math.round(pos);
    if (v !== rect[key]) out[id] = { ...rect, [key]: v };
    pos += rect[size] + gap;
  }
  return out;
}
