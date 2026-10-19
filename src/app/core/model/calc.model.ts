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

/** « Décimales » in the editor's footer: `auto` reads the value, a figure forces it. */
export const CALC_DECIMALS = ['auto', '0', '1', '2', '3', '4'] as const;
export type CalcDecimals = (typeof CALC_DECIMALS)[number];

/** A concept of the grammar and its spellings in each language, as Rust's table gives them. */
export interface CalcWordEntry {
  readonly word: (typeof CALC_WORDS)[number]['word'];
  readonly class: 'syntax' | 'value';
  readonly fr: readonly string[];
  readonly en: readonly string[];
}

export type CalcWord = CalcWordEntry['word'];

/** The units of one kind, as Rust lists them: what the editor proposes after `en` or `à`. */
export function unitsOf(dimension: Dimension): readonly UnitId[] {
  return CALC_UNITS.filter((unit) => unit.dimension === dimension).map((unit) => unit.id);
}
