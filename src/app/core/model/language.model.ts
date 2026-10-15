import type { Language } from '@core/ipc/bindings';

/** Generated from the Rust `Language` enum, so a variant added there breaks the build here. */
export type LanguageTag = Language;

/** The key order is the editor select's. */
export const LANGUAGE_LABELS: Record<LanguageTag, string> = {
  json: 'JSON',
  js: 'JS',
  ts: 'TS',
  py: 'PY',
  rs: 'RS',
  go: 'GO',
  java: 'JAVA',
  cs: 'CS',
  php: 'PHP',
  c: 'C',
  sql: 'SQL',
  graphql: 'GRAPHQL',
  yml: 'YML',
  toml: 'TOML',
  xml: 'XML',
  html: 'HTML',
  css: 'CSS',
  scss: 'SCSS',
  sh: 'SH',
  ps1: 'PS1',
  kt: 'KT',
  rb: 'RB',
  swift: 'SWIFT',
  dart: 'DART',
  ex: 'EX',
  md: 'MD',
  txt: 'TXT',
};

/** What each format is called in full, in a sentence: the one place a language is named. */
export const LANGUAGE_FULL_NAMES: Record<LanguageTag, string> = {
  json: 'JSON',
  js: 'JavaScript',
  ts: 'TypeScript',
  py: 'Python',
  rs: 'Rust',
  go: 'Go',
  java: 'Java',
  cs: 'C#',
  php: 'PHP',
  c: 'C',
  sql: 'SQL',
  graphql: 'GraphQL',
  yml: 'YAML',
  toml: 'TOML',
  xml: 'XML',
  html: 'HTML',
  css: 'CSS',
  scss: 'SCSS',
  sh: 'Shell',
  ps1: 'PowerShell',
  kt: 'Kotlin',
  rb: 'Ruby',
  swift: 'Swift',
  dart: 'Dart',
  ex: 'Elixir',
  md: 'Markdown',
  txt: 'Text',
};

/**
 * The full name where the label is an abbreviation of something else — « YML », YAML — which a
 * menu shows as a hint and a filter also matches. « TXT » keeps its label alone.
 */
export const LANGUAGE_NAMES: Partial<Record<LanguageTag, string>> = Object.fromEntries(
  (Object.entries(LANGUAGE_FULL_NAMES) as [LanguageTag, string][]).filter(
    ([id, name]) => id !== 'txt' && name.toUpperCase() !== LANGUAGE_LABELS[id],
  ),
);

/** Kept when the value received is not (or no longer) recognised. */
export const FALLBACK_LANGUAGE: LanguageTag = 'txt';

/** Narrows a free string — a `<select>`'s value, never data from the bridge. */
export function isLanguageTag(value: unknown): value is LanguageTag {
  return typeof value === 'string' && Object.hasOwn(LANGUAGE_LABELS, value);
}
