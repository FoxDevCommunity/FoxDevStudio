/** VFP RGB() colors: r + g*256 + b*65536 stored as a plain integer. */

export function rgb(r: number, g: number, b: number): number {
  return (clamp(r) | (clamp(g) << 8) | (clamp(b) << 16)) >>> 0;
}

export function toRgb(color: number): { r: number; g: number; b: number } {
  const n = color >>> 0;
  return { r: n & 0xff, g: (n >> 8) & 0xff, b: (n >> 16) & 0xff };
}

export function toHex(color: number): string {
  const { r, g, b } = toRgb(color);
  return '#' + [r, g, b].map((v) => v.toString(16).padStart(2, '0')).join('');
}

export function fromHex(hex: string): number | null {
  const m = /^#?([0-9a-f]{6})$/i.exec(hex.trim());
  if (!m) return null;
  const n = parseInt(m[1]!, 16);
  return rgb((n >> 16) & 0xff, (n >> 8) & 0xff, n & 0xff);
}

/** Formats as VFP source text: RGB(255,0,0). */
export function formatColor(color: number): string {
  const { r, g, b } = toRgb(color);
  return `RGB(${r},${g},${b})`;
}

/** Accepts "RGB(r,g,b)", "#rrggbb", "r,g,b" or a plain integer string. */
export function parseColor(text: string): number | null {
  const t = text.trim();
  if (t === '') return null;
  const m = /^rgb\s*\(\s*(\d{1,3})\s*,\s*(\d{1,3})\s*,\s*(\d{1,3})\s*\)$/i.exec(t) ?? /^(\d{1,3})\s*,\s*(\d{1,3})\s*,\s*(\d{1,3})$/.exec(t);
  if (m) return rgb(Number(m[1]), Number(m[2]), Number(m[3]));
  if (t.startsWith('#')) return fromHex(t);
  if (/^-?\d+$/.test(t)) {
    const n = Number(t);
    return n >= 0 && n <= 0xffffff ? n : null;
  }
  return null;
}

function clamp(v: number): number {
  return Math.max(0, Math.min(255, Math.round(v))) & 0xff;
}
