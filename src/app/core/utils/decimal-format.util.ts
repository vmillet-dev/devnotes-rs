/** Between thousands: a narrow no-break space in French, a comma in English. */
const GROUPS: Record<string, string> = { fr: ' ', en: ',' };
const DECIMAL_MARKS: Record<string, string> = { fr: ',', en: '.' };

/**
 * An exact decimal as Rust wrote it (`-1234567.891`), written for the language on screen
 * (`-1 234 567,891`). Text in, text out: a float would round what Rust kept exact.
 */
export function formatDecimal(value: string, lang: string): string {
  const match = /^(-?)(\d+)(?:\.(\d+))?$/.exec(value);
  if (!match) return value;
  const [, sign, whole, fraction] = match;
  const grouped = whole!.replace(/\B(?=(\d{3})+(?!\d))/g, GROUPS[lang] ?? ',');
  return fraction === undefined
    ? `${sign}${grouped}`
    : `${sign}${grouped}${DECIMAL_MARKS[lang] ?? '.'}${fraction}`;
}

const SUPERSCRIPTS = '⁰¹²³⁴⁵⁶⁷⁸⁹';

/**
 * A value below one that rounding to `decimals` would write as 0, in scientific notation instead:
 * `0.000008` is `8 × 10⁻⁶`, its mantissa rounded half away from zero. Text in, text out; `null`
 * for what is no such value.
 */
export function formatScientific(
  value: string,
  lang: string,
  decimals: number,
): { text: string; exact: boolean } | null {
  const match = /^(-?)0\.(0*)([1-9]\d*)$/.exec(value);
  if (!match) return null;
  const [, sign, zeros, digits] = match as unknown as [string, string, string, string];
  const kept = digits.slice(0, decimals + 1);
  let exponent = -(zeros.length + 1);
  let mantissa = (BigInt(kept) + (Number(digits[decimals + 1] ?? '0') >= 5 ? 1n : 0n)).toString();
  // 9.99 rounded up is 10.0: one more power of ten, one digit fewer.
  if (mantissa.length > kept.length) {
    mantissa = mantissa.slice(0, -1);
    exponent += 1;
  }
  const fraction = mantissa.slice(1).replace(/0+$/, '');
  const written = fraction === '' ? mantissa[0] : `${mantissa[0]}${DECIMAL_MARKS[lang] ?? '.'}${fraction}`;
  const power = String(exponent)
    .replace('-', '⁻')
    .replace(/\d/g, (digit) => SUPERSCRIPTS[Number(digit)] ?? digit);
  return { text: `${sign}${written} × 10${power}`, exact: digits.length <= decimals + 1 };
}
