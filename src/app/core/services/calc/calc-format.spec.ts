import { describe, expect, it } from 'vitest';
import { CALC_LOCALES } from '@core/model/calc.model';
import type { CalcDateFormat, CalcSheet, CalcValue } from '@core/model/calc.model';
import en from '@core/services/i18n/translations/en.json';
import fr from '@core/services/i18n/translations/fr.json';
import { CalcFormatter, decimalMean, decimalSum } from './calc-format';

/** The real strings, `{{name}}` replaced: what the words read in each language is the point. */
function translator(file: object) {
  return (key: string, params: Record<string, unknown> = {}): string => {
    const found = key
      .split('.')
      .reduce<unknown>((node, part) => (node as Record<string, unknown>)?.[part], file);
    const text = typeof found === 'string' ? found : key;
    return text.replace(/\{\{(\w+)\}\}/g, (_, name: string) => {
      const value = params[name];
      return typeof value === 'string' || typeof value === 'number' ? String(value) : '';
    });
  };
}

const french = new CalcFormatter('fr', translator(fr));
const english = new CalcFormatter('en', translator(en));
const plain = (text: string): string => text.replace(/[\u00a0\u202f]/g, ' ');

const euros = (value: string): CalcValue => ({ kind: 'number', value, unit: { unit: 'euro', per: null } });

