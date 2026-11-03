/**
 * The order `Array.prototype.sort` gives strings without a comparator, UTF-16 code units,
 * spelled out. Not `localeCompare`: any stable order serves the `equal` it feeds.
 */
export function byCodeUnit(a: string, b: string): number {
  if (a < b) return -1;
  if (a > b) return 1;
  return 0;
}
