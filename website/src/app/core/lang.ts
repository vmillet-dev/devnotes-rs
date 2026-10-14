/**
 * French at the root, English under `/en/`: one address per page per language, each written
 * to its own `index.html`. A page's text is a pair of dictionaries typed alike, so a key
 * missing from one language is a build error rather than a word left on screen.
 */
export type Lang = 'fr' | 'en';

export const OTHER: Readonly<Record<Lang, Lang>> = { fr: 'en', en: 'fr' };

/** Where `<html lang>`, Open Graph and the language switch read a language's names. */
export const LOCALES: Readonly<Record<Lang, { readonly og: string; readonly name: string }>> = {
  fr: { og: 'fr_FR', name: 'Français' },
  en: { og: 'en_US', name: 'English' },
};

export function langOf(path: string): Lang {
  return path === '/en' || path.startsWith('/en/') ? 'en' : 'fr';
}

export function homePath(lang: Lang): string {
  return lang === 'fr' ? '/' : '/en';
}

export function downloadPath(lang: Lang): string {
  return lang === 'fr' ? '/telecharger' : '/en/download';
}