describe('CalcFormatter', () => {
  it('writes a result in the language on screen, its decimal mark and its unit symbols', () => {
    const size: CalcValue = { kind: 'number', value: '1.5', unit: { unit: 'gigabyte', per: null } };

    expect(plain(french.value(size).text)).toBe('1,5 Go');
    expect(plain(english.value(size).text)).toBe('1.5 GB');
  });

  it('shows money with two decimals and says ≈ when the display rounds', () => {
    expect(french.value(euros('26'))).toMatchObject({ approx: false });
    expect(plain(french.value(euros('26')).text)).toBe('26,00 €');
    expect(french.value(euros('100.521'))).toMatchObject({ approx: true });
    expect(plain(french.value(euros('1578.132')).text)).toBe('1 578,13 €');
  });

  /** The digits and the order of the parts: a WebView may write a group with a narrow space or a plain one. */
  it('writes its numbers in the locale chosen, its words still in the language on screen', () => {
    const size: CalcValue = { kind: 'number', value: '1578.13', unit: { unit: 'gigabyte', per: null } };
    const shape = (locale: string, lang = 'fr'): string =>
      plain(
        new CalcFormatter(lang, translator(lang === 'fr' ? fr : en), { locale }).value(size).text,
      ).replace(/[\s'’.,](?=\d{3}\b)/g, '_');

    expect(shape('de-DE')).toBe('1_578,13 Go');
    expect(shape('en-US')).toBe('1_578.13 Go');
    expect(shape('fr-FR', 'en')).toBe('1_578,13 GB');
    expect(shape('ja-JP')).toBe('1_578.13 Go');
    for (const locale of CALC_LOCALES) {
      const written = new CalcFormatter('fr', translator(fr), { locale }).value(size).text;
      expect(written.replace(/\D/g, ''), locale).toBe('157813');
    }
  });

  it('writes a date numeric, short or long, in the locale chosen', () => {
    const date: CalcValue = { kind: 'date', date: '2026-12-01' };
    const written = (format: CalcDateFormat, locale: string | null = null): string =>
      plain(new CalcFormatter('fr', translator(fr), { locale, date: format }).value(date).text);

    expect(written('numeric')).toBe('01/12/2026');
    expect(written('numeric', 'en-US')).toBe('12/01/2026');
    expect(written('numeric', 'de-DE')).toBe('01.12.2026');
    expect(written('short')).toBe('1 déc. 2026');
    expect(written('long')).toBe('mardi 1 décembre 2026');
    expect(written('long', 'en-GB')).toMatch(/^Tuesday,? 1 December 2026$/);
    expect(plain(english.value(date).text)).toBe('Dec 1, 2026');
  });

  it('shows fourteen decimals of the exact value, past what a float holds', () => {
    const fourteen = new CalcFormatter('en', translator(en), { decimals: '14' });
    expect(fourteen.value({ kind: 'number', value: '1234567.12345678901234', unit: null })).toEqual({
      text: '1,234,567.12345678901234',
      approx: false,
    });
    expect(fourteen.value({ kind: 'number', value: '0.333333333333333333', unit: null })).toEqual({
      text: '0.33333333333333',
      approx: true,
    });
  });

  it('keeps the first figures of a small value and forces what « Décimales » asks', () => {
    const rate: CalcValue = { kind: 'number', value: '0.045', unit: { unit: 'euro', per: 'hour' } };
    expect(plain(french.value(rate).text)).toBe('0,045 €/h');

    const forced = new CalcFormatter('fr', translator(fr), { decimals: '0' });
    expect(forced.value(euros('100.521'))).toMatchObject({ approx: true });
    expect(plain(forced.value(euros('100.521')).text)).toBe('101 €');
  });

  it('writes percentages, signed for a change, dates and spans in each language', () => {
    expect(plain(french.value({ kind: 'percent', value: '25', signed: true }).text)).toBe('+25 %');
    expect(english.value({ kind: 'percent', value: '25', signed: false }).text).toBe('25%');
    expect(french.value({ kind: 'date', date: '2026-11-30' }).text).toContain('30 nov. 2026');
    const span: CalcValue = {
      kind: 'span',
      parts: [
        { value: '6', unit: 'week' },
        { value: '3', unit: 'day' },
      ],
    };
    expect(plain(french.value(span).text)).toBe('6 sem 3 j');
    expect(plain(english.value(span).text)).toBe('6 wk 3 d');
  });

  it('gives every decimal for a copy of one result', () => {
    expect(plain(french.exact(euros('100.521')))).toBe('100,521 €');
  });

  it('says why a line has no result, a capital first', () => {
    expect(french.error({ kind: 'currency' })).toBe('Hors ligne : pas de taux de change');
    expect(
      french.error({
        kind: 'mismatch',
        expected: 'length',
        found: 'data',
        unit: { unit: 'gigabyte', per: null },
      }),
    ).toBe('Go n’est pas une longueur');
    expect(english.error({ kind: 'unknownName', name: 'ttc' })).toBe('“ttc” is not defined anywhere above');
    expect(french.error({ kind: 'reserved', word: 'en' })).toBe('« en » est un mot réservé, pas un nom');
    expect(french.error({ kind: 'noPrevious' })).toBe('Aucune ligne au-dessus n’a de résultat');
    expect(english.error({ kind: 'noPrevious' })).toBe('No line above has a result');
    expect(english.error({ kind: 'reserved', word: 'de' })).toBe('“de” is a reserved word, not a name');
    expect(french.error({ kind: 'notConvertible', from: 'mass', to: null })).toBe(
      'Une masse ne se convertit pas en %',
    );
  });

  it('copies a sheet with each result after its line, and a comment already there replaced', () => {
    const sheet: CalcSheet = {
      rows: [
        { line: { kind: 'heading', title: 'Budget' }, spans: [], converting: null },
        {
          line: { kind: 'value', value: euros('100.521'), name: 'ttc', aggregate: false, dimension: 'euro' },
          spans: [],
          converting: null,
        },
        { line: { kind: 'text' }, spans: [], converting: null },
      ],
      variables: [],
    };

    expect(plain(french.copyText('# Budget\nttc = 100,521 € // ancien\nune phrase', sheet))).toBe(
      '# Budget\nttc = 100,521 € // ≈ 100,52 €\nune phrase',
    );
  });
});

describe('the exact arithmetic of the footer', () => {
  it('adds and averages the decimals Rust sends without a float in between', () => {
    expect(decimalSum(['0.1', '0.2'])).toBe('0.3');
    expect(decimalSum(['100.521', '4.99', '26'])).toBe('131.511');
    expect(decimalSum(['-1.5', '0.5'])).toBe('-1');
    expect(decimalMean(['1', '2'])).toBe('1.5');
    expect(decimalMean(['1', '1', '2']).startsWith('1.333333')).toBe(true);
  });
});
