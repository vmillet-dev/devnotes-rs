import { SearchQuery, openSearchPanel } from '@codemirror/search';
import { EditorState, StateEffect } from '@codemirror/state';
import { Command } from '@codemirror/view';
import { PreferencesService } from '@core/services/preferences/preferences.service';

/** Past this many, the count stops: typing in the field stays smooth on a 2 MB snippet. */
export const COUNT_CAP = 1000;

export interface MatchCount {
  readonly total: number;
  /** One-based; `null` while the selection is on none of them. */
  readonly current: number | null;
  readonly capped: boolean;
}

export function countMatches(query: SearchQuery, state: EditorState, cap = COUNT_CAP): MatchCount {
  if (!query.valid) return { total: 0, current: null, capped: false };
  const { from, to } = state.selection.main;
  const cursor = query.getCursor(state);
  let total = 0;
  let current: number | null = null;
  for (let next = cursor.next(); !next.done; next = cursor.next()) {
    total += 1;
    if (next.value.from === from && next.value.to === to) current = total;
    if (total >= cap) return { total, current, capped: true };
  }
  return { total, current, capped: false };
}

/** The first match from `origin` on, wrapping round to the start. */
export function firstMatchFrom(
  query: SearchQuery,
  state: EditorState,
  origin: number,
): { from: number; to: number } | null {
  if (!query.valid) return null;
  const after = query.getCursor(state, origin).next();
  if (!after.done) return after.value;
  const before = query.getCursor(state).next();
  return before.done ? null : before.value;
}

export type SearchOption = 'caseSensitive' | 'wholeWord' | 'regexp';

export type SearchOptions = Readonly<Record<SearchOption, boolean>>;

const OPTIONS_KEY = 'devnotes.editorSearchOptions';

const NO_OPTIONS: SearchOptions = { caseSensitive: false, wholeWord: false, regexp: false };

/** For the application, like full screen: a regex turned on stays on from note to note. */
export function readSearchOptions(preferences: PreferencesService): SearchOptions {
  try {
    const stored: unknown = JSON.parse(preferences.read(OPTIONS_KEY) ?? '{}');
    const value = (stored ?? {}) as Partial<Record<SearchOption, unknown>>;
    return {
      caseSensitive: value.caseSensitive === true,
      wholeWord: value.wholeWord === true,
      regexp: value.regexp === true,
    };
  } catch {
    return NO_OPTIONS;
  }
}

export function writeSearchOptions(preferences: PreferencesService, options: SearchOptions): void {
  preferences.write(OPTIONS_KEY, JSON.stringify(options));
}

/** Asks an open bar for its replace row, the focus in it. */
export const showReplace = StateEffect.define<null>();

/** Ctrl+H: the bar, its replace row unfolded. */
export const openReplace: Command = (view) => {
  openSearchPanel(view);
  view.dispatch({ effects: showReplace.of(null) });
  return true;
};
