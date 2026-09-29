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
  md: 'MD',
  txt: 'TXT',
};

/** Kept when the value received is not (or no longer) recognised. */
export const FALLBACK_LANGUAGE: LanguageTag = 'txt';

/** Narrows a free string — a `<select>`'s value, never data from the bridge. */
export function isLanguageTag(value: unknown): value is LanguageTag {
  return typeof value === 'string' && Object.hasOwn(LANGUAGE_LABELS, value);
}
