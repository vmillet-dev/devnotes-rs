import { Injectable } from '@angular/core';
import { Translation, TranslocoLoader } from '@jsverse/transloco';
import { APP_INFO } from '@core/services/app-info/app-info.service';
import { AppLocale, isAppLocale } from './locale.model';

/** `Record`: a locale added to `APP_LOCALES` stops this compiling until it has its file. */
const TRANSLATIONS: Readonly<Record<AppLocale, () => Promise<{ default: Translation }>>> = {
  fr: () => import('./translations/fr.json'),
  en: () => import('./translations/en.json'),
};

/**
 * ⚠️ `app` is not a string anyone reads: it is what `{{app}}` resolves to. Transloco falls
 * back to a sibling key when an interpolation is not in the params, so the name reaches
 * every string without a call site passing it.
 */
export function withAppName(translation: Translation): Translation {
  return { ...translation, app: APP_INFO.name };
}

/**
 * One chunk per language, imported when it becomes active rather than fetched over HTTP. The
 * files live outside `src/assets` on purpose: the asset glob would ship a second, unread copy.
 */
@Injectable({ providedIn: 'root' })
export class AppTranslocoLoader implements TranslocoLoader {
  async getTranslation(lang: string): Promise<Translation> {
    return withAppName(isAppLocale(lang) ? (await TRANSLATIONS[lang]()).default : {});
  }
}
