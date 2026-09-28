import { EnvironmentProviders, Injectable, Provider } from '@angular/core';
import { TRANSLOCO_TRANSPILER, Translation, TranslocoLoader, provideTransloco } from '@jsverse/transloco';
import { Observable, of } from 'rxjs';
import { PluralTranspiler } from '@core/services/i18n/plural-transpiler';
import { withAppName } from '@core/services/i18n/transloco-loader';
import en from '@core/services/i18n/translations/en.json';
import fr from '@core/services/i18n/translations/fr.json';

const TRANSLATIONS: Record<string, Translation> = { fr, en };

/**
 * Both files, synchronously: a spec asserts on text right after a render, before the
 * application's loader could have imported a chunk. Hence no `TranslocoTestingModule` either.
 */
@Injectable()
class SynchronousTranslocoLoader implements TranslocoLoader {
  getTranslation(lang: string): Observable<Translation> {
    return of(withAppName(TRANSLATIONS[lang] ?? {}));
  }
}

/**
 * ⚠️ The plural transpiler comes with it, exactly as `app.config.ts` provides it: without it
 * a spec asserting on a counted string reads the ICU source back instead of a sentence.
 */
export function provideTranslocoTesting(): (EnvironmentProviders | Provider)[] {
  return [
    ...provideTransloco({
      config: {
        availableLangs: ['fr', 'en'],
        defaultLang: 'fr',
        reRenderOnLangChange: true,
        missingHandler: { logMissingKey: false, useFallbackTranslation: false, allowEmpty: true },
      },
      loader: SynchronousTranslocoLoader,
    }),
    { provide: TRANSLOCO_TRANSPILER, useClass: PluralTranspiler },
  ];
}
