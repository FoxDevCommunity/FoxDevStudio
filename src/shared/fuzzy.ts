/**
 * Subsequence matching, the kind a "go to file" box does: `pgf` finds `pfsam1.fxf` and
 * `pgframe/msgbox.fxf` without either being typed in full.
 *
 * Scoring exists so the obvious answer comes first when a query matches many things. Runs of
 * adjacent characters score highest, then matches at a word boundary, then anything else; an
 * earlier first match breaks a tie. That is enough to put `db/order.fxf` above
 * `controls/pgframe/msgbox.fxf` for `ord`, without pretending to be a ranking engine.
 */

/** A matched position in the candidate, so the caller can show what matched. */
export interface FuzzyMatch {
  score: number;
  /** Indices into the candidate, ascending. */
  positions: number[];
}

const ADJACENT = 8;
const BOUNDARY = 6;
const PLAIN = 1;
/** Taken off the score for each character skipped before the first match. */
const LEADING_SKIP = 0.1;

/**
 * Matches `query` against `text` as a subsequence, case-insensitively. `null` when a character of
 * the query is not there at all.
 *
 * An empty query matches everything with score 0, which is what lets a search box start empty.
 */
export function fuzzyMatch(query: string, text: string): FuzzyMatch | null {
  if (query === '') return { score: 0, positions: [] };

  const q = query.toLowerCase();
  const t = text.toLowerCase();
  const positions: number[] = [];
  let score = 0;
  let at = 0;

  for (const ch of q) {
    const found = t.indexOf(ch, at);
    if (found < 0) return null;
    score += pointsFor(text, found, positions[positions.length - 1]);
    positions.push(found);
    at = found + 1;
  }
  return { score: score - (positions[0] ?? 0) * LEADING_SKIP, positions };
}

function pointsFor(text: string, at: number, previous: number | undefined): number {
  if (previous !== undefined && at === previous + 1) return ADJACENT;
  return isBoundary(text, at) ? BOUNDARY : PLAIN;
}

/** The start of the string, or of a word: after a separator, or an upper-case run beginning. */
function isBoundary(text: string, at: number): boolean {
  if (at === 0) return true;
  const before = text[at - 1] ?? '';
  if (/[^A-Za-z0-9]/.test(before)) return true;
  const here = text[at] ?? '';
  return before === before.toLowerCase() && here !== here.toLowerCase();
}

/**
 * Filters and orders `items` by how well `key` matches the query. Ties keep the original order,
 * so a list that was sorted stays sorted among equals.
 */
export function fuzzyFilter<T>(query: string, items: readonly T[], key: (item: T) => string): { item: T; match: FuzzyMatch }[] {
  const scored = items
    .map((item, index) => ({ item, index, match: fuzzyMatch(query, key(item)) }))
    .filter((r): r is { item: T; index: number; match: FuzzyMatch } => r.match !== null);

  scored.sort((a, b) => b.match.score - a.match.score || a.index - b.index);
  return scored.map(({ item, match }) => ({ item, match }));
}
