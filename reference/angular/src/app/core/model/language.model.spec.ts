import { describe, expect, it } from 'vitest';
import {
  FALLBACK_LANGUAGE,
  LANGUAGE_FULL_NAMES,
  LANGUAGE_HUES,
  LANGUAGE_LABELS,
  LANGUAGE_NAMES,
  LanguageTag,
  isLanguageTag,
} from './language.model';

describe('language model', () => {
  it('labels, names and hues every language it offers', () => {
    const tags = Object.keys(LANGUAGE_LABELS) as LanguageTag[];

    expect(tags).toHaveLength(142);
    expect(tags.every((tag) => LANGUAGE_LABELS[tag].length > 0 && LANGUAGE_FULL_NAMES[tag].length > 0)).toBe(
      true,
    );
    expect(Object.keys(LANGUAGE_HUES)).toEqual(tags);
  });

  /** A badge is read at a glance: two languages behind one label would be one badge. */
  it('gives every language a label of its own', () => {
    const labels = Object.values(LANGUAGE_LABELS);

    expect(new Set(labels).size).toBe(labels.length);
  });

  it('hints the full name where the label abbreviates it', () => {
    expect(LANGUAGE_NAMES.tf).toBe('Terraform / HCL');
    expect(LANGUAGE_NAMES.json).toBeUndefined();
  });

  /** The key order is the editor select's, so it is part of what the file decides. */
  it('opens on the data formats and ends on plain text', () => {
    const tags = Object.keys(LANGUAGE_LABELS);

    expect(tags[0]).toBe('json');
    expect(tags.slice(-2)).toEqual(['md', 'txt']);
  });

  it('narrows a free string to a language it knows', () => {
    expect(isLanguageTag('sql')).toBe(true);
    expect(isLanguageTag('rs')).toBe(true);
    expect(isLanguageTag('tf')).toBe(true);
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
