import { describe, expect, it } from 'vitest';
import { SearchQuery } from '@codemirror/search';
import { EditorState } from '@codemirror/state';
import { PreferencesService } from '@core/services/preferences/preferences.service';
import { countMatches, firstMatchFrom, readSearchOptions, writeSearchOptions } from './search-query';

const state = (doc: string, anchor = 0, head = anchor) =>
  EditorState.create({ doc, selection: { anchor, head } });

/** The two calls the options use, over a map. */
function preferences(stored: Record<string, string> = {}): PreferencesService {
  return {
    read: (key: string) => stored[key] ?? null,
    write: (key: string, value: string) => {
      stored[key] = value;
    },
  } as unknown as PreferencesService;
}

describe('countMatches', () => {
  it('counts every match, and says which one the selection is on', () => {
    expect(countMatches(new SearchQuery({ search: 'ab' }), state('ab ab ab', 3, 5))).toEqual({
      total: 3,
      current: 2,
      capped: false,
    });
  });

  it('stops at the cap', () => {
    expect(countMatches(new SearchQuery({ search: 'a' }), state('aaaaa'), 3)).toEqual({
      total: 3,
      current: null,
      capped: true,
    });
  });

  it('counts nothing for an expression that does not compile', () => {
    expect(countMatches(new SearchQuery({ search: '(', regexp: true }), state('((('))).toEqual({
      total: 0,
      current: null,
      capped: false,
    });
  });
});

describe('firstMatchFrom', () => {
  const query = new SearchQuery({ search: 'ab' });

  it('takes the first match from the origin on', () => {
    expect(firstMatchFrom(query, state('ab ab'), 1)).toMatchObject({ from: 3, to: 5 });
  });

  it('wraps round to the first match of the text', () => {
    expect(firstMatchFrom(query, state('ab cd'), 2)).toMatchObject({ from: 0, to: 2 });
  });

  it('finds nothing where nothing matches, nor for an invalid query', () => {
    expect(firstMatchFrom(query, state('cd'), 0)).toBeNull();
    expect(firstMatchFrom(new SearchQuery({ search: '' }), state('cd'), 0)).toBeNull();
  });
});

describe('the search options', () => {
  it('are off until written, and read back as written', () => {
    const stored = preferences();
    expect(readSearchOptions(stored)).toEqual({ caseSensitive: false, wholeWord: false, regexp: false });

    writeSearchOptions(stored, { caseSensitive: true, wholeWord: false, regexp: true });

    expect(readSearchOptions(stored)).toEqual({ caseSensitive: true, wholeWord: false, regexp: true });
  });

  it('take anything but a stored true as off', () => {
    const odd = preferences({ 'devnotes.editorSearchOptions': '{"caseSensitive":"yes","regexp":true}' });
    expect(readSearchOptions(odd)).toEqual({ caseSensitive: false, wholeWord: false, regexp: true });

    expect(readSearchOptions(preferences({ 'devnotes.editorSearchOptions': 'not json' }))).toEqual({
      caseSensitive: false,
      wholeWord: false,
      regexp: false,
    });
    expect(readSearchOptions(preferences({ 'devnotes.editorSearchOptions': 'null' })).regexp).toBe(false);
  });
});
