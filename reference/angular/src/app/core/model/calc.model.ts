import type {
  CalcCardLine,
  CalcError,
  CalcLine,
  CalcRow,
  CalcSheet,
  CalcSpan,
  CalcUnit,
  CalcValue,
  CalcVariable,
  Dimension,
  SpanPart,
  TokenKind,
  UnitId,
} from '@core/ipc/bindings';
import { CALC_UNITS, CALC_WORDS } from '@core/ipc/bindings';

export type {
  CalcCardLine,
  CalcError,
  CalcLine,
  CalcRow,
  CalcSheet,
  CalcSpan,
  CalcUnit,
  CalcValue,
  CalcVariable,
  Dimension as CalcDimension,
  SpanPart,
  TokenKind as CalcTokenKind,
  UnitId as CalcUnitId,
};

/** « Décimales »: `auto` reads the value, a figure forces it. */
export const CALC_DECIMALS = [
  'auto',
  '0',
  '1',
  '2',
  '3',
  '4',
  '5',
  '6',
  '7',
  '8',
  '9',
  '10',
  '11',
  '12',
  '13',
  '14',
] as const;
export type CalcDecimals = (typeof CALC_DECIMALS)[number];

/**
 * « Format des nombres et des dates »: the language on screen, or one of these, written by
 * `Intl` — a locale carries no data of ours, so adding one is a label and a test.
 */
export const CALC_LOCALES = [
  'fr-FR',
  'fr-CH',
  'en-US',
  'en-GB',
  'de-DE',
  'es-ES',
  'it-IT',
  'pt-BR',
  'nl-NL',
  'pl-PL',
  'sv-SE',
  'ru-RU',
  'ja-JP',
  'zh-CN',
] as const;
export const CALC_NUMBER_FORMATS = ['language', ...CALC_LOCALES] as const;
export type CalcNumberFormat = (typeof CALC_NUMBER_FORMATS)[number];

/** « Format de date »: `01/12/2026`, `1 déc. 2026` or `mardi 1 décembre 2026`. */
export const CALC_DATE_FORMATS = ['numeric', 'short', 'long'] as const;
export type CalcDateFormat = (typeof CALC_DATE_FORMATS)[number];

/** A concept of the grammar and its spellings in each language, as Rust's table gives them. */
export interface CalcWordEntry {
  readonly word: (typeof CALC_WORDS)[number]['word'];
  readonly class: 'syntax' | 'value' | 'setting';
  readonly fr: readonly string[];
  readonly en: readonly string[];
}

export type CalcWord = CalcWordEntry['word'];

/** The units of one kind, as Rust lists them: what the editor proposes after `en` or `à`. */
export function unitsOf(dimension: Dimension): readonly UnitId[] {
  return CALC_UNITS.filter((unit) => unit.dimension === dimension).map((unit) => unit.id);
}
