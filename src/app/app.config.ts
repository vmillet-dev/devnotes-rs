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
import { PluralTranspiler } from '@core/services/i18n/plural-transpiler';
import { AppTranslocoLoader } from '@core/services/i18n/transloco-loader';
import { startApplication } from '@core/services/startup/start-application';
import { UpdateStore } from '@core/services/updates/update.store';

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

    // French keeps the singular at zero where English does not, and four of these
    // strings carry three independent counts in one sentence, each with its own agreement.
    // A key per form would have meant eight variants of those alone.
    //
    // ⚠️ Ours rather than `@jsverse/transloco-messageformat`, which compiles each message
    // with `new Function` — the CSP here is `script-src 'self'`, and the application boots
    // onto an error banner with it. See `plural-transpiler.ts`.
    { provide: TRANSLOCO_TRANSPILER, useClass: PluralTranspiler },

    // One initialiser for the whole sequence rather than several: Angular starts them
    // together and awaits their promises as a block, so a later step would read a
    // still-empty cache.
    provideAppInitializer(startApplication),

    // ⚠️ The promise is deliberately not returned: Angular awaits an initialiser's, and
    // the application would sit on a blank screen for the length of a network call.
    provideAppInitializer(() => {
      void inject(UpdateStore).check();
    }),

    { provide: ErrorHandler, useClass: AppErrorHandler },
  ],
};
