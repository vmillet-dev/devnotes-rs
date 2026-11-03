import { ReferenceRow, ReferenceSource } from '@core/services/tools/reference.model';
import { MARKDOWN_ENTRIES, MarkdownEntry, MarkdownWords } from './markdown-sheet.data';

export interface MarkdownRow extends ReferenceRow {
  readonly entry: MarkdownEntry;
  readonly what: string;
}

/** A space that matters is drawn `␣`, and copied as a space. */
const SPACE = '␣';

export const MARKDOWN_SHEET_REFERENCE: ReferenceSource<MarkdownWords, MarkdownRow> = {
  words: {
    fr: () => import('./markdown-sheet.fr.json'),
    en: () => import('./markdown-sheet.en.json'),
  },
  rows: (words, label) =>
    MARKDOWN_ENTRIES.map((entry) => {
      const what = words.entries[entry.syntax] ?? '';
      return {
        key: entry.syntax,
        group: entry.group,
        entry,
        what,
        ...(entry.syntax.includes(SPACE) && {
          copy: { value: entry.syntax.replaceAll(SPACE, ' '), what: entry.syntax },
        }),
        searchable: [entry.syntax, what, label(`tools.markdown-sheet.groups.${entry.group}`)],
      };
    }),
  title: (row) => row.key,
  line: (row) => row.what,
};
