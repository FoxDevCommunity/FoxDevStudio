import type { ControlNode, FormNode } from './schema';

/** Anything that owns a child list: the form itself or a container control. */
export type ParentNode = FormNode | ControlNode;

export interface NodeLocation {
  node: ControlNode;
  parent: ParentNode;
  index: number;
  /** Ancestor chain, outermost first, form excluded. */
  ancestors: ControlNode[];
}

export function isFormNode(n: ParentNode): n is FormNode {
  return !('type' in n);
}

/** Depth-first walk over every control. Return false from the visitor to stop. */
export function walk(form: FormNode, visit: (loc: NodeLocation) => boolean | void): void {
  const rec = (parent: ParentNode, ancestors: ControlNode[]): boolean => {
    const children = parent.children ?? [];
    for (let i = 0; i < children.length; i++) {
      const node = children[i]!;
      if (visit({ node, parent, index: i, ancestors }) === false) return false;
      if (node.children && !rec(node, [...ancestors, node])) return false;
    }
    return true;
  };
  rec(form, []);
}

export function findLocation(form: FormNode, id: string): NodeLocation | undefined {
  let found: NodeLocation | undefined;
  walk(form, (loc) => {
    if (loc.node.id === id) {
      found = loc;
      return false;
    }
  });
  return found;
}

export function findNode(form: FormNode, id: string): ControlNode | undefined {
  return findLocation(form, id)?.node;
}

export function findByName(form: FormNode, name: string): ControlNode | undefined {
  const lower = name.toLowerCase();
  let found: ControlNode | undefined;
  walk(form, ({ node }) => {
    if (node.name.toLowerCase() === lower) {
      found = node;
      return false;
    }
  });
  return found;
}

export function allNodes(form: FormNode): ControlNode[] {
  const out: ControlNode[] = [];
  walk(form, ({ node }) => void out.push(node));
  return out;
}

/** True if `id` is `ancestorId` or nested anywhere under it. */
export function isDescendantOf(form: FormNode, id: string, ancestorId: string): boolean {
  const loc = findLocation(form, id);
  if (!loc) return false;
  return id === ancestorId || loc.ancestors.some((a) => a.id === ancestorId);
}

/** Resolves a parent by id; `undefined`/null means the form. */
export function resolveParent(form: FormNode, parentId: string | null | undefined): ParentNode | undefined {
  return parentId == null ? form : findNode(form, parentId);
}

// ---- mutators: operate in place (works on immer drafts and on cloned docs) ----

export function insertChild(parent: ParentNode, node: ControlNode, index?: number): void {
  parent.children ??= [];
  const i = index === undefined ? parent.children.length : Math.max(0, Math.min(index, parent.children.length));
  parent.children.splice(i, 0, node);
}

export function removeNode(form: FormNode, id: string): ControlNode | undefined {
  const loc = findLocation(form, id);
  if (!loc) return undefined;
  loc.parent.children!.splice(loc.index, 1);
  return loc.node;
}

/** Moves a node to a new parent (or the form when parentId is null). Refuses cycles. */
export function reparent(form: FormNode, id: string, parentId: string | null, index?: number): boolean {
  if (parentId !== null && isDescendantOf(form, parentId, id)) return false;
  const target = resolveParent(form, parentId);
  if (!target) return false;
  const node = removeNode(form, id);
  if (!node) return false;
  insertChild(target, node, index);
  return true;
}

/** Z-order within the parent's child list. Later = drawn on top. */
export function reorder(form: FormNode, id: string, where: 'front' | 'back' | 'forward' | 'backward'): boolean {
  const loc = findLocation(form, id);
  if (!loc) return false;
  const list = loc.parent.children!;
  list.splice(loc.index, 1);
  const last = list.length;
  const target =
    where === 'front' ? last : where === 'back' ? 0 : where === 'forward' ? Math.min(last, loc.index + 1) : Math.max(0, loc.index - 1);
  list.splice(target, 0, loc.node);
  return true;
}

/** Deep clone with fresh ids (for copy/paste). Returns the map old id -> new id as well. */
export function cloneSubtree(node: ControlNode, newId: () => string): ControlNode {
  return {
    ...node,
    id: newId(),
    props: { ...node.props },
    methods: { ...node.methods },
    children: node.children?.map((c) => cloneSubtree(c, newId)),
  };
}
