import { describe, expect, it } from 'vitest';
import { alignRects, computeMove, computeResize, distribute, marqueeHits, normalizeRect, sameSize, snap, snapRect, unionRect } from '@renderer/designer/geometry';

const r = (left: number, top: number, width: number, height: number) => ({ left, top, width, height });

describe('geometry', () => {
  it('snaps values and rects', () => {
    expect(snap(11, 8)).toBe(8);
    expect(snap(12, 8)).toBe(16);
    expect(snap(11, 8, false)).toBe(11);
    expect(snap(11.4, 1)).toBe(11);
    expect(snapRect(r(3, 13, 21, 9), 8)).toEqual(r(0, 16, 24, 8));
    expect(normalizeRect(50, 60, 10, 20)).toEqual(r(10, 20, 40, 40));
    expect(unionRect([r(0, 0, 10, 10), r(20, 30, 5, 5)])).toEqual(r(0, 0, 25, 35));
  });

  it('moves groups keeping relative offsets; snap applies to the anchor', () => {
    const rects = [r(16, 16, 84, 27), r(121, 20, 84, 27)];
    expect(computeMove(rects, 23, 17, { grid: 8, snap: true })).toEqual([r(40, 32, 84, 27), r(145, 36, 84, 27)]);
    expect(computeMove(rects, 23, 17, { grid: 8, snap: false })).toEqual([r(39, 33, 84, 27), r(144, 37, 84, 27)]);
    expect(computeMove([], 1, 1, { grid: 8, snap: true })).toEqual([]);
  });

  it('resizes from any handle keeping the opposite edge fixed', () => {
    const base = r(16, 16, 80, 40);
    const o = { grid: 8, snap: true };
    expect(computeResize(base, 'se', 23, 17, o)).toEqual(r(16, 16, 104, 56));
    expect(computeResize(base, 'nw', -7, -9, o)).toEqual(r(8, 8, 88, 48));
    expect(computeResize(base, 'e', 10, 999, o)).toEqual(r(16, 16, 88, 40));
    expect(computeResize(base, 's', 999, -8, o)).toEqual(r(16, 16, 80, 32));
    // never smaller than min; the dragged edge gives way
    expect(computeResize(base, 'w', 200, 0, { ...o, min: { width: 8, height: 8 } })).toEqual(r(88, 16, 8, 40));
    expect(computeResize(base, 'n', 0, 200, { ...o, min: { width: 8, height: 8 } })).toEqual(r(16, 48, 80, 8));
    // lines stay one-dimensional
    expect(computeResize(r(0, 0, 100, 0), 'se', 0, 30, { grid: 1, snap: false, aspect: 'line' })).toEqual(r(0, 0, 100, 0));
    expect(computeResize(r(0, 0, 100, 0), 'se', -95, 30, { grid: 1, snap: false, aspect: 'line' })).toEqual(r(0, 0, 0, 30));
  });

  it('selects by marquee', () => {
    const items = [
      { id: 'a', rect: r(0, 0, 10, 10) },
      { id: 'b', rect: r(20, 20, 10, 10) },
      { id: 'c', rect: r(100, 100, 10, 10) },
    ];
    expect(marqueeHits(items, r(5, 5, 20, 20))).toEqual(['a', 'b']);
    expect(marqueeHits(items, r(5, 5, 20, 20), 'contain')).toEqual([]);
    expect(marqueeHits(items, r(0, 0, 31, 31), 'contain')).toEqual(['a', 'b']);
    expect(marqueeHits(items, r(10, 10, 10, 10))).toEqual([]); // touching edges do not count
  });

  it('aligns, sizes and distributes relative to the first item', () => {
    const items = [
      { id: 'a', rect: r(10, 10, 50, 20) },
      { id: 'b', rect: r(30, 40, 30, 10) },
      { id: 'c', rect: r(90, 15, 20, 40) },
    ];
    expect(alignRects(items, 'left')).toEqual({ b: r(10, 40, 30, 10), c: r(10, 15, 20, 40) });
    expect(alignRects(items, 'right')).toEqual({ c: r(40, 15, 20, 40) }); // b already flush
    expect(alignRects(items, 'top')).toEqual({ b: r(30, 10, 30, 10), c: r(90, 10, 20, 40) });
    expect(alignRects(items, 'bottom')).toEqual({ b: r(30, 20, 30, 10), c: r(90, -10, 20, 40) });
    expect(alignRects(items, 'centerH')).toEqual({ b: r(20, 40, 30, 10), c: r(25, 15, 20, 40) });
    expect(alignRects(items, 'centerV')).toEqual({ b: r(30, 15, 30, 10), c: r(90, 0, 20, 40) });
    expect(alignRects(items.slice(0, 1), 'left')).toEqual({});
    expect(sameSize(items, 'width')).toEqual({ b: r(30, 40, 50, 10), c: r(90, 15, 50, 40) });
    expect(sameSize(items, 'both')).toEqual({ b: r(30, 40, 50, 20), c: r(90, 15, 50, 20) });
    expect(distribute(items, 'horizontal')).toEqual({ b: r(60, 40, 30, 10) }); // 100px span, 100px used: no gaps
    expect(distribute(items.slice(0, 2), 'horizontal')).toEqual({});
  });
});
