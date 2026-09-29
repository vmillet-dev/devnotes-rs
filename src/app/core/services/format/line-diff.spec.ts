import { describe, expect, it } from 'vitest';
import { changedLines } from './line-diff';

describe('changedLines', () => {
  it('says nothing about identical texts', () => {
    expect(changedLines('a\nb', 'a\nb')).toEqual([]);
  });

  it('marks a line rewritten in place', () => {
    expect(changedLines('a\nb\nc', 'a\nB\nc')).toEqual([1]);
  });

  it('marks the lines a statement was split into, and nothing after them', () => {
    expect(changedLines('keep\nf(a, b)\nend', 'keep\nf(\n  a,\n  b,\n)\nend')).toEqual([1, 2, 3, 4]);
  });

  it('keeps the lines that survive between changed ones', () => {
    expect(changedLines('x\nsame\ny', 'X\nsame\nY')).toEqual([0, 2]);
  });

  it('marks nothing when lines were only removed', () => {
    expect(changedLines('a\n\n\nb', 'a\nb')).toEqual([]);
  });

  it('marks every new line when the texts are too long to compare', () => {
    const before = Array.from({ length: 2100 }, (_, index) => `old ${index}`).join('\n');
    const after = Array.from({ length: 2100 }, (_, index) => `new ${index}`).join('\n');

    expect(changedLines(before, after)).toHaveLength(2100);
  });
});
