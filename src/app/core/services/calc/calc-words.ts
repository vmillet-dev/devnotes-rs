import { InjectionToken } from '@angular/core';
import { CALC_WORDS } from '@core/ipc/bindings';
import type { CalcWord, CalcWordEntry } from '@core/model/calc.model';

/** Rust's table of the words a sheet reads: the front writes no Calcul word of its own. */
export const CALC_WORD_TABLE = new InjectionToken<readonly CalcWordEntry[]>('CALC_WORD_TABLE', {
  providedIn: 'root',
  factory: () => CALC_WORDS,
});

/** Typed while writing, a shorter start proposes too much: `de` is no `demain`. */
const PROPOSED_FROM = 3;

function escaped(spelling: string): string {
  return spelling.replace(/[.*+?^${}()|[\]\\]/g, '\\$&').replace(/ /g, '\\s+');
}

/** What the completion reads of the table, in both languages or in the one on screen. */
export class CalcWords {
  constructor(private readonly table: readonly CalcWordEntry[]) {}

  /** Every spelling of these concepts, in both languages: a sheet reads either. */
  spellings(...words: readonly CalcWord[]): string[] {
    return this.table
      .filter((entry) => words.includes(entry.word))
      .flatMap((entry) => [...entry.fr, ...entry.en]);
  }

  /** A conversion word, then the start of a unit: what the units of the line's kind complete. */
  conversion(): RegExp {
    const words = this.spellings('convert', 'at').map(escaped).join('|');
    return new RegExp(`(?:^|\\s)(?:${words})\\s+(\\S*)$`, 'iu');
  }

  /**
   * The words the start typed may become, in the spelling of the language on screen: the values
   * of the sheet and the phrases of several words, which are the ones looked for. `explicit`
   * asks from the first letter.
   */
  complete(typed: string, lang: string, explicit = false): string[] {
    const start = typed.toLowerCase().replace(/’/g, "'");
    if (start.length < (explicit ? 1 : PROPOSED_FROM)) return [];
    const language = lang.startsWith('fr') ? 'fr' : 'en';
    const found = this.table
      .flatMap((entry) =>
        entry[language].filter((spelling) => entry.class === 'value' || spelling.includes(' ')),
      )
      .filter((spelling) => spelling.startsWith(start) && spelling !== start);
    return [...new Set(found)];
  }
}
