import { AppLocale } from '@core/services/i18n/locale.model';
import { toolMatches } from './tool.model';

/** A `Record`, so a language added to `APP_LOCALES` stops a reference compiling until it has its file. */
export type ReferenceWordLoaders<W> = Readonly<Record<AppLocale, () => Promise<{ default: W }>>>;

export interface ReferenceRow {
  /** Unique in its table, and what the row's button copies: `404`, `Content-Type`. */
  readonly key: string;
  readonly group: string;
  /** What a search reads, case and accents folded: the key, the name, the words on screen. */
  readonly searchable: readonly string[];
  /** What the row's button copies instead of the key, and its name aloud: a character, keyed by its code. */
  readonly copy?: { readonly value: string; readonly what: string };
}

export function matchesSearchable(row: ReferenceRow, query: string): boolean {
  return toolMatches(query, row.searchable);
}

/**
 * A reference as data, apart from its component: the home's search reads the rows its table
 * shows, built by the one function and found by the one matcher. « too many » finds the same
 * rows from either field.
 */
export interface ReferenceSource<W, R extends ReferenceRow> {
  /** The words in each language, files of the reference's own chunk. */
  readonly words: ReferenceWordLoaders<W>;
  /** `label` reads a main translation: a group's heading, the name of an action. */
  rows(words: W, label: (key: string) => string): readonly R[];
  /** What a row is called away from its table: « 418 I'm a teapot ». */
  title(row: R): string;
  /** What it means, in the words of the language on screen. */
  line(row: R): string;
  /** For a reference that reads a query its own way: a code in any base. */
  matches?(row: R, query: string): boolean;
}

/** Where a reference keeps its own search, for the session: the home writes it before opening one. */
export function referenceQueryKey(toolId: string): string {
  return `${toolId}.query`;
}
