/** Element by element, each by `same`: `===` compares two arrays by identity. */
export function sameArray<T>(
  a: readonly T[],
  b: readonly T[],
  same: (x: T, y: T) => boolean = Object.is,
): boolean {
  return a.length === b.length && a.every((value, index) => same(value, b[index] as T));
}

/**
 * One comparator per field. A mapped record rather than a chain of `&&`: a field added
 * to `T` stops compiling until it says how it compares, where a chain would treat it as
 * always equal and a `resource` keyed on it would never re-run.
 */
export type Comparators<T> = { readonly [K in keyof T]-?: (a: T[K], b: T[K]) => boolean };

/** Equal when every field is, each by its own comparator. */
export function sameBy<T extends object>(comparators: Comparators<T>): (a: T, b: T) => boolean {
  const fields = Object.keys(comparators) as (keyof T)[];

  return (a, b) =>
    fields.every((field) => (comparators[field] as (x: unknown, y: unknown) => boolean)(a[field], b[field]));
}
