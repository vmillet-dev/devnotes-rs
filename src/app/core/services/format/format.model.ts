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

/** `editor` is the Tab key's own level, whatever the editor's preference makes it. */
export type PrettierIndentation = 'editor' | 'two' | 'four' | 'tab';

/** The library's, like its arrangement: every snippet of a library formats alike. */
export interface PrettierSettings {
  readonly printWidth: number;
  readonly indentation: PrettierIndentation;
  readonly quotes: 'single' | 'double';
  readonly semicolons: boolean;
  readonly trailingCommas: boolean;
  readonly formatOnSave: boolean;
}

export const PRETTIER_INDENTATIONS: readonly PrettierIndentation[] = ['editor', 'two', 'four', 'tab'];
export const PRINT_WIDTH_RANGE = { min: 40, max: 200 } as const;

export const DEFAULT_PRETTIER_SETTINGS: PrettierSettings = {
  printWidth: 100,
  indentation: 'editor',
  quotes: 'single',
  semicolons: true,
  trailingCommas: true,
  formatOnSave: false,
};

/** A width typed out of range comes back to the nearest bound; one that is no number, to none. */
export function clampPrintWidth(value: number): number | null {
  if (!Number.isFinite(value)) return null;
  return Math.min(PRINT_WIDTH_RANGE.max, Math.max(PRINT_WIDTH_RANGE.min, Math.round(value)));
}

/** A preference file is trusted no further than it reads: what it does not name stays default. */
export function readPrettierSettings(stored: string | null): PrettierSettings {
  let value: Partial<Record<keyof PrettierSettings, unknown>>;
  try {
    value = stored === null ? {} : (JSON.parse(stored) as typeof value);
  } catch {
    return DEFAULT_PRETTIER_SETTINGS;
  }

  const fallback = DEFAULT_PRETTIER_SETTINGS;
  const flag = (key: 'semicolons' | 'trailingCommas' | 'formatOnSave'): boolean => {
    const stored = value[key];
    return typeof stored === 'boolean' ? stored : fallback[key];
  };
  const width = typeof value.printWidth === 'number' ? clampPrintWidth(value.printWidth) : null;
  return {
    printWidth: width ?? fallback.printWidth,
    indentation: PRETTIER_INDENTATIONS.find((each) => each === value.indentation) ?? fallback.indentation,
    quotes: value.quotes === 'double' || value.quotes === 'single' ? value.quotes : fallback.quotes,
    semicolons: flag('semicolons'),
    trailingCommas: flag('trailingCommas'),
    formatOnSave: flag('formatOnSave'),
  };
}

/** The width of a tab when the level is one: what the editor's Shift+Tab takes back. */
const TAB_WIDTH = 4;

const INDENTS: Readonly<Record<Exclude<PrettierIndentation, 'editor'>, string>> = {
  two: '  ',
  four: '    ',
  tab: '\t',
};

/** `editorIndent` is one level as the editor's Tab key writes it. */
export function prettierOptions(settings: PrettierSettings, editorIndent: string): PrettierOptions {
  const indent = settings.indentation === 'editor' ? editorIndent : INDENTS[settings.indentation];
  return {
    printWidth: settings.printWidth,
    tabWidth: indent === '\t' ? TAB_WIDTH : indent.length,
    useTabs: indent === '\t',
    singleQuote: settings.quotes === 'single',
    semi: settings.semicolons,
    trailingComma: settings.trailingCommas ? 'all' : 'none',
  };
}
