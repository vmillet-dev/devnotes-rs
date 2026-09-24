import { Injectable } from '@angular/core';
import { Translation, TranslocoLoader } from '@jsverse/transloco';
import { Observable, of } from 'rxjs';
import { APP_INFO } from '@core/services/app-info/app-info.service';
import en from './translations/en.json';
import fr from './translations/fr.json';

const TRANSLATIONS: Record<string, Translation> = { fr, en };

/**
 * Bundled at build time rather than fetched over HTTP. The files live outside
 * `src/assets` on purpose: the asset glob would ship a second, unread copy in `dist`.
 */
@Injectable({ providedIn: 'root' })
export class AppTranslocoLoader implements TranslocoLoader {
  getTranslation(lang: string): Observable<Translation> {
    // ⚠️ `app` is not a string anyone reads: it is what `{{app}}` resolves to. Transloco
    // falls back to a sibling key when an interpolation is not in the params, so the name
    // reaches every string without a call site passing it.
    return of({ ...(TRANSLATIONS[lang] ?? {}), app: APP_INFO.name });
  }
}
