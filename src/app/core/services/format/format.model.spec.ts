import { describe, expect, it } from 'vitest';
import {
  DEFAULT_PRETTIER_SETTINGS,
  clampPrintWidth,
  prettierOptions,
  readPrettierSettings,
} from './format.model';

describe('readPrettierSettings', () => {
  it('answers the defaults for a library that never chose', () => {
    expect(readPrettierSettings(null)).toEqual(DEFAULT_PRETTIER_SETTINGS);
    expect(readPrettierSettings('{not json')).toEqual(DEFAULT_PRETTIER_SETTINGS);
  });

  it('keeps what it can read and defaults the rest', () => {
    const stored = JSON.stringify({
      printWidth: 80,
      indentation: 'tab',
      quotes: 'sideways',
      formatOnSave: 'yes',
    });

    expect(readPrettierSettings(stored)).toEqual({
      ...DEFAULT_PRETTIER_SETTINGS,
      printWidth: 80,
      indentation: 'tab',
    });
  });

  it('brings a width out of range back within it', () => {
    expect(readPrettierSettings(JSON.stringify({ printWidth: 9000 })).printWidth).toBe(200);
  });
});

describe('clampPrintWidth', () => {
  it('rounds, bounds, and refuses what is no number', () => {
    expect(clampPrintWidth(99.6)).toBe(100);
    expect(clampPrintWidth(3)).toBe(40);
    expect(clampPrintWidth(Number.NaN)).toBeNull();
  });
});

describe('prettierOptions', () => {
  it("follows the editor's indentation by default", () => {
    expect(prettierOptions(DEFAULT_PRETTIER_SETTINGS, '\t')).toMatchObject({ useTabs: true, tabWidth: 4 });
    expect(prettierOptions(DEFAULT_PRETTIER_SETTINGS, '    ')).toMatchObject({ useTabs: false, tabWidth: 4 });
  });

  it('writes what the library chose over it', () => {
    const settings = {
      ...DEFAULT_PRETTIER_SETTINGS,
      printWidth: 80,
      indentation: 'two' as const,
      quotes: 'double' as const,
      semicolons: false,
      trailingCommas: false,
    };

    expect(prettierOptions(settings, '\t')).toEqual({
      printWidth: 80,
      tabWidth: 2,
      useTabs: false,
      singleQuote: false,
      semi: false,
      trailingComma: 'none',
    });
  });
});
