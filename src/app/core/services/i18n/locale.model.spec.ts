import { afterEach, describe, expect, it } from 'vitest';
import {
  APP_LOCALES,
  DEFAULT_LOCALE,
  SYSTEM_FALLBACK_LOCALE,
  isAppLocale,
  resolveSystemLocale,
} from './locale.model';

function stubLanguages(...tags: string[]): void {
  Object.defineProperty(navigator, 'languages', { value: tags, configurable: true });
  Object.defineProperty(navigator, 'language', { value: tags[0] ?? '', configurable: true });
}

describe('locale model', () => {
  afterEach(() => {
    Reflect.deleteProperty(navigator, 'languages');
    Reflect.deleteProperty(navigator, 'language');
  });

  describe('the two the application speaks', () => {
    it('answers for a locale it ships, and only for those', () => {
      expect(APP_LOCALES).toEqual(['fr', 'en']);
      expect(isAppLocale('fr')).toBe(true);
      expect(isAppLocale('en')).toBe(true);
      expect(isAppLocale('de')).toBe(false);
    });

    it('treats an absent choice as no locale rather than as a default', () => {
      expect(isAppLocale(null)).toBe(false);
      expect(isAppLocale('')).toBe(false);
    });

    /** The file that answers when a key is missing from the other one. */
    it('falls back to French for a missing key', () => {
      expect(DEFAULT_LOCALE).toBe('fr');
    });
  });

  describe('reading the machine', () => {
    it('takes the first language the machine names that the application speaks', () => {
      stubLanguages('en-GB', 'fr-FR');

      expect(resolveSystemLocale()).toBe('en');
    });

    it('skips past languages it does not ship', () => {
      stubLanguages('de-DE', 'es-ES', 'fr-CA');

      expect(resolveSystemLocale()).toBe('fr');
    });

    it('reads the region off the tag', () => {
      stubLanguages('FR-fr');

      expect(resolveSystemLocale()).toBe('fr');
    });

    /**
     * English, not the default: `DEFAULT_LOCALE` answers a missing translation key,
     * this answers a machine speaking neither — and the wider audience is the right guess
     * there, where the fallback file is the right answer to the other question.
     */
    it('falls back to English for a machine that speaks neither', () => {
      stubLanguages('de-DE', 'ja-JP');

      expect(resolveSystemLocale()).toBe(SYSTEM_FALLBACK_LOCALE);
      expect(SYSTEM_FALLBACK_LOCALE).toBe('en');
      expect(SYSTEM_FALLBACK_LOCALE).not.toBe(DEFAULT_LOCALE);
    });

    it('reads `language` when the list is empty', () => {
      Object.defineProperty(navigator, 'languages', { value: [], configurable: true });
      Object.defineProperty(navigator, 'language', { value: 'fr-BE', configurable: true });

      expect(resolveSystemLocale()).toBe('fr');
    });
  });
});
