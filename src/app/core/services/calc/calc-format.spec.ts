import { describe, expect, it } from 'vitest';
import type { CalcSheet, CalcValue } from '@core/model/calc.model';
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

  it('keeps the first figures of a small value and forces what « Décimales » asks', () => {
    const rate: CalcValue = { kind: 'number', value: '0.045', unit: { unit: 'euro', per: 'hour' } };
    expect(plain(french.value(rate).text)).toBe('0,045 €/h');

    const forced = new CalcFormatter('fr', translator(fr), '0');
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
