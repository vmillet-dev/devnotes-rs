/**
 * The order `Array.prototype.sort` gives strings when it is handed no comparator — UTF-16
 * code units — spelled out, so no reader has to wonder whether it was meant.
 *
 * ⚠️ Not `localeCompare`: where this sorts the key a `resource`'s `equal` compares against
 * its previous value, any stable order does, and changing it gains nothing.
 */
export function byCodeUnit(a: string, b: string): number {
  if (a < b) return -1;
  if (a > b) return 1;
  return 0;
}
