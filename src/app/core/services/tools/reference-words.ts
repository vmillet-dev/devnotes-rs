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

  return computed(() => kept());
}
