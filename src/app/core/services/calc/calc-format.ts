import type {
  CalcDecimals,
  CalcError,
  CalcSheet,
  CalcUnit,
  CalcUnitId,
  CalcValue,
} from '@core/model/calc.model';

/** What a result reads on screen, and whether the display rounded it: `≈ 100,52 €`. */
export interface CalcText {
  readonly text: string;
  readonly approx: boolean;
}

type Translate = (key: string, params?: Record<string, unknown>) => string;

const NBSP = ' ';

/** `-12.5` as its sign, its whole part and its decimals: an exact value never goes through a float. */
function parts(value: string): { negative: boolean; whole: string; fraction: string } {
  const negative = value.startsWith('-');
  const [whole = '0', fraction = ''] = value.replace(/^-/, '').split('.');
  return { negative, whole, fraction: fraction.replace(/0+$/, '') };
}

/** How many decimals a value is shown with, and at least how many. */
function digitsFor(value: string, decimals: CalcDecimals, money: boolean): { min: number; max: number } {
  if (decimals !== 'auto') {
    const forced = Number(decimals);
    return { min: forced, max: forced };
  }
  if (money) return { min: 2, max: 2 };
  const { whole, fraction } = parts(value);
  // A small value keeps its first figures: 0,045 €/h is not 0,05.
  if (/^0*$/.test(whole) && fraction !== '') {
    const first = fraction.search(/[1-9]/);
    return { min: 0, max: Math.min(first + 3, 8) };
  }
  return { min: 0, max: 2 };
}

/** Exact decimal arithmetic for the footer's sum and mean, on the strings Rust sends. */
function scaled(value: string, scale: number): bigint {
  const { negative, whole, fraction } = parts(value);
  const digits = BigInt((whole || '0') + fraction.padEnd(scale, '0'));
  return negative ? -digits : digits;
}

function unscaled(value: bigint, scale: number): string {
  const negative = value < 0n;
  const digits = (negative ? -value : value).toString().padStart(scale + 1, '0');
  const whole = digits.slice(0, digits.length - scale);
  const fraction = scale > 0 ? digits.slice(digits.length - scale).replace(/0+$/, '') : '';
  return `${negative ? '-' : ''}${whole}${fraction ? `.${fraction}` : ''}`;
}

export function decimalSum(values: readonly string[]): string {
  const scale = Math.max(0, ...values.map((value) => parts(value).fraction.length));
  return unscaled(
    values.reduce((sum, value) => sum + scaled(value, scale), 0n),
    scale,
  );
}

/** Twelve decimals past the operands': past what any display shows. */
export function decimalMean(values: readonly string[]): string {
  const scale = Math.max(0, ...values.map((value) => parts(value).fraction.length)) + 12;
  const sum = values.reduce((total, value) => total + scaled(value, scale), 0n);
  return unscaled(sum / BigInt(values.length), scale);
}

/**
 * Writes what Rust computed in the language on screen: its decimal mark, its unit symbols
 * (`Go` or `GB`), its dates and its messages. Rust sends exact values and codes, never words.
 */
export class CalcFormatter {
  constructor(
    private readonly lang: string,
    private readonly translate: Translate,
    private readonly decimals: CalcDecimals = 'auto',
  ) {}

  value(value: CalcValue): CalcText {
    switch (value.kind) {
      case 'number': {
        const money = value.unit !== null && value.unit.per === null && isMoney(value.unit.unit);
        const shown = this.number(value.value, digitsFor(value.value, this.decimals, money));
        const text = value.unit === null ? shown.text : `${shown.text}${NBSP}${this.unit(value.unit)}`;
        return { text, approx: shown.approx };
      }
      case 'percent': {
        const shown = this.number(value.value, digitsFor(value.value, this.decimals, false), value.signed);
        return { text: this.translate('calc.percent', { value: shown.text }), approx: shown.approx };
      }
      case 'date':
        return { text: this.date(value.date), approx: false };
      case 'span':
        return {
          text: value.parts
            .map(
              (part) => `${this.number(part.value, { min: 0, max: 0 }).text}${NBSP}${this.symbol(part.unit)}`,
            )
            .join(' '),
          approx: false,
        };
    }
  }

