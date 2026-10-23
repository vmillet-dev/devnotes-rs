/** Case and accents aside, as the notes' search does in Rust: "hachage" finds "Hachage". */
export function foldForSearch(text: string): string {
  return text.normalize('NFD').replace(/\p{M}/gu, '').toLowerCase();
}

/**
 * How well a name answers a search, lower first, as Rust ranks the notes' titles: all of it,
 * its start, the start of one of its words, anywhere in it, then a match elsewhere.
 */
export function nameRank(name: string, query: string): number {
  const wanted = foldForSearch(query.trim());
  const folded = foldForSearch(name);
  if (folded === wanted) return 0;
  if (folded.startsWith(wanted)) return 1;
  if (folded.split(/[^\p{L}\p{N}]+/u).some((word) => word.startsWith(wanted))) return 2;
  return folded.includes(wanted) ? 3 : 4;
}

/** `items` best first by `nameRank`, the order they came in breaking ties. */
export function rankedByName<T>(items: readonly T[], query: string, nameOf: (item: T) => string): T[] {
  return items
    .map((item, at) => ({ item, at, rank: nameRank(nameOf(item), query) }))
    .sort((a, b) => a.rank - b.rank || a.at - b.at)
    .map(({ item }) => item);
}
