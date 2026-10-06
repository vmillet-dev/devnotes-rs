import { Signal, computed, inject, linkedSignal, resource } from '@angular/core';
import { toSignal } from '@angular/core/rxjs-interop';
import { TranslocoService } from '@jsverse/transloco';
import { AppLocale, DEFAULT_LOCALE, isAppLocale } from '@core/services/i18n/locale.model';

/** A `Record`, so a language added to `APP_LOCALES` stops a reference compiling until it has its file. */
export type ReferenceWordLoaders<W> = Readonly<Record<AppLocale, () => Promise<{ default: W }>>>;

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

/** Where a search finds a row by its group: its heading. */
export function headingOf<G extends string>(headings: readonly ReferenceHeading<G>[], id: G): string {
  return headings.find((heading) => heading.id === id)?.label ?? '';
}
