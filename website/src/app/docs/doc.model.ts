/**
 * A page of the documentation as typed data, drawn by one renderer that never writes
 * markup from a string: what the page says can hold `{{field}}` or `<T>` without a single
 * escape, and nothing in it is ever parsed as HTML.
 */
export type Inline =
  | string
  | { readonly code: string }
  | { readonly keys: readonly string[] }
  | { readonly strong: string }
  /** A label as the application writes it on a button or a menu. */
  | { readonly ui: string }
  /** `to` is a path on this site (`/docs/…`) or an address elsewhere (`https://…`). */
  | { readonly link: string; readonly to: string };

export type Text = readonly Inline[];

export interface ShortcutRow {
  readonly keys: readonly string[];
  readonly does: string;
}

export type Block =
  | { readonly kind: 'p'; readonly text: Text }
  | { readonly kind: 'h2' | 'h3'; readonly id: string; readonly text: string }
  | { readonly kind: 'list'; readonly ordered: boolean; readonly items: readonly Text[] }
  | { readonly kind: 'code'; readonly title: string; readonly lang: string; readonly code: string }
  | { readonly kind: 'note'; readonly tone: 'tip' | 'warn'; readonly text: Text }
  | { readonly kind: 'keys'; readonly caption: string; readonly rows: readonly ShortcutRow[] }
  | {
      readonly kind: 'table';
      readonly caption: string;
      readonly head: readonly string[];
      readonly rows: readonly (readonly Text[])[];
    }
  /** The overview's list of every other page. */
  | { readonly kind: 'guides' };

export interface DocPage {
  readonly slug: string;
  /** The page's `<h1>`, and the start of its `<title>`. */
  readonly title: string;
  /** One sentence: the lede under the title, and the meta description. */
  readonly summary: string;
  readonly body: readonly Block[];
}

// Writers, so a page reads as prose rather than as object literals.

export const p = (...text: Inline[]): Block => ({ kind: 'p', text });
export const h2 = (id: string, text: string): Block => ({ kind: 'h2', id, text });
export const h3 = (id: string, text: string): Block => ({ kind: 'h3', id, text });
export const ul = (...items: Text[]): Block => ({ kind: 'list', ordered: false, items });
export const ol = (...items: Text[]): Block => ({ kind: 'list', ordered: true, items });
export const tip = (...text: Inline[]): Block => ({ kind: 'note', tone: 'tip', text });
export const warn = (...text: Inline[]): Block => ({ kind: 'note', tone: 'warn', text });
export const snippet = (title: string, lang: string, code: string): Block => ({
  kind: 'code',
  title,
  lang,
  code,
});
export const shortcuts = (caption: string, ...rows: [keys: string, does: string][]): Block => ({
  kind: 'keys',
  caption,
  rows: rows.map(([keys, does]) => ({ keys: keys.split(' '), does })),
});
/** A cell is one inline, or several. */
export const table = (
  caption: string,
  head: readonly string[],
  ...rows: (readonly (Inline | Text)[])[]
): Block => ({
  kind: 'table',
  caption,
  head,
  rows: rows.map((row) => row.map((cell) => (Array.isArray(cell) ? (cell as Text) : [cell as Inline]))),
});

export const code = (text: string): Inline => ({ code: text });
export const keys = (combination: string): Inline => ({ keys: combination.split(' ') });
export const b = (text: string): Inline => ({ strong: text });
export const ui = (label: string): Inline => ({ ui: label });
export const link = (text: string, to: string): Inline => ({ link: text, to });

/** A snippet's `{{field}}`, spelled once so the braces are never typed by hand. */
export const field = (name: string): Inline => ({ code: `{{${name}}}` });
