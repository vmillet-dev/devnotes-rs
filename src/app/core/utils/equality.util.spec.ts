import { describe, expect, it } from 'vitest';
import { sameArray, sameBy } from './equality.util';

describe('sameArray', () => {
  it('compares element by element rather than by identity', () => {
    expect(sameArray(['a', 'b'], ['a', 'b'])).toBe(true);
    expect(sameArray(['a', 'b'], ['b', 'a'])).toBe(false);
    expect(sameArray(['a'], ['a', 'b'])).toBe(false);
  });

  it('takes the comparator the elements need', () => {
    const byText = (x: { text: string }, y: { text: string }): boolean => x.text === y.text;

    expect(sameArray([{ text: 'a' }], [{ text: 'a' }])).toBe(false);
    expect(sameArray([{ text: 'a' }], [{ text: 'a' }], byText)).toBe(true);
  });
});

describe('sameBy', () => {
  interface Params {
    readonly day: string;
    readonly tags: readonly string[];
  }

  const same = sameBy<Params>({ day: Object.is, tags: sameArray });

  it('is equal when every field is, each by its own comparator', () => {
    expect(same({ day: 'd', tags: ['a'] }, { day: 'd', tags: ['a'] })).toBe(true);
  });

  it('is unequal as soon as one field differs', () => {
    expect(same({ day: 'd', tags: ['a'] }, { day: 'e', tags: ['a'] })).toBe(false);
    expect(same({ day: 'd', tags: ['a'] }, { day: 'd', tags: ['b'] })).toBe(false);
  });
});
