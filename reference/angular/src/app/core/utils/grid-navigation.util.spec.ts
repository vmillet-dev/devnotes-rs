import { describe, expect, it } from 'vitest';
import { CardBox, nextFocusIndex } from './grid-navigation.util';

function grid(columns: number, count: number): CardBox[] {
  return Array.from({ length: count }, (_, index) => ({
    top: Math.floor(index / columns) * 160,
    left: (index % columns) * 200,
  }));
}

describe('nextFocusIndex', () => {
  it('walks the flat order sideways', () => {
    const boxes = grid(3, 6);

    expect(nextFocusIndex(boxes, 1, 'next')).toBe(2);
    expect(nextFocusIndex(boxes, 1, 'prev')).toBe(0);
  });

  it('stops at the edges instead of wrapping around', () => {
    const boxes = grid(3, 6);

    expect(nextFocusIndex(boxes, 0, 'prev')).toBe(0);
    expect(nextFocusIndex(boxes, 5, 'next')).toBe(5);
    expect(nextFocusIndex(boxes, 1, 'up')).toBe(1);
    expect(nextFocusIndex(boxes, 4, 'down')).toBe(4);
  });

  it('moves a whole row at a time, keeping the column', () => {
    const boxes = grid(3, 6);

    expect(nextFocusIndex(boxes, 1, 'down')).toBe(4);
    expect(nextFocusIndex(boxes, 5, 'up')).toBe(2);
  });

  it('lands on the nearest card when the target row is shorter', () => {
    const boxes = grid(3, 5);

    expect(nextFocusIndex(boxes, 2, 'down')).toBe(4);
  });

  it('tolerates a few pixels of difference within a row', () => {
    // Two cards on one row do not always share a pixel-exact `top`: with no tolerance,
    // each would form a row of its own.
    const boxes: CardBox[] = [
      { top: 0, left: 0 },
      { top: 2, left: 200 },
      { top: 160, left: 0 },
    ];

    expect(nextFocusIndex(boxes, 0, 'down')).toBe(2);
    expect(nextFocusIndex(boxes, 1, 'down')).toBe(2);
  });

  it('enters through the first card when nothing is focused yet', () => {
    expect(nextFocusIndex(grid(3, 4), -1, 'down')).toBe(0);
  });

  it('reports no target on an empty grid', () => {
    expect(nextFocusIndex([], 0, 'next')).toBe(-1);
  });
});
