import { SearchQuery, openSearchPanel } from '@codemirror/search';
import { EditorState, StateEffect } from '@codemirror/state';
import { Command } from '@codemirror/view';
import { COUNT_CAP, MatchCount, SearchSpec } from '../search/search-target';

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

/** A CodeMirror query as the bar reads it. */
export function specOf(query: SearchQuery): SearchSpec {
  const { search, replace, caseSensitive, wholeWord, regexp } = query;
  return { search, replace, caseSensitive, wholeWord, regexp };
}

/** Asks an open bar for its replace row, the focus in it. */
export const showReplace = StateEffect.define<null>();

/** Ctrl+H: the bar, its replace row unfolded. */
export const openReplace: Command = (view) => {
  openSearchPanel(view);
  view.dispatch({ effects: showReplace.of(null) });
  return true;
};
