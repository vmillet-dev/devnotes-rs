import { TestBed } from '@angular/core/testing';
import { firstValueFrom } from 'rxjs';
import { beforeEach, describe, expect, it } from 'vitest';
import { APP_INFO } from '@core/services/app-info/app-info.service';
import en from './translations/en.json';
import fr from './translations/fr.json';
import { AppTranslocoLoader } from './transloco-loader';

describe('AppTranslocoLoader', () => {
  let loader: AppTranslocoLoader;

  beforeEach(() => {
    TestBed.resetTestingModule();
    TestBed.configureTestingModule({});
    loader = TestBed.inject(AppTranslocoLoader);
  });

  it('adds the application name as a key, which is what `{{app}}` resolves to', async () => {
    const translation = await firstValueFrom(loader.getTranslation('fr'));

    expect(translation['app']).toBe(APP_INFO.name);
  });

  it('answers an empty translation for a language it does not bundle', async () => {
    const translation = await firstValueFrom(loader.getTranslation('de'));

    expect(Object.keys(translation)).toEqual(['app']);
  });
});

/** Asserted on the shipped files: no translated string may spell the app's name out. */
describe('the translation files', () => {
  function strings(node: unknown): string[] {
    if (typeof node === 'string') return [node];
    if (node !== null && typeof node === 'object') {
      return Object.values(node).flatMap(strings);
    }
    return [];
  }

  it.each([
    ['fr', fr],
    ['en', en],
  ])('never spells the application name out (%s)', (_locale, translations) => {
    expect(strings(translations).filter((value) => value.includes(APP_INFO.name))).toEqual([]);
  });

  it.each([
    ['fr', fr],
    ['en', en],
  ])('writes {{app}} instead (%s)', (_locale, translations) => {
    expect(strings(translations).some((value) => value.includes('{{app}}'))).toBe(true);
  });

  /** Both keys, in every locale: the way out must be as visible as the way through. */
  it.each([
    ['fr', fr, ['Suppr', 'Échap']],
    ['en', en, ['Del', 'Esc']],
  ])(
    'names the key that calls an armed deletion off as well as the one that fires it (%s)',
    (_locale, translations, keys) => {
      const banner = (translations as { notes: { deleteArmed: string } }).notes.deleteArmed;

      for (const key of keys as string[]) {
        expect(banner).toContain(key);
      }
    },
  );

  /** The same key on both surfaces: the bar that offers the note back names it too. */
  it.each([
    ['fr', fr, 'Échap'],
    ['en', en, 'Esc'],
  ])('names that same key on the bar that offers the note back (%s)', (_locale, translations, key) => {
    const bar = (translations as { undo: { restoreKey: string } }).undo.restoreKey;

    expect(bar).toBe(key);
  });

  /**
   * "(s)" is not a plural, it is a refusal to choose one — and French does not agree
   * with English about zero, or about where the mark goes on a past participle. Counting
   * is the transpiler's job now.
   */
  it.each([
    ['fr', fr],
    ['en', en],
  ])('counts with a plural rather than an apologetic "(s)" (%s)', (_locale, translations) => {
    expect(strings(translations).filter((value) => value.includes('(s)'))).toEqual([]);
  });

  /**
   * The messageformat transpiler replaces the default one, so `{` is syntax. A literal
   * brace in a translated string would have to be escaped as `'{'`, and the failure mode is
   * silent — the string renders as something else entirely rather than throwing.
   */
  it.each([
    ['fr', fr],
    ['en', en],
  ])('leaves no brace that is neither an interpolation nor a plural (%s)', (_locale, translations) => {
    const suspicious = strings(translations).filter((value) =>
      // `{{name}}` and `{name…}` are both fine; a lone `{` with nothing after it is not.
      /\{(?!\{)\s*(?:\}|$)/.test(value),
    );

    expect(suspicious).toEqual([]);
  });
});
