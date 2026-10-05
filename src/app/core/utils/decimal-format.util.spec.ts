import { describe, expect, it } from 'vitest';
import { formatDecimal, formatScientific } from './decimal-format.util';

describe('formatDecimal', () => {
  it('groups the thousands and marks the decimals as each language does', () => {
    expect(formatDecimal('1500000000', 'fr')).toBe('1 500 000 000');
    expect(formatDecimal('1500000000', 'en')).toBe('1,500,000,000');
    expect(formatDecimal('-1234567.891', 'fr')).toBe('-1 234 567,891');
    expect(formatDecimal('0.931322574615478515625', 'en')).toBe('0.931322574615478515625');
    expect(formatDecimal('999', 'fr')).toBe('999');
  });

  it('leaves alone what is no plain decimal', () => {
    expect(formatDecimal('1e9', 'fr')).toBe('1e9');
    expect(formatDecimal('', 'en')).toBe('');
  });

  it('falls back to English marks for a language it does not know', () => {
    expect(formatDecimal('1234.5', 'de')).toBe('1,234.5');
  });
});

describe('formatScientific', () => {
  it('writes a value too small for its decimals as a power of ten', () => {
    expect(formatScientific('0.000008', 'fr', 3)).toEqual({ text: '8 × 10⁻⁶', exact: true });
    expect(formatScientific('0.00000123', 'en', 3)).toEqual({ text: '1.23 × 10⁻⁶', exact: true });
    expect(formatScientific('-0.0012', 'fr', 3)).toEqual({ text: '-1,2 × 10⁻³', exact: true });
  });

  it('rounds the mantissa to the decimals, and says it did', () => {
    expect(formatScientific('0.000012345', 'en', 2)).toEqual({ text: '1.23 × 10⁻⁵', exact: false });
    expect(formatScientific('0.0000999', 'en', 1)).toEqual({ text: '1 × 10⁻⁴', exact: false });
    expect(formatScientific('0.00000105', 'fr', 0)).toEqual({ text: '1 × 10⁻⁶', exact: false });
  });

  it('leaves alone what is no value below one', () => {
    expect(formatScientific('0', 'fr', 3)).toBeNull();
    expect(formatScientific('12.5', 'fr', 3)).toBeNull();
  });
});
