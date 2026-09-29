import { describe, expect, it } from 'vitest';
import { formatDecimal } from './decimal-format.util';

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