  /** Every decimal kept, as the copy of one result and the variable's tooltip give it. */
  exact(value: CalcValue): string {
    if (value.kind !== 'number' && value.kind !== 'percent') return this.value(value).text;
    const length = parts(value.value).fraction.length;
    const shown = this.number(
      value.value,
      { min: 0, max: Math.min(length, 20) },
      value.kind === 'percent' && value.signed,
    );
    if (value.kind === 'percent') return this.translate('calc.percent', { value: shown.text });
    return value.unit === null ? shown.text : `${shown.text}${NBSP}${this.unit(value.unit)}`;
  }

  unit(unit: CalcUnit): string {
    return unit.per === null ? this.symbol(unit.unit) : `${this.symbol(unit.unit)}/${this.symbol(unit.per)}`;
  }

  symbol(unit: CalcUnitId): string {
    return this.translate(`calc.units.${unit}`);
  }

  /** Said on its line in place of a result, a capital first. */
  error(error: CalcError): string {
    const dimension = (key: string): string => this.translate(`calc.dimensions.${key}`);
    let text: string;
    switch (error.kind) {
      case 'unreadable':
        text = this.translate('calc.errors.unreadable');
        break;
      case 'unknownName':
        text = this.translate('calc.errors.unknownName', { name: error.name });
        break;
      case 'reserved':
        text = this.translate('calc.errors.reserved', { word: error.word });
        break;
      case 'mismatch':
        text =
          error.unit === null
            ? this.translate('calc.errors.mismatch', {
                found: dimension(error.found),
                expected: dimension(error.expected),
              })
            : this.translate('calc.errors.mismatchUnit', {
                unit: this.unit(error.unit),
                expected: dimension(error.expected),
              });
        break;
      case 'currency':
        text = this.translate('calc.errors.currency');
        break;
      case 'notConvertible':
        text = this.translate('calc.errors.notConvertible', {
          from: dimension(error.from),
          to: error.to === null ? '%' : this.unit(error.to),
        });
        break;
      case 'divisionByZero':
      case 'nothingAbove':
      case 'noPrevious':
      case 'dateShift':
      case 'overflow':
      case 'unsupported':
        text = this.translate(`calc.errors.${error.kind}`);
        break;
    }
    return text.charAt(0).toUpperCase() + text.slice(1);
  }

  /**
   * The text with each result after it as a comment: pasted into another sheet, the comments
   * are read past and the same lines compute again.
   */
  copyText(sheet: string, answer: CalcSheet): string {
    return sheet
      .split('\n')
      .map((line, index) => {
        const row = answer.rows[index];
        if (row?.line.kind !== 'value') return line;
        const shown = this.value(row.line.value);
        const at = line.indexOf('//');
        const written = (at < 0 ? line : line.slice(0, at)).trimEnd();
        return `${written} // ${shown.approx ? '≈ ' : ''}${shown.text}`;
      })
      .join('\n');
  }

  private number(value: string, digits: { min: number; max: number }, signed = false): CalcText {
    const format = new Intl.NumberFormat(this.lang, {
      minimumFractionDigits: digits.min,
      maximumFractionDigits: digits.max,
      signDisplay: signed ? 'exceptZero' : 'auto',
    });
    return {
      text: format.format(Number(value)),
      approx: parts(value).fraction.length > digits.max,
    };
  }

  private date(iso: string): string {
    const [year, month, day] = iso.split('-').map(Number);
    return new Intl.DateTimeFormat(this.lang, {
      weekday: 'short',
      day: 'numeric',
      month: 'short',
      year: 'numeric',
    }).format(new Date(year ?? 1970, (month ?? 1) - 1, day ?? 1));
  }
}

function isMoney(unit: CalcUnitId): boolean {
  return unit === 'euro' || unit === 'dollar';
}
