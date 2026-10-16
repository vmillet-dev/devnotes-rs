import { Editor, Extension } from '@tiptap/core';
import type { Node as ProseMirrorNode } from '@tiptap/pm/model';
import { Plugin, PluginKey, Selection, TextSelection } from '@tiptap/pm/state';
import { Decoration, DecorationSet } from '@tiptap/pm/view';
import { COUNT_CAP, MatchCount, SearchSpec, SearchTarget } from '../search/search-target';
import { TextMatch, findMatches, replacementOf, searchPattern } from '../search/text-search';

interface SearchState {
  readonly spec: SearchSpec | null;
  readonly matches: readonly TextMatch[];
}

const searchKey = new PluginKey<SearchState>('textSearch');
const CLOSED: SearchState = { spec: null, matches: [] };

export type SearchKey = 'find' | 'replace' | 'next' | 'previous' | 'close';

/** The keys the code field's search answers to, read the same way here. */
export function searchKeyOf(event: KeyboardEvent): SearchKey | null {
  const mod = event.ctrlKey || event.metaKey;
  const key = event.key.toLowerCase();
  if (event.key === 'Escape') return 'close';
  if (event.key === 'F3' || (mod && key === 'g')) return event.shiftKey ? 'previous' : 'next';
  if (mod && key === 'f') return 'find';
  if (mod && key === 'h') return 'replace';
  return null;
}

/**
 * The matches in the text the reader sees: each block's characters with its marks unseen, so
 * `**Ship** it` holds « Ship it ». A line break or an image is a separator no match crosses.
 */
export function matchesIn(doc: ProseMirrorNode, spec: SearchSpec | null): TextMatch[] {
  const pattern = spec && searchPattern(spec);
  if (!pattern) return [];
  const found: TextMatch[] = [];
  doc.descendants((node, position) => {
    if (!node.isTextblock) return true;
    const at: number[] = [];
    let text = '';
    node.forEach((child, offset) => {
      const start = position + 1 + offset;
      const characters = child.isText ? (child.text ?? '') : '\n';
      for (let index = 0; index < characters.length; index += 1) at.push(start + index);
      text += characters;
    });
    for (const match of findMatches(text, pattern)) {
      found.push({ from: at[match.from]!, to: at[match.to - 1]! + 1, text: match.text });
    }
    return false;
  });
  return found;
}

/** The query and what it finds, kept in the editor's state; highlighted as the code field does. */
export const TextSearch = Extension.create({
  name: 'textSearch',
  addProseMirrorPlugins() {
    return [
      new Plugin<SearchState>({
        key: searchKey,
        state: {
          init: () => CLOSED,
          apply: (tr, previous) => {
            const spec = tr.getMeta(searchKey) as SearchSpec | null | undefined;
            if (spec === undefined && !tr.docChanged) return previous;
            const next = spec === undefined ? previous.spec : spec;
            return { spec: next, matches: matchesIn(tr.doc, next) };
          },
        },
        props: {
          decorations: (state) => {
            const { matches } = searchKey.getState(state) ?? CLOSED;
            if (matches.length === 0) return null;
            const { from, to } = state.selection;
            return DecorationSet.create(
              state.doc,
              matches.map((match) =>
                Decoration.inline(match.from, match.to, {
                  class: match.from === from && match.to === to ? 'search-match current' : 'search-match',
                }),
              ),
            );
          },
        },
      }),
    ];
  },
});

export function searchSpecOf(editor: Editor): SearchSpec | null {
  return (searchKey.getState(editor.state) ?? CLOSED).spec;
}

/** `null` closes the search: nothing highlighted. Never a step of undo. */
export function setSearchSpec(editor: Editor, spec: SearchSpec | null): void {
  editor.view.dispatch(editor.state.tr.setMeta(searchKey, spec).setMeta('addToHistory', false));
}

export interface RichSearchHooks {
  find(): void;
  replace(): void;
  close(): void;
}

/** The bar's questions, answered over the rich editor's document. */
export function richTextSearch(editor: Editor, hooks: RichSearchHooks): SearchTarget {
  const found = (): SearchState => searchKey.getState(editor.state) ?? CLOSED;
  const select = (match: TextMatch | undefined): void => {
    if (!match) return;
    const { tr, doc } = editor.state;
    editor.view.dispatch(tr.setSelection(TextSelection.create(doc, match.from, match.to)).scrollIntoView());
  };
  const after = (position: number): TextMatch | undefined => {
    const { matches } = found();
    return matches.find((match) => match.from >= position) ?? matches[0];
  };
  const before = (position: number): TextMatch | undefined => {
    const { matches } = found();
    return [...matches].reverse().find((match) => match.to <= position) ?? matches.at(-1);
  };
  const selected = (): TextMatch | undefined => {
    const { from, to } = editor.state.selection;
    return found().matches.find((match) => match.from === from && match.to === to);
  };

  return {
    size: () => editor.state.doc.content.size,
    caret: () => editor.state.selection.from,
    query: () =>
      found().spec ?? { search: '', replace: '', caseSensitive: false, wholeWord: false, regexp: false },
    setQuery: (spec) => setSearchSpec(editor, spec),
    seek: (_spec, origin) => select(after(origin)),
    count: () => countOf(found().matches, editor.state.selection),
    next: () => select(after(editor.state.selection.to)),
    previous: () => select(before(editor.state.selection.from)),
    replaceOne: () => {
      const match = selected();
      const { spec } = found();
      const pattern = spec && searchPattern(spec);
      if (!match || !spec || !pattern) {
        select(after(editor.state.selection.to));
        return;
      }
      const replacement = replacementOf(match, spec, pattern);
      editor.view.dispatch(editor.state.tr.insertText(replacement, match.from, match.to));
      select(after(match.from + replacement.length));
    },
    replaceAll: () => {
      const { spec, matches } = found();
      const pattern = spec && searchPattern(spec);
      if (!spec || !pattern || matches.length === 0) return;
      // From the end: each replacement leaves the positions before it where they were.
      const tr = editor.state.tr;
      for (const match of [...matches].reverse()) {
        tr.insertText(replacementOf(match, spec, pattern), match.from, match.to);
      }
      editor.view.dispatch(tr);
    },
    close: () => hooks.close(),
    handleKey: (event) => {
      const key = searchKeyOf(event);
      if (key === 'close') {
        // Before the dialog's own Escape, which would close the note with the bar.
        event.stopPropagation();
        hooks.close();
      } else if (key === 'next') {
        select(after(editor.state.selection.to));
      } else if (key === 'previous') {
        select(before(editor.state.selection.from));
      } else if (key === 'find') {
        hooks.find();
      } else if (key === 'replace') {
        hooks.replace();
      }
      return key !== null;
    },
  };
}

function countOf(matches: readonly TextMatch[], selection: Selection): MatchCount {
  const at = matches.findIndex((match) => match.from === selection.from && match.to === selection.to);
  return {
    total: Math.min(matches.length, COUNT_CAP),
    current: at < 0 || at >= COUNT_CAP ? null : at + 1,
    capped: matches.length > COUNT_CAP,
  };
}
