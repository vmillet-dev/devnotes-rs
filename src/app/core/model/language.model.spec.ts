import { describe, expect, it } from 'vitest';
import { FALLBACK_LANGUAGE, LANGUAGE_LABELS, LanguageTag, isLanguageTag } from './language.model';

describe('language model', () => {
  it('labels every language it offers, and offers the eighteen it highlights', () => {
    const tags = Object.keys(LANGUAGE_LABELS) as LanguageTag[];

    expect(tags.length).toBe(19);
    expect(tags.every((tag) => LANGUAGE_LABELS[tag].length > 0)).toBe(true);
  });

  /** The key order is the editor select's, so it is part of what the file decides. */
  it('opens on the data formats and ends on plain text', () => {
    const tags = Object.keys(LANGUAGE_LABELS);

    expect(tags[0]).toBe('json');
    expect(tags.at(-1)).toBe('txt');
  });

  it('narrows a free string to a language it knows', () => {
    expect(isLanguageTag('sql')).toBe(true);
    expect(isLanguageTag('rs')).toBe(true);
  });

  /**
   * It narrows a `<select>`'s value, never data from the bridge: an unknown language
   * coming from Rust fails deserialisation before it gets here.
   */
  it('refuses anything it does not label', () => {
    expect(isLanguageTag('rust')).toBe(false);
    expect(isLanguageTag('')).toBe(false);
    expect(isLanguageTag(null)).toBe(false);
    expect(isLanguageTag(undefined)).toBe(false);
    expect(isLanguageTag(42)).toBe(false);
  });

  /** `Object.hasOwn`, so a name off the prototype is not a language. */
  it('refuses the names every object carries', () => {
    expect(isLanguageTag('toString')).toBe(false);
    expect(isLanguageTag('constructor')).toBe(false);
    expect(isLanguageTag('__proto__')).toBe(false);
  });

  it('falls back to something it can label', () => {
    expect(isLanguageTag(FALLBACK_LANGUAGE)).toBe(true);
    expect(FALLBACK_LANGUAGE).toBe('txt');
  });
});
