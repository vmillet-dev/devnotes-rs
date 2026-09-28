import { Injectable, Signal, computed, effect, inject } from '@angular/core';
import { TranslocoService } from '@jsverse/transloco';
import { firstValueFrom } from 'rxjs';
import { LocaleChoice } from '@core/services/settings/app-settings.model';
import { SettingsStore } from '@core/services/settings/settings.store';
import { AppLocale, DEFAULT_LOCALE, isAppLocale, resolveSystemLocale } from './locale.model';

@Injectable({ providedIn: 'root' })
export class LocaleService {
  private readonly transloco = inject(TranslocoService);
  private readonly settings = inject(SettingsStore);

  /** What is on screen, `system` already resolved. */
  readonly activeLocale: Signal<AppLocale> = computed(() => {
    const active = this.transloco.activeLang();
    return isAppLocale(active) ? active : DEFAULT_LOCALE;
  });

  /** What the user picked, which the preferences panel shows back. */
  readonly preference: Signal<LocaleChoice> = this.settings.locale;

  constructor() {
    // `<html lang>` drives screen-reader pronunciation and typographic rules.
    effect(() => {
      document.documentElement.lang = this.activeLocale();
    });

    effect(() => {
      this.apply(this.settings.locale());
    });
  }

  /**
   * Called after `SettingsStore.restore()`: the effect above only flushes after the
   * first render, which would show the interface in one language then the other. It
   * resolves once that language's chunk is in, so the first render has its strings.
   */
  async restore(): Promise<void> {
    this.apply(this.settings.locale());
    await firstValueFrom(this.transloco.load(this.transloco.getActiveLang()));
  }

  setLocale(choice: LocaleChoice): void {
    this.settings.setLocale(choice);
  }

  private apply(choice: LocaleChoice): void {
    this.transloco.setActiveLang(choice === 'system' ? resolveSystemLocale() : choice);
  }
}
