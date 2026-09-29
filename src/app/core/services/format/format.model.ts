import { LanguageTag } from '@core/model/language.model';

export type PrettierParser =
  'babel' | 'typescript' | 'json' | 'css' | 'scss' | 'html' | 'markdown' | 'yaml' | 'graphql';

/** `Record`, not `Partial`: a language added in Rust has to say whether Prettier formats it. */
export const PRETTIER_PARSERS: Readonly<Record<LanguageTag, PrettierParser | null>> = {
  json: 'json',
  js: 'babel',
  ts: 'typescript',
  py: null,
  rs: null,
  go: null,
  java: null,
  cs: null,
  php: null,
  c: null,
  sql: null,
  graphql: 'graphql',
  yml: 'yaml',
  toml: null,
  xml: null,
  html: 'html',
  css: 'css',
  scss: 'scss',
  sh: null,
  md: 'markdown',
  txt: null,
};

export interface PrettierOptions {
  readonly printWidth: number;
  readonly tabWidth: number;
  readonly useTabs: boolean;
  readonly singleQuote: boolean;
  readonly semi: boolean;
  readonly trailingComma: 'all' | 'none';
}

/** What crosses into the worker: text and settings, never code to run — it has no CSP. */
export interface FormatRequest {
  readonly text: string;
  readonly cursor: number;
  readonly parser: PrettierParser;
  readonly options: PrettierOptions;
}

export type FormatAnswer =
  | {
      readonly kind: 'formatted';
      readonly text: string;
      readonly cursor: number;
      /** Zero-based, in the formatted text. */
      readonly changedLines: readonly number[];
    }
  | { readonly kind: 'unchanged' }
  /** One-based, as Prettier reports them. */
  | { readonly kind: 'syntax'; readonly line: number; readonly column: number }
  /** A `{{…}}` did not come back as it went in, so nothing is applied. */
  | { readonly kind: 'fields' }
  | { readonly kind: 'failed' };

/** The width of a tab when the level is one: what the editor's Shift+Tab takes back. */
const TAB_WIDTH = 4;

/** One level of indentation, as the editor's Tab key writes it. */
export function prettierOptions(indent: string): PrettierOptions {
  return {
    printWidth: 100,
    tabWidth: indent === '\t' ? TAB_WIDTH : indent.length,
    useTabs: indent === '\t',
    singleQuote: true,
    semi: true,
    trailingComma: 'all',
  };
}
