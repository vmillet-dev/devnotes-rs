import { describe, it } from 'vitest';
import { CALC_DATE_FORMATS, CALC_DECIMALS, CALC_LOCALES } from '@core/model/calc.model';
import type { CalcValue } from '@core/model/calc.model';
import { CalcFormatter, decimalMean, decimalSum } from '@core/services/calc/calc-format';
import { expiryRef, relativeTimeRef, spanRef } from '@core/utils/relative-time.util';
import { formatDecimal, formatScientific } from '@core/utils/decimal-format.util';
import { GoldenCase, writeGolden, writingGolden } from '@testing/golden';
import { expandPlurals } from './plural-transpiler';
import en from './translations/en.json';
import fr from './translations/fr.json';

const LANGUAGES = { fr, en } as const;
type Lang = keyof typeof LANGUAGES;

/** A translation as the screen shows it: plurals expanded, then `{{name}}` replaced. */
function translate(lang: Lang, key: string, params: Record<string, unknown> = {}): string {
  const found = key.split('.').reduce<unknown>((node, part) => (node as Record<string, unknown>)?.[part], LANGUAGES[lang]);
  const source = typeof found === 'string' ? found : key;
  return expandPlurals(source, params as never, lang).replace(/\{\{\s*(\w+)\s*\}\}/g, (_, name: string) => {
    const value = params[name];
    return typeof value === 'string' || typeof value === 'number' ? String(value) : '';
  });
}

