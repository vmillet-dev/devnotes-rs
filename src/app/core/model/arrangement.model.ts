import type { Grouping, NoteOrder, SortDirection, SortKey } from './note.model';

/** How the date view is laid out: the library's choice, remembered between launches. */
export interface Arrangement {
  readonly order: NoteOrder;
  readonly grouping: Grouping;
  readonly pinnedFirst: boolean;
}

export const SORT_KEYS: readonly SortKey[] = ['modified', 'created', 'priority', 'format', 'title'];
export const SORT_DIRECTIONS: readonly SortDirection[] = ['descending', 'ascending'];
export const GROUPINGS: readonly Grouping[] = ['date', 'priority', 'format', 'none'];

export const DEFAULT_ARRANGEMENT: Arrangement = {
  order: { key: 'modified', direction: 'descending' },
  grouping: 'date',
  pinnedFirst: true,
};

/** The way a key reads first: the newest or the most pressing on top, words from A to Z. */
export function naturalOrder(key: SortKey): NoteOrder {
  return { key, direction: key === 'title' || key === 'format' ? 'ascending' : 'descending' };
}

function oneOf<T extends string>(allowed: readonly T[], value: unknown, fallback: T): T {
  return allowed.find((each) => each === value) ?? fallback;
}

/** A preference file is trusted no further than it reads: what it does not name stays default. */
export function readArrangement(stored: string | null): Arrangement {
  let value: Partial<Record<'order' | 'grouping' | 'pinnedFirst', unknown>>;
  try {
    value = stored === null ? {} : (JSON.parse(stored) as typeof value);
  } catch {
    return DEFAULT_ARRANGEMENT;
  }

  const order = (value.order ?? {}) as Partial<Record<keyof NoteOrder, unknown>>;
  const fallback = DEFAULT_ARRANGEMENT;
  return {
    order: {
      key: oneOf(SORT_KEYS, order.key, fallback.order.key),
      direction: oneOf(SORT_DIRECTIONS, order.direction, fallback.order.direction),
    },
    grouping: oneOf(GROUPINGS, value.grouping, fallback.grouping),
    pinnedFirst: typeof value.pinnedFirst === 'boolean' ? value.pinnedFirst : fallback.pinnedFirst,
  };
}
