/**
 * Scores `text` against `query` as an in-order subsequence match.
 * Higher is better; `null` means no match. Consecutive and word-start hits
 * score more, so `chk` ranks `check_password` above `cache_lookup_key`.
 *
 * @example fuzzyScore("chk", "check_password") !== null // true
 * @example fuzzyScore("zz", "main") // null
 */
export function fuzzyScore(query: string, text: string): number | null {
  const q = query.toLowerCase();
  const t = text.toLowerCase();
  if (!q) return 0;
  let score = 0;
  let ti = 0;
  let prev = -2;
  for (const ch of q) {
    const at = t.indexOf(ch, ti);
    if (at < 0) return null;
    score += at === prev + 1 ? 3 : 1;
    if (at === 0 || /[_\-.@\s]/.test(t[at - 1] ?? "")) score += 2;
    prev = at;
    ti = at + 1;
  }
  return score - t.length * 0.01;
}

/**
 * Filters and sorts `items` by fuzzy match of `key(item)` against `query`.
 * @example fuzzyFilter("mn", ["main", "menu", "x"], (s) => s) // ["main", "menu"]
 */
export function fuzzyFilter<T>(query: string, items: T[], key: (t: T) => string, limit = 50): T[] {
  const scored: { item: T; score: number }[] = [];
  for (const item of items) {
    const score = fuzzyScore(query, key(item));
    if (score !== null) scored.push({ item, score });
  }
  scored.sort((a, b) => b.score - a.score);
  return scored.slice(0, limit).map((s) => s.item);
}
