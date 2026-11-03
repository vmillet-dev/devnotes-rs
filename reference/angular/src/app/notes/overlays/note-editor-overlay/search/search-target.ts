/** Past this many, the count stops: typing in the field stays smooth on a 2 MB snippet. */
export const COUNT_CAP = 1000;

export interface MatchCount {
  readonly total: number;
  /** One-based; `null` while the selection is on none of them. */
  readonly current: number | null;
  readonly capped: boolean;
}

/** What is searched for and what replaces it: `@codemirror/search`'s query, as plain data. */
export interface SearchSpec {
  readonly search: string;
  readonly replace: string;
  readonly caseSensitive: boolean;
  readonly wholeWord: boolean;
  readonly regexp: boolean;
}

/** What moved in the editor since the bar last looked. */
export interface SearchChange {
  /** A query set from outside the bar: Ctrl+F on a selection. */
  readonly query?: SearchSpec;
  /** Ctrl+H, the replace row asked for. */
  readonly replace?: boolean;
  /** An edit or a move, which changes the count. */
  readonly edited?: boolean;
}

/**
 * What the search bar asks of the editor it searches: CodeMirror answers with its own engine, the
 * rich editor with the text the reader sees. The bar draws, counts and waits; the editor finds,
 * highlights and replaces.
 */
export interface SearchTarget {
  /** The text's length, past which finding and counting wait for a pause in the typing. */
  size(): number;
  /** Where typing searches from: the caret when the bar opened. */
  caret(): number;
  query(): SearchSpec;
  /** Highlights what the query finds. */
  setQuery(spec: SearchSpec): void;
  /** Selects the first match from `origin` on, wrapping round to the start. */
  seek(spec: SearchSpec, origin: number): void;
  count(): MatchCount;
  next(): void;
  previous(): void;
  /** Replaces the selected match, or selects the next one. */
  replaceOne(): void;
  /** Every match, as one step of undo. */
  replaceAll(): void;
  close(): void;
  /** The editor's own search keys — F3, Ctrl+G, Ctrl+H, Escape — pressed in the bar's fields. */
  handleKey(event: KeyboardEvent): boolean;
}
