/**
 * A page of the documentation, as `scripts/docs.mjs` reads it from `docs/content`: typed data,
 * drawn by one renderer that never writes markup from a string.
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
  /** A list of pages, as the overview's. */
  | { readonly kind: 'guides'; readonly pages: readonly string[] };

export interface DocPage {
  /** Its entry in `DOC_INDEX`, the same in every language. */
  readonly id: string;
  /** The page's `<h1>`, and the start of its `<title>`. */
  readonly title: string;
  /** One sentence: the lede under the title, and the meta description. */
  readonly summary: string;
  readonly body: readonly Block[];
}
