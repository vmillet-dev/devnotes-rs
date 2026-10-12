import { Signal, computed, inject, linkedSignal, resource } from '@angular/core';
import { toSignal } from '@angular/core/rxjs-interop';
import { TranslocoService } from '@jsverse/transloco';
import { DEFAULT_LOCALE, isAppLocale } from '@core/services/i18n/locale.model';
import { ReferenceRow, ReferenceSource, matchesSearchable } from './reference.model';
import { ToolDefinition } from './tool.model';

/** How many rows of one reference the home lists before « n autres ». */
export const HITS_PER_REFERENCE = 4;

/**
 * A single character is nearly every row of every reference: the references are read from the
 * second one on. Their own field, which searches one table, reads the first.
 */
const WORTH_SEARCHING = 2;

export interface ReferenceHit {
  /** The row's key in its table: what the reference is opened on. */
  readonly key: string;
  readonly title: string;
  readonly line: string;
}

export interface ReferenceHits {
  readonly tool: ToolDefinition;
  readonly hits: readonly ReferenceHit[];
  /** Found past the ones listed. */
  readonly more: number;
}

interface Loaded {
  readonly tool: ToolDefinition;
  readonly source: ReferenceSource<unknown, ReferenceRow>;
  readonly words: unknown;
}

/**
 * What `query` finds inside the references of `catalogue`, a reference at a time. ⚠️ Their data
 * and their words are loaded by the first thing typed, once a language, and not by the home
 * opening: sixty descriptions a reference weigh on nothing until someone searches. The rows
 * are built by each reference's own source and found by its own matcher, so the home finds
 * what the reference's field finds. Built in an injection context.
 */
export function referenceSearch(
  catalogue: readonly ToolDefinition[],
  query: Signal<string>,
): { readonly found: Signal<readonly ReferenceHits[]>; readonly loading: Signal<boolean> } {
  const transloco = inject(TranslocoService);
  const lang = toSignal(transloco.langChanges$, { initialValue: transloco.getActiveLang() });
  // Read so that the headings a row is found by follow the language, and its file once landed.
  const translation = toSignal(transloco.selectTranslation());

  /** Once something was typed it stays asked for: emptying the field unloads nothing. */
  const asked = linkedSignal<boolean, boolean>({
    source: () => query().trim() !== '',
    computation: (typed, previous) => typed || (previous?.value ?? false),
  });

  const loaded = resource({
    params: () => {
      if (!asked()) return undefined;
      const active = lang();
      return isAppLocale(active) ? active : DEFAULT_LOCALE;
    },
    loader: ({ params: locale }): Promise<readonly Loaded[]> =>
      Promise.all(
        catalogue.flatMap((tool) =>
          tool.reference === undefined
            ? []
            : [
                tool.reference().then(async (source) => ({
                  tool,
                  source,
                  words: (await source.words[locale]()).default,
                })),
              ],
        ),
      ),
  });

  /** The previous language's rows stay until the next land. */
  const kept = linkedSignal<readonly Loaded[] | undefined, readonly Loaded[]>({
    source: () => (loaded.hasValue() ? loaded.value() : undefined),
    computation: (fresh, previous) => fresh ?? previous?.value ?? [],
  });

  const tables = computed(() => {
    translation();
    return kept().map(({ tool, source, words }) => ({
      tool,
      source,
      rows: source.rows(words, (key) => transloco.translate(key)),
    }));
  });

  /** Asked for and neither landed nor failed: « nothing matches » would be said too soon. */
  const loading = computed(() => asked() && !loaded.hasValue() && loaded.error() === undefined);

  const found = computed<readonly ReferenceHits[]>(() => {
    const typed = query();
    if ([...typed.trim()].length < WORTH_SEARCHING) return [];

    return tables().flatMap(({ tool, source, rows }) => {
      const found = rows.filter((row) => source.matches?.(row, typed) ?? matchesSearchable(row, typed));
      if (found.length === 0) return [];
      return [
        {
          tool,
          hits: found.slice(0, HITS_PER_REFERENCE).map((row) => ({
            key: row.key,
            title: source.title(row),
            line: source.line(row),
          })),
          more: Math.max(0, found.length - HITS_PER_REFERENCE),
        },
      ];
    });
  });

  return { found, loading };
}
