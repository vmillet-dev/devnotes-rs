import { describe, expect, it } from 'vitest';
import { language } from '@codemirror/language';
import { EditorState } from '@codemirror/state';
import { LANGUAGE_LABELS, LanguageTag } from '@core/model/language.model';
import { LANGUAGES, languageOf } from './languages';

describe('the grammars', () => {
  const tags = Object.keys(LANGUAGE_LABELS) as LanguageTag[];

  it('has an entry for every language a snippet can be in', () => {
    expect(Object.keys(LANGUAGES).sort()).toEqual([...tags].sort());
  });

  it.each(tags.filter((tag) => tag !== 'txt'))('loads a grammar for %s', async (tag) => {
    const state = EditorState.create({ extensions: await languageOf(tag) });

    expect(state.facet(language)).not.toBeNull();
  });

  it('gives plain text none', async () => {
    const state = EditorState.create({ extensions: await languageOf('txt') });

    expect(state.facet(language)).toBeNull();
  });
});
