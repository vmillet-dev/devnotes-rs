import { describe, expect, it } from 'vitest';
import { language } from '@codemirror/language';
import { EditorState } from '@codemirror/state';
import { LANGUAGE_LABELS, LanguageTag } from '@core/model/language.model';
import { GRAMMARS, grammarOf, loadGrammar } from './grammars';

describe('the grammars', () => {
  const tags = Object.keys(LANGUAGE_LABELS) as LanguageTag[];

  it('has an entry for every language a snippet can be in', () => {
    expect(Object.keys(GRAMMARS).sort()).toEqual([...tags].sort());
  });

  it.each(tags.filter((tag) => tag !== 'txt'))('loads a grammar for %s', async (tag) => {
    const state = EditorState.create({ extensions: (await loadGrammar(tag)) ?? [] });

    expect(state.facet(language)).not.toBeNull();
  });

  it('gives plain text none', async () => {
    expect(await loadGrammar('txt')).toBeNull();
    expect(grammarOf('txt')).toBeNull();
  });

  it('loads a grammar once, whoever asks', async () => {
    expect(loadGrammar('lua')).toBe(loadGrammar('lua'));
    expect(await loadGrammar('lua')).toBe(await loadGrammar('lua'));
  });

  it('answers nothing until the grammar has arrived, then the grammar', async () => {
    expect(grammarOf('cobol')).toBeNull();

    const support = await loadGrammar('cobol');
    await Promise.resolve();

    expect(grammarOf('cobol')).toBe(support?.language);
  });
});
