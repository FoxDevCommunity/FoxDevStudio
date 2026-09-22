import { fireEvent } from '@testing-library/react';

export interface Pt {
  x: number;
  y: number;
}

const base = { pointerId: 1, pointerType: 'mouse', isPrimary: true, button: 0, buttons: 1 };

export function pointerDown(el: Element, at: Pt, init: Record<string, unknown> = {}) {
  fireEvent.pointerDown(el, { ...base, clientX: at.x, clientY: at.y, ...init });
}

export function pointerMove(at: Pt) {
  fireEvent.pointerMove(window, { ...base, clientX: at.x, clientY: at.y });
}

export function pointerUp(at: Pt) {
  fireEvent.pointerUp(window, { ...base, buttons: 0, clientX: at.x, clientY: at.y });
}

/** Full drag gesture: down on `el`, one or more moves, up. */
export function drag(el: Element, from: Pt, to: Pt, init: Record<string, unknown> = {}) {
  pointerDown(el, from, init);
  pointerMove({ x: from.x + (to.x - from.x) / 2, y: from.y + (to.y - from.y) / 2 });
  pointerMove(to);
  pointerUp(to);
}

/** A click without movement. */
export function click(el: Element, at: Pt = { x: 0, y: 0 }, init: Record<string, unknown> = {}) {
  pointerDown(el, at, init);
  pointerUp(at);
}
