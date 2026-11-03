/** As many columns of at least `minWidth` as `width` holds, `gap` between each; never none. */
export function columnCount(width: number, minWidth: number, gap: number): number {
  return Math.max(1, Math.floor((width + gap) / (minWidth + gap)));
}

/**
 * The items in reading order, cut into consecutive stacks, one per column, the tallest as short
 * as it can be. CSS `columns` finds that height and then fills from the left, which left the
 * last columns empty whenever one tall item set it.
 */
export function stackInColumns<T>(
  items: readonly T[],
  heightOf: (item: T) => number,
  columns: number,
): T[][] {
  const count = Math.min(Math.max(1, columns), items.length);
  if (count === 0) return [];

  const heights = items.map(heightOf);
  const below = (from: number, to: number): number =>
    heights.slice(from, to).reduce((sum, height) => sum + height, 0);

  // tallest[k][i]: the tallest column when the first `i` items fill `k` columns.
  const tallest: number[][] = [[0, ...heights.map(() => Infinity)]];
  const cut: number[][] = [[]];
  for (let k = 1; k <= count; k += 1) {
    tallest.push([Infinity]);
    cut.push([0]);
    for (let i = 1; i <= items.length; i += 1) {
      let best = Infinity;
      let at = k - 1;
      for (let from = k - 1; from < i; from += 1) {
        const candidate = Math.max(tallest[k - 1]![from]!, below(from, i));
        if (candidate < best) {
          best = candidate;
          at = from;
        }
      }
      tallest[k]!.push(best);
      cut[k]!.push(at);
    }
  }

  const stacks: T[][] = [];
  let end = items.length;
  for (let k = count; k > 0; k -= 1) {
    const from = cut[k]![end]!;
    stacks.unshift(items.slice(from, end));
    end = from;
  }
  return stacks;
}
