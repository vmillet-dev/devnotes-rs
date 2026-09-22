import {
  ApplicationConfig,
  ErrorHandler,
  inject,
  provideAppInitializer,
  provideBrowserGlobalErrorListeners,
  provideZonelessChangeDetection,
} from '@angular/core';
import { provideRouter, withHashLocation } from '@angular/router';
import { TRANSLOCO_TRANSPILER, provideTransloco } from '@jsverse/transloco';

import { routes } from './app.routes';
import { AppErrorHandler } from '@core/services/errors/app-error-handler';
import { APP_LOCALES, DEFAULT_LOCALE } from '@core/services/i18n/locale.model';
import { LocaleService } from '@core/services/i18n/locale.service';
import { PluralTranspiler } from '@core/services/i18n/plural-transpiler';
import { AppTranslocoLoader } from '@core/services/i18n/transloco-loader';
import { AutostartService } from '@core/services/autostart/autostart.service';
import { PreferencesService } from '@core/services/preferences/preferences.service';
import { LibrariesStore } from '@core/state/libraries.store';
import { SettingsStore } from '@core/services/settings/settings.store';
import { GlobalShortcutsService } from '@core/services/shortcuts/global-shortcuts.service';
import { TrayService } from '@core/services/tray/tray.service';
import { UpdateStore } from '@core/services/updates/update.store';
import { VaultStore } from '@core/state/vault.store';
import { WindowBehaviorService } from '@core/services/window/window-behavior.service';

export const appConfig: ApplicationConfig = {
  providers: [
    provideBrowserGlobalErrorListeners(),
    provideZonelessChangeDetection(),

    // Hash routing: the files are served from Tauri's internal protocol, where a
    // reloaded deep URL has no server to rewrite it to index.html.
    provideRouter(routes, withHashLocation()),

    provideTransloco({
      config: {
        availableLangs: [...APP_LOCALES],
        defaultLang: DEFAULT_LOCALE,
        fallbackLang: DEFAULT_LOCALE,
        reRenderOnLangChange: true,
      },
      loader: AppTranslocoLoader,
    }),

    // ⚠️ French keeps the singular at zero where English does not, and four of these
    // strings carry three independent counts in one sentence, each with its own agreement.
    // A key per form would have meant eight variants of those alone.
    //
    // ⚠️ Ours rather than `@jsverse/transloco-messageformat`, which compiles each message
    // with `new Function` — the CSP here is `script-src 'self'`, and the application boots
    // onto an error banner with it. See `plural-transpiler.ts`.
    { provide: TRANSLOCO_TRANSPILER, useClass: PluralTranspiler },

    // ⚠️ One initialiser for both steps rather than two chained: Angular starts them
    // together and awaits their promises as a block, so `restore()` would read a
    // still-empty cache.
    provideAppInitializer(async () => {
      // ⚠️ Everything is injected before the first `await`: an `inject()` after one
      // leaves the injection context and fails the bootstrap (NG0203).
      const preferences = inject(PreferencesService);
      const locale = inject(LocaleService);
      const settings = inject(SettingsStore);
      const tray = inject(TrayService);
      const shortcuts = inject(GlobalShortcutsService);
      const windowBehavior = inject(WindowBehaviorService);
      const autostart = inject(AutostartService);
      const vault = inject(VaultStore);
      const libraries = inject(LibrariesStore);

      await preferences.hydrate();
      // ⚠️ Before the first render, and before `locale.restore()`, which reads the
      // language out of it.
      settings.restore();
      locale.restore();
      // ⚠️ After `restore()`: the front creates the tray by giving it its labels, and
      // earlier would push the default language and a setting the user had changed.
      tray.start();

      // ⚠️ Before the vault and before the first render: it says which library is open,
      // and opens that library's own preference file. The samples marker and the view
      // each space was left on are read out of it, and reading the wrong one seeds a
      // library that is full or opens another library's board.
      await libraries.load();

      // ⚠️ Before the first render: the shell renders nothing at all until this answers,
      // rather than flashing a canvas it is about to replace with an unlock screen.
      await vault.load();

      shortcuts.start();
      windowBehavior.start();
      // Not awaited: asking the system must not delay the first render.
      void autostart.start();
    }),

    // ⚠️ The promise is deliberately not returned: Angular awaits an initialiser's, and
    // the application would sit on a blank screen for the length of a network call.
    provideAppInitializer(() => {
      void inject(UpdateStore).check();
    }),

    { provide: ErrorHandler, useClass: AppErrorHandler },
  ],
};
