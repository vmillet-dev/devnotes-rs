import { describe, expect, it } from 'vitest';
import { DEFAULT_ARRANGEMENT, naturalOrder, readArrangement } from './arrangement.model';

describe('readArrangement', () => {
  it('starts from the newest edit, grouped by date, pinned notes first', () => {
    expect(readArrangement(null)).toEqual(DEFAULT_ARRANGEMENT);
  });

  it('reads back what was written', () => {
    const written = {
      order: { key: 'title', direction: 'ascending' },
      grouping: 'format',
      pinnedFirst: false,
    };

    expect(readArrangement(JSON.stringify(written))).toEqual(written);
  });

  /** A hand-edited file, or one from a newer build: what it does not name stays default. */
  it('keeps the default for whatever it cannot read', () => {
    expect(readArrangement('not json')).toEqual(DEFAULT_ARRANGEMENT);
    expect(
      readArrangement(JSON.stringify({ order: { key: 'colour' }, grouping: 'priority', pinnedFirst: 'yes' })),
    ).toEqual({ ...DEFAULT_ARRANGEMENT, grouping: 'priority' });
  });
});

describe('naturalOrder', () => {
  it('puts the newest and the most pressing first, and words from A to Z', () => {
    expect(naturalOrder('created').direction).toBe('descending');
    expect(naturalOrder('priority').direction).toBe('descending');
    expect(naturalOrder('title').direction).toBe('ascending');
    expect(naturalOrder('format').direction).toBe('ascending');
  });
});