/** Every key of a translation file whose text holds a plural block, with the name it counts. */
function pluralKeys(node: unknown, prefix = ''): [key: string, counted: string][] {
  if (typeof node === 'string') {
    return [...node.matchAll(/\{\s*([A-Za-z0-9_]+)\s*,\s*plural\s*,/g)].map((match) => [prefix, match[1] ?? '']);
  }
  return Object.entries(node as Record<string, unknown>).flatMap(([key, child]) =>
    pluralKeys(child, prefix ? `${prefix}.${key}` : key),
  );
}

const COUNTS = [0, 1, 2, 5, 21, 1_000, 1_000_000, 1.5];
const NOW = new Date('2026-07-25T09:00:00.000Z');

/**
 * Each `new Intl.*` of the front, with the options it is built with (the call site in the
 * comment), and values chosen at the edges: negatives, large numbers, midnight, a year's turn.
 */
const DATES = [
  '2026-07-25T09:05:07.123Z',
  '2026-12-31T23:59:59.999Z',
  '2027-01-01T00:00:00.000Z',
  '1971-11-08T04:25:55.000Z',
].map((iso) => new Date(iso));
const NUMBERS = [0, -0.5, 1.25, 12.345, 999.95, 1_234_567.891, -1_234.5];
const INTL: readonly { readonly site: string; readonly kind: 'date' | 'number'; readonly options: object }[] = [
  { site: 'calc-format DATE_STYLES.numeric', kind: 'date', options: { day: '2-digit', month: '2-digit', year: 'numeric' } },
  { site: 'calc-format DATE_STYLES.short', kind: 'date', options: { day: 'numeric', month: 'short', year: 'numeric' } },
  { site: 'calc-format DATE_STYLES.long', kind: 'date', options: { weekday: 'long', day: 'numeric', month: 'long', year: 'numeric' } },
  { site: 'http-history.store draft name', kind: 'date', options: { hour: '2-digit', minute: '2-digit' } },
  { site: 'history-page day', kind: 'date', options: { weekday: 'long', day: 'numeric', month: 'long' } },
  { site: 'history-page time', kind: 'date', options: { hour: '2-digit', minute: '2-digit', second: '2-digit' } },
  { site: 'cookie-jar-dialog expiry', kind: 'date', options: { dateStyle: 'medium', timeStyle: 'short' } },
  { site: 'socket-log time', kind: 'date', options: { hour: '2-digit', minute: '2-digit', second: '2-digit', fractionalSecondDigits: 3 } },
  { site: 'dates-tool day', kind: 'date', options: { weekday: 'long', day: 'numeric', month: 'long', year: 'numeric' } },
  { site: 'dates-tool time', kind: 'date', options: { hour: '2-digit', minute: '2-digit', second: '2-digit', hourCycle: 'h23' } },
  { site: 'durations day', kind: 'date', options: { day: 'numeric', month: 'long', timeZone: 'UTC' } },
  { site: 'identity-tool birth', kind: 'date', options: { dateStyle: 'long', timeZone: 'UTC' } },
  { site: 'response-pane tenths', kind: 'number', options: { minimumFractionDigits: 1, maximumFractionDigits: 1 } },
  { site: 'runner-results seconds', kind: 'number', options: { maximumFractionDigits: 1, minimumFractionDigits: 1 } },
  { site: 'response-view size', kind: 'number', options: { maximumFractionDigits: 2 } },
  { site: 'search-bar count', kind: 'number', options: {} },
  { site: 'json-explorer size', kind: 'number', options: { maximumFractionDigits: 1 } },
  { site: 'settings-page hint', kind: 'number', options: { minimumFractionDigits: 2 } },
  { site: 'colour-tool', kind: 'number', options: { minimumFractionDigits: 2, maximumFractionDigits: 2 } },
  { site: 'durations number', kind: 'number', options: { maximumFractionDigits: 6 } },
  { site: 'transfer number', kind: 'number', options: {} },
  { site: 'calc-format signed', kind: 'number', options: { minimumFractionDigits: 0, maximumFractionDigits: 2, signDisplay: 'exceptZero' } },
  { site: 'calc-format ungrouped', kind: 'number', options: { minimumFractionDigits: 0, maximumFractionDigits: 2, useGrouping: false } },
];

const CALC_VALUES: readonly CalcValue[] = [
  { kind: 'number', value: '1234567.891', unit: null },
  { kind: 'number', value: '-0.5', unit: { unit: 'kilometre', per: null } },
  { kind: 'number', value: '0.000012', unit: null },
  { kind: 'percent', value: '25', signed: true },
  { kind: 'percent', value: '-12.5', signed: false },
  { kind: 'date', date: '2026-12-01' },
  { kind: 'base', digits: 'FF', radix: 16, negative: false },
  { kind: 'scientific', mantissa: '5.3', exponent: 3 },
  { kind: 'fraction', numerator: '1', denominator: '5', negative: true },
  { kind: 'multiplier', value: '4' },
];

describe.runIf(writingGolden)('golden: i18n', () => {
  it('captures every plural key at the counts that tell the rules apart', () => {
    const cases: GoldenCase[] = (Object.keys(LANGUAGES) as Lang[]).flatMap((lang) =>
      pluralKeys(LANGUAGES[lang]).flatMap(([key, counted]) =>
        COUNTS.map((count) => ({ input: key, options: { lang, [counted]: count }, output: translate(lang, key, { [counted]: count }) })),
      ),
    );
    writeGolden('plurals', cases);
  });

  it('captures each Intl formatter of the front', () => {
    const cases: GoldenCase[] = INTL.flatMap(({ site, kind, options }) =>
      (['fr', 'en'] as const).flatMap((lang) =>
        (kind === 'date' ? DATES : NUMBERS).map((value) => ({
          input: value instanceof Date ? value.toISOString() : value,
          options: { site, lang, ...options },
          output:
            value instanceof Date
              ? new Intl.DateTimeFormat(lang, options).format(value)
              : new Intl.NumberFormat(lang, options).format(value),
        })),
      ),
    );
    for (const lang of ['fr', 'en'] as const) {
      cases.push(
        { input: ['un', 'deux', 'trois'], options: { site: 'cron-tool', lang, type: 'conjunction' }, output: new Intl.ListFormat(lang, { type: 'conjunction' }).format(['un', 'deux', 'trois']) },
        { input: ['un', 'deux'], options: { site: 'note-picker', lang, type: 'disjunction' }, output: new Intl.ListFormat(lang, { type: 'disjunction' }).format(['un', 'deux']) },
        ...['FR', 'DE', 'GB', 'CH', 'BE'].map((region) => ({ input: region, options: { site: 'checks-tool', lang, type: 'region' }, output: new Intl.DisplayNames([lang], { type: 'region' }).of(region) })),
        ...COUNTS.map((count) => ({ input: count, options: { site: 'plural-transpiler', lang }, output: new Intl.PluralRules(lang).select(count) })),
      );
    }
    writeGolden('intl', cases);
  });

  it('captures the relative labels from a second to a year', () => {
    const gaps = [0, 30_000, 60_000, 59 * 60_000, 3_600_000, 23 * 3_600_000, 86_400_000, 6 * 86_400_000, 40 * 86_400_000, 400 * 86_400_000];
    const cases: GoldenCase[] = (['fr', 'en'] as const).flatMap((lang) =>
      gaps.flatMap((gap) => {
        const past = new Date(NOW.getTime() - gap);
        const future = new Date(NOW.getTime() + gap);
        const label = (ref: { key: string; params?: Record<string, unknown> }) => ({ ...ref, text: translate(lang, ref.key, ref.params) });
        return [
          { input: { now: NOW.toISOString(), at: past.toISOString() }, options: { lang, kind: 'relative' }, output: label(relativeTimeRef(past, NOW)) },
          { input: { now: NOW.toISOString(), at: future.toISOString() }, options: { lang, kind: 'expiry' }, output: label(expiryRef(future, NOW)) },
          { input: { now: NOW.toISOString(), at: past.toISOString() }, options: { lang, kind: 'span' }, output: label(spanRef(past.getTime(), NOW)) },
          { input: { now: NOW.toISOString(), at: future.toISOString() }, options: { lang, kind: 'span' }, output: label(spanRef(future.getTime(), NOW)) },
        ];
      }),
    );
    writeGolden('relative-time', cases);
  });

  it('captures how a Calcul sheet writes its values, its sum and its mean', () => {
    const cases: GoldenCase[] = [];
    for (const lang of ['fr', 'en'] as const) {
      for (const locale of [null, ...CALC_LOCALES]) {
        for (const decimals of CALC_DECIMALS) {
          for (const date of CALC_DATE_FORMATS) {
            const formatter = new CalcFormatter(lang, (key, params) => translate(lang, key, params), { locale, decimals, date });
            for (const value of CALC_VALUES) {
              cases.push({ input: value, options: { lang, locale, decimals, date }, output: formatter.value(value) });
            }
          }
        }
      }
    }
    const columns = [['1.5', '2.25', '-0.75'], ['0.1', '0.2'], ['1000000', '0.000001']];
    for (const values of columns) {
      cases.push({ input: values, options: { footer: 'sum' }, output: decimalSum(values) });
      cases.push({ input: values, options: { footer: 'mean' }, output: decimalMean(values) });
    }
    for (const lang of ['fr', 'en', 'de']) {
      for (const value of ['1234567.891', '-1234', '0.5']) {
        cases.push({ input: value, options: { lang, helper: 'formatDecimal' }, output: formatDecimal(value, lang) });
      }
      cases.push({ input: '0.000012', options: { lang, helper: 'formatScientific', decimals: 2 }, output: formatScientific('0.000012', lang, 2) });
    }
    writeGolden('calc-format', cases);
  });
});
