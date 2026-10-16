import { describe, expect, it } from 'vitest';
import { columnCount, stackInColumns } from './columns.util';

const height = (item: number): number => item;

describe('columnCount', () => {
  it('fits as many columns as the width holds, gaps included', () => {
    expect(columnCount(1700, 260, 14)).toBe(6);
    expect(columnCount(534, 260, 14)).toBe(2);
    expect(columnCount(533, 260, 14)).toBe(1);
  });

  it('keeps one column however narrow, or unmeasured', () => {
    expect(columnCount(0, 260, 14)).toBe(1);
  });
});

describe('stackInColumns', () => {
  /** The tools home at 2000 px: CSS filled four of six columns and left two empty. */
  it('uses every column when one tall item sets the height', () => {
    const stacks = stackInColumns([5, 5, 5, 5, 4, 6, 10], height, 6);

    expect(stacks).toHaveLength(6);
    expect(Math.max(...stacks.map((stack) => stack.reduce((sum, item) => sum + item, 0)))).toBe(10);
    expect(stacks.flat()).toEqual([5, 5, 5, 5, 4, 6, 10]);
  });

  it('keeps the reading order, read down each column', () => {
    expect(stackInColumns(['a', 'b', 'c', 'd'], () => 1, 2)).toEqual([
      ['a', 'b'],
      ['c', 'd'],
    ]);
  });

  it('makes the tallest column as short as it can be', () => {
    expect(stackInColumns([5, 5, 5, 5, 4, 6, 10], height, 3)).toEqual([[5, 5, 5], [5, 4, 6], [10]]);
  });

  it('gives each item its own column when there are more columns than items', () => {
    expect(stackInColumns([3, 1], height, 6)).toEqual([[3], [1]]);
  });

  it('answers no column for no item, and one for a single column', () => {
    expect(stackInColumns([], height, 4)).toEqual([]);
    expect(stackInColumns([1, 2, 3], height, 1)).toEqual([[1, 2, 3]]);
  });
});
