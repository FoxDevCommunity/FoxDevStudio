import type { ControlNode, FormNode } from './schema';

/** VFP object names are case-insensitive. */
export function sameName(a: string, b: string): boolean {
  return a.toLowerCase() === b.toLowerCase();
}

/** All names in the form (form name included), lower-cased. */
export function collectNames(form: FormNode): Set<string> {
  const names = new Set<string>([form.name.toLowerCase()]);
  const visit = (nodes: ControlNode[] | undefined) => {
    for (const n of nodes ?? []) {
      names.add(n.name.toLowerCase());
      visit(n.children);
    }
  };
  visit(form.children);
  return names;
}

/** Next free name for a prefix: Text1, Text2... Mutates nothing; `taken` is read only. */
export function uniqueName(prefix: string, taken: Set<string> | Iterable<string>): string {
  const set = new Set([...taken].map((s) => s.toLowerCase()));
  for (let i = 1; ; i++) {
    const candidate = `${prefix}${i}`;
    if (!set.has(candidate.toLowerCase())) return candidate;
  }
}

/** Returns `base` if free, otherwise base with a numeric suffix bumped until free (Command1 -> Command2). */
export function dedupeName(base: string, taken: Set<string>): string {
  if (![...taken].some((t) => t.toLowerCase() === base.toLowerCase())) return base;
  const m = /^(.*?)(\d+)$/.exec(base);
  const prefix = m ? m[1]! : base;
  return uniqueName(prefix, taken);
}

export function isValidName(name: string): boolean {
  return /^[A-Za-z_][A-Za-z0-9_]{0,127}$/.test(name);
}
