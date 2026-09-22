import type { MenuItem } from './schema';

export interface MenuLocation {
  item: MenuItem;
  parent: MenuItem | null; // null = top level
  siblings: MenuItem[];
  index: number;
}

export function findMenuItem(items: MenuItem[], id: string, parent: MenuItem | null = null): MenuLocation | undefined {
  for (let i = 0; i < items.length; i++) {
    const item = items[i]!;
    if (item.id === id) return { item, parent, siblings: items, index: i };
    if (item.children) {
      const found = findMenuItem(item.children, id, item);
      if (found) return found;
    }
  }
  return undefined;
}

export function walkMenu(items: MenuItem[], visit: (item: MenuItem, depth: number) => void, depth = 0): void {
  for (const item of items) {
    visit(item, depth);
    if (item.children) walkMenu(item.children, visit, depth + 1);
  }
}

/** Flattened, depth-first order with depth (what a tree view shows). */
export function flattenMenu(items: MenuItem[]): { item: MenuItem; depth: number }[] {
  const out: { item: MenuItem; depth: number }[] = [];
  walkMenu(items, (item, depth) => out.push({ item, depth }));
  return out;
}

export function removeMenuItem(items: MenuItem[], id: string): MenuItem | undefined {
  const loc = findMenuItem(items, id);
  if (!loc) return undefined;
  loc.siblings.splice(loc.index, 1);
  return loc.item;
}

/** Moves the item up or down among its siblings. */
export function moveMenuItem(items: MenuItem[], id: string, delta: -1 | 1): boolean {
  const loc = findMenuItem(items, id);
  if (!loc) return false;
  const target = loc.index + delta;
  if (target < 0 || target >= loc.siblings.length) return false;
  loc.siblings.splice(loc.index, 1);
  loc.siblings.splice(target, 0, loc.item);
  return true;
}

/** Makes the item the last child of its previous sibling (which becomes a submenu). */
export function indentMenuItem(items: MenuItem[], id: string): boolean {
  const loc = findMenuItem(items, id);
  if (!loc || loc.index === 0) return false;
  const prev = loc.siblings[loc.index - 1]!;
  loc.siblings.splice(loc.index, 1);
  prev.children ??= [];
  prev.result = { type: 'submenu' };
  prev.children.push(loc.item);
  return true;
}

/** Moves the item out of its parent, placing it right after the parent. */
export function outdentMenuItem(items: MenuItem[], id: string): boolean {
  const loc = findMenuItem(items, id);
  if (!loc || !loc.parent) return false;
  const parentLoc = findMenuItem(items, loc.parent.id)!;
  loc.siblings.splice(loc.index, 1);
  parentLoc.siblings.splice(parentLoc.index + 1, 0, loc.item);
  return true;
}
