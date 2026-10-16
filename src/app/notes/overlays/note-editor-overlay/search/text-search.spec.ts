import { describe, expect, it } from 'vitest';
import { SearchSpec } from './search-target';
import { findMatches, replacementOf, searchPattern } from './text-search';

const spec = (search: string, options: Partial<SearchSpec> = {}): SearchSpec => ({
  search,
  replace: '',
  caseSensitive: false,
  wholeWord: false,
  regexp: false,
  ...options,
});

const found = (text: string, query: SearchSpec): string[] =>
  findMatches(text, searchPattern(query)!).map((match) => `${match.from}-${match.to}`);

describe('text search', () => {
  it('finds a text as typed, its special characters taken literally, in any case', () => {
    expect(found('Price (EUR) and price (eur)', spec('price (eur)'))).toEqual(['0-11', '16-27']);
  });

  it('respects the case when asked', () => {
    expect(found('Deploy deploy', spec('deploy', { caseSensitive: true }))).toEqual(['7-13']);
  });

  it('keeps to whole words, accented letters counted as letters', () => {
    expect(found('été étésien été_2 été', spec('été', { wholeWord: true }))).toEqual(['0-3', '18-21']);
  });

  it('reads a regular expression, and skips what matches nothing', () => {
    expect(found('v1.2 v10', spec(String.raw`v\d+`, { regexp: true }))).toEqual(['0-2', '5-8']);
    expect(found('abc', spec('x*', { regexp: true }))).toEqual([]);
  });

  it('answers no pattern for an empty query or one that does not compile', () => {
    expect(searchPattern(spec(''))).toBeNull();
    expect(searchPattern(spec('(', { regexp: true }))).toBeNull();
  });

  it('replaces with the text as typed, and with a regular expression reads its groups', () => {
    const literal = spec('a.b', { replace: '$1' });
    const regexp = spec(String.raw`(\w+)@(\w+)`, { regexp: true, replace: '$2 at $1' });

    expect(replacementOf({ from: 0, to: 3, text: 'a.b' }, literal, searchPattern(literal)!)).toBe('$1');
    expect(replacementOf({ from: 0, to: 9, text: 'ops@corp' }, regexp, searchPattern(regexp)!)).toBe(
      'corp at ops',
    );
  });
});
