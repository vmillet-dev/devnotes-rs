import { describe, expect, it } from 'vitest';
import { CALC_WORDS } from '@core/ipc/bindings';
import type { CalcWordEntry } from '@core/model/calc.model';
import { CalcWords } from './calc-words';

const TABLE: readonly CalcWordEntry[] = [
  { word: 'convert', class: 'syntax', fr: ['en', 'vers'], en: ['in', 'to'] },
  { word: 'at', class: 'syntax', fr: ['à'], en: ['at'] },
  { word: 'of', class: 'syntax', fr: ['de'], en: ['of', 'out of'] },
  { word: 'today', class: 'value', fr: ["aujourd'hui"], en: ['today'] },
  { word: 'total', class: 'value', fr: ['total', 'somme'], en: ['total', 'sum'] },
];

describe('CalcWords', () => {
  const words = new CalcWords(TABLE);

  it('finds a conversion word of either language before the start of a unit', () => {
    const conversion = words.conversion();
    expect(conversion.exec('3 h 20 min en m')?.[1]).toBe('m');
    expect(conversion.exec('1,5 GB to ')?.[1]).toBe('');
    expect(conversion.exec('5 km À ')?.[1]).toBe('');
    expect(conversion.exec('bientôt')).toBeNull();
  });

  it('proposes the values and the phrases of the table in the spelling of the language on screen', () => {
    expect(words.complete('auj', 'fr')).toEqual(["aujourd'hui"]);
    expect(words.complete('Aujourd’', 'fr')).toEqual(["aujourd'hui"]);
    expect(words.complete('tod', 'fr')).toEqual([]);
    expect(words.complete('tod', 'en')).toEqual(['today']);
    expect(words.complete('som', 'fr')).toEqual(['somme']);
    expect(words.complete('out', 'en')).toEqual(['out of']);
  });

  it('proposes nothing for a word already whole, a syntax word or a start too short to mean much', () => {
    expect(words.complete('total', 'fr')).toEqual([]);
    expect(words.complete('ve', 'fr')).toEqual([]);
    expect(words.complete('ver', 'fr')).toEqual([]);
    expect(words.complete('to', 'en')).toEqual([]);
    expect(words.complete('to', 'en', true)).toEqual(['today', 'total']);
  });

  it('reads the table Rust generated, every concept in both languages', () => {
    expect(CALC_WORDS.length).toBeGreaterThan(0);
    for (const entry of CALC_WORDS) {
      expect(entry.fr.length).toBeGreaterThan(0);
      expect(entry.en.length).toBeGreaterThan(0);
    }
  });
});
