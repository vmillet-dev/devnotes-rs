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

/** Runs written as they are typed: a key, or a piece of code. */
const AS_TYPED = new Set(['code', 'keys']);

/**
 * A French dictionary set as French is printed: no line break before `: ; ? ! »`, nor after `«`.
 * It goes through every word of the dictionary, a function's answer included; code and keys
 * are left as typed.
 */
export function french<T>(text: T): T {
  if (typeof text === 'string') {
    const spaced = text
      .replace(/ :/g, '\u00a0:')
      .replace(/ ([;?!»])/g, '\u202f$1')
      .replace(/« /g, '«\u202f');
    return spaced as T;
  }
  if (typeof text === 'function') {
    const answer = text as (...args: never[]) => unknown;
    return ((...args: never[]) => french(answer(...args))) as T;
  }
  if (Array.isArray(text)) return text.map((each: unknown) => french(each)) as T;
  if (text && typeof text === 'object') {
    const entries = Object.entries(text as Record<string, unknown>).map(([key, value]) => [
      key,
      AS_TYPED.has(key) ? value : french(value),
    ]);
    return Object.fromEntries(entries) as T;
  }
  return text;
}

export function langOf(path: string): Lang {
  return path === '/en' || path.startsWith('/en/') ? 'en' : 'fr';
}

export function homePath(lang: Lang): string {
  return lang === 'fr' ? '/' : '/en';
}

export function downloadPath(lang: Lang): string {
  return lang === 'fr' ? '/telecharger' : '/en/download';
}

export function accessibilityPath(lang: Lang): string {
  return lang === 'fr' ? '/accessibilite' : '/en/accessibility';
}
