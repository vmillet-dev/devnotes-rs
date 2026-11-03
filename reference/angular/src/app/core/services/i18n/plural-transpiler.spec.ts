import { describe, expect, it } from 'vitest';
import { expandPlurals } from './plural-transpiler';

/**
 * Exhaustive on purpose. This is a parser written here rather than taken from a library,
 * because the library compiles with `new Function` and the CSP forbids it — so the grammar
 * it accepts is the grammar nobody else is checking.
 */
describe('expandPlurals', () => {
  const fr = (source: string, params: Record<string, unknown> = {}) => expandPlurals(source, params, 'fr');
  const en = (source: string, params: Record<string, unknown> = {}) => expandPlurals(source, params, 'en');

  it('picks the singular and the plural from the count', () => {
    const source = '{count, plural, one {# note} other {# notes}}';

    expect(fr(source, { count: 1 })).toBe('1 note');
    expect(fr(source, { count: 4 })).toBe('4 notes');
  });

  /** The whole reason `=0` is written out: French calls zero `one`. */
  it('lets an exact match win over the category', () => {
    const source = '{count, plural, =0 {aucune note} one {# note} other {# notes}}';

    expect(fr(source, { count: 0 })).toBe('aucune note');
    expect(new Intl.PluralRules('fr').select(0)).toBe('one');
  });

  it('follows the locale rather than a rule of its own', () => {
    const source = '{count, plural, one {# note} other {# notes}}';

    // French keeps the singular at one *and* at zero; English does not.
    expect(fr(source, { count: 0 })).toBe('0 note');
    expect(en(source, { count: 0 })).toBe('0 notes');
  });

  it('expands several independent counts in one sentence', () => {
    const source =
      '{notes, plural, one {# note} other {# notes}} depuis {{path}}, ' +
      '{skipped, plural, one {# ignorée} other {# ignorées}}.';

    expect(fr(source, { notes: 1, skipped: 3 })).toBe('1 note depuis {{path}}, 3 ignorées.');
  });

  /** `{{app}}` and the like must pass through untouched: they are the other transpiler's. */
  it('leaves a double-brace interpolation alone', () => {
    expect(fr('Bienvenue dans {{app}}')).toBe('Bienvenue dans {{app}}');
    expect(fr('{{notes}} et {{path}}')).toBe('{{notes}} et {{path}}');
  });

  it('leaves a hash outside a block alone, which is what a tag is written with', () => {
    expect(fr('Renommer #{{tag}} sur {count, plural, one {# note} other {# notes}}', { count: 2 })).toBe(
      'Renommer #{{tag}} sur 2 notes',
    );
  });

  it('falls back to other when the category has no branch', () => {
    expect(fr('{count, plural, other {# choses}}', { count: 1 })).toBe('1 choses');
  });

  it('renders nothing rather than "undefined" when the parameter is missing', () => {
    expect(fr('{count, plural, one {# note} other {# notes}}')).toBe(' notes');
  });

  /** A half-written block is left as written: visibly wrong beats silently truncated. */
  it('leaves an unbalanced block exactly as it found it', () => {
    const broken = 'avant {count, plural, one {# note} other {# notes}';

    expect(fr(broken, { count: 2 })).toBe(broken);
  });

  it('keeps the text around a block, on both sides', () => {
    expect(fr('a {count, plural, other {#}} b', { count: 7 })).toBe('a 7 b');
  });

  it('is unchanged by a string with no block at all', () => {
    expect(fr('Rien à compter ici.')).toBe('Rien à compter ici.');
  });
});
