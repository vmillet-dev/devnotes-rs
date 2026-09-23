import { describe, expect, it } from 'vitest';
import { byCodeUnit } from './order.util';

describe('byCodeUnit', () => {
  it('orders exactly as a sort with no comparator does', () => {
    const words = ['étape', 'Zebra', 'apple', 'Étape', 'zebra', 'Apple', '10', '9'];

    expect([...words].sort(byCodeUnit)).toEqual([...words].sort());
  });

  it('answers zero for equal strings', () => {
    expect(byCodeUnit('sql', 'sql')).toBe(0);
  });
});
