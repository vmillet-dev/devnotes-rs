import { ReferenceRow, ReferenceSource, matchesSearchable } from '@core/services/tools/reference.model';
import { REGEX_ENTRIES, RegexEntry, RegexWords } from './regex-sheet.data';

export interface RegexRow extends ReferenceRow {
  readonly entry: RegexEntry;
  readonly what: string;
  readonly jsNote: string | null;
  readonly pcreNote: string | null;
}

const METACHARACTERS = /[\\^$.*+?()[\]{}|]/;

/**
 * A query written like a pattern is looked for in the syntax, case and all: `\b` is not `\B`.
 * Words — « lookbehind », « paresseux » — are searched as everywhere else.
 */
export function matchesRegex(row: RegexRow, query: string): boolean {
  const wanted = query.trim();
  return METACHARACTERS.test(wanted) ? row.key.includes(wanted) : matchesSearchable(row, query);
}

export const REGEX_SHEET_REFERENCE: ReferenceSource<RegexWords, RegexRow> = {
  words: {
    fr: () => import('./regex-sheet.fr.json'),
    en: () => import('./regex-sheet.en.json'),
  },
  rows: (words, label) =>
    REGEX_ENTRIES.map((entry) => {
      const said = words.entries[entry.syntax];
      const what = said?.what ?? '';
      return {
        key: entry.syntax,
        group: entry.group,
        entry,
        what,
        jsNote: said?.js ?? null,
        pcreNote: said?.pcre ?? null,
        searchable: [
          entry.syntax,
          what,
          said?.js ?? '',
          said?.pcre ?? '',
          label(`tools.regex-sheet.groups.${entry.group}`),
        ],
      };
    }),
  title: (row) => row.key,
  line: (row) => row.what,
  matches: matchesRegex,
};
