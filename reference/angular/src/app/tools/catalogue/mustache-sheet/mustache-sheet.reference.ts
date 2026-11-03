import { ReferenceRow, ReferenceSource } from '@core/services/tools/reference.model';
import { MUSTACHE_ENTRIES, MustacheEntry, MustacheWords } from './mustache-sheet.data';

export interface MustacheRow extends ReferenceRow {
  readonly entry: MustacheEntry;
  readonly what: string;
}

export const MUSTACHE_SHEET_REFERENCE: ReferenceSource<MustacheWords, MustacheRow> = {
  words: {
    fr: () => import('./mustache-sheet.fr.json'),
    en: () => import('./mustache-sheet.en.json'),
  },
  rows: (words, label) =>
    MUSTACHE_ENTRIES.map((entry) => {
      const what = words.entries[entry.syntax] ?? '';
      return {
        key: entry.syntax,
        group: entry.group,
        entry,
        what,
        searchable: [entry.syntax, what, label(`tools.mustache-sheet.groups.${entry.group}`)],
      };
    }),
  title: (row) => row.key,
  line: (row) => row.what,
};
