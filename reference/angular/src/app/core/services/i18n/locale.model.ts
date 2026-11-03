/** The UI display languages — unrelated to `LanguageTag`, which colours notes. */
export const APP_LOCALES = ['fr', 'en'] as const;
export type AppLocale = (typeof APP_LOCALES)[number];

/** Which file answers when a key is missing from the other. */
export const DEFAULT_LOCALE: AppLocale = 'fr';

/** What `system` is worth when the machine speaks neither language. */
export const SYSTEM_FALLBACK_LOCALE: AppLocale = 'en';

export function isAppLocale(value: string | null): value is AppLocale {
  return (APP_LOCALES as readonly string[]).includes(value ?? '');
}

/** The WebView's report is as close to asking the OS as we get without a plugin. */
export function resolveSystemLocale(): AppLocale {
  const tags = navigator.languages?.length ? navigator.languages : [navigator.language];

  for (const tag of tags) {
    const primary = tag.split('-')[0]?.toLowerCase() ?? '';
    if (isAppLocale(primary)) return primary;
  }

  return SYSTEM_FALLBACK_LOCALE;
}
