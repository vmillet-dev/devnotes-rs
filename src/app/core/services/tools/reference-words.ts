import { Signal, computed, inject, linkedSignal, resource } from '@angular/core';
import { toSignal } from '@angular/core/rxjs-interop';
import { TranslocoService } from '@jsverse/transloco';
import { DEFAULT_LOCALE, isAppLocale } from '@core/services/i18n/locale.model';
import { ReferenceRow, ReferenceSource, ReferenceWordLoaders } from './reference.model';

/**
 * A reference's words in the language on screen, from a file of its own imported with the tool's
 * chunk rather than the main translations: sixty descriptions weigh on nothing until the tool is
 * opened. The previous language's words stay until the next land; `null` before the first.
 * Built in an injection context.
 */
export function referenceWords<W>(loaders: ReferenceWordLoaders<W>): Signal<W | null> {
  const transloco = inject(TranslocoService);
  const lang = toSignal(transloco.langChanges$, { initialValue: transloco.getActiveLang() });

  const words = resource({
    params: () => {
      const active = lang();
      return isAppLocale(active) ? active : DEFAULT_LOCALE;
    },
    loader: async ({ params }) => (await loaders[params]()).default,
  });

  const kept = linkedSignal<W | undefined, W | null>({
    source: () => (words.hasValue() ? words.value() : undefined),
    computation: (fresh, previous) => fresh ?? previous?.value ?? null,
  });

  return kept.asReadonly();
}

/**
 * A reference's rows in the language on screen, none until its words have landed: what its
 * table shows, and what the home's search reads through the same source. Built in an injection
 * context.
 */
export function referenceRows<W, R extends ReferenceRow>(
  source: ReferenceSource<W, R>,
): Signal<readonly R[]> {
  const transloco = inject(TranslocoService);
  // Read so that the headings a row is found by follow the language, and its file once landed.
  const translation = toSignal(transloco.selectTranslation());
  const words = referenceWords(source.words);

  return computed(() => {
    translation();
    const loaded = words();
    return loaded === null ? [] : source.rows(loaded, (key) => transloco.translate(key));
  });
}

export interface ReferenceHeading<G extends string> {
  readonly id: G;
  readonly label: string;
}

/**
 * A reference's group headings in the language on screen, translated here rather than in the
 * template because a search reads them: « redirection » finds every 3xx. Built in an injection
 * context.
 */
export function referenceHeadings<G extends string>(
  ids: readonly G[],
  key: (id: G) => string,
): Signal<readonly ReferenceHeading<G>[]> {
  const transloco = inject(TranslocoService);
  // Read so that the headings follow the language, and the file once it has landed.
  const translation = toSignal(transloco.selectTranslation());

  return computed(() => {
    translation();
    return ids.map((id) => ({ id, label: transloco.translate(key(id)) }));
  });
}
