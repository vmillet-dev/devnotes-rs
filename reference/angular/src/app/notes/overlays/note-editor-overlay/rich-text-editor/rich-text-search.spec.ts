import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import type { Editor } from '@tiptap/core';
import { SearchSpec, SearchTarget } from '../search/search-target';
import { createRichEditor, markdownOf } from './rich-text.engine';
import { richTextSearch, searchKeyOf } from './rich-text-search';

const spec = (search: string, options: Partial<SearchSpec> = {}): SearchSpec => ({
  search,
  replace: '',
  caseSensitive: false,
  wholeWord: false,
  regexp: false,
  ...options,
});

/** The adapter over a real TipTap document, as the bar drives it. */
describe('richTextSearch', () => {
  let host: HTMLElement;
  let editor: Editor;
  let target: SearchTarget;
  const hooks = { find: vi.fn(), replace: vi.fn(), close: vi.fn() };

  function mount(markdown: string): void {
    editor = createRichEditor(host, markdown, {
      label: '',
      placeholder: '',
      change: () => undefined,
      blur: () => undefined,
      transaction: () => undefined,
      paste: () => false,
      click: () => false,
      keydown: () => false,
      indent: () => '  ',
      embeds: { picture: () => Promise.resolve(null), missing: () => '' },
      diagrams: { draw: () => Promise.resolve({ svg: '' }), title: () => '', failed: () => '' },
    });
    target = richTextSearch(editor, hooks);
  }

  const selected = (): string =>
    editor.state.doc.textBetween(editor.state.selection.from, editor.state.selection.to);

  beforeEach(() => {
    host = document.createElement('div');
    document.body.appendChild(host);
  });

  afterEach(() => {
    editor.destroy();
    host.remove();
    vi.clearAllMocks();
  });

  /** In the words the reader sees: the bold around « Ship » is no part of the text. */
  it('counts across the marks, and not across a line', () => {
    mount('**Ship** it now, ship it.\n\nShip  \nit');

    target.setQuery(spec('ship it'));

    expect(target.count()).toEqual({ total: 2, current: null, capped: false });
    // Highlighted where they are: the first one in two pieces, one each side of the bold.
    expect(editor.view.dom.querySelectorAll('.search-match')).toHaveLength(3);
  });

  it('steps through the matches both ways, wrapping round, and says which one is selected', () => {
    mount('alpha beta alpha beta alpha');
    target.setQuery(spec('alpha'));

    target.seek(spec('alpha'), 3);
    expect(target.count().current).toBe(2);
    target.next();
    expect(target.count().current).toBe(3);
    target.next();
    expect(target.count().current).toBe(1);
    target.previous();
    expect(target.count().current).toBe(3);
    expect(editor.view.dom.querySelector('.search-match.current')?.textContent).toBe('alpha');
  });

  it('replaces the selected match and moves to the next, or selects one first', () => {
    mount('one fish, two fish');
    target.setQuery(spec('fish', { replace: 'cat' }));

    target.replaceOne();
    expect(selected()).toBe('fish');
    target.replaceOne();

    expect(markdownOf(editor)).toBe('one cat, two fish');
    expect(target.count()).toEqual({ total: 1, current: 1, capped: false });
  });

  /** One transaction: one Ctrl+Z puts every word back. */
  it('replaces every match as one step of undo, keeping the marks around them', () => {
    mount('**deploy** now, then deploy again');
    target.setQuery(spec('deploy', { replace: 'ship' }));

    target.replaceAll();
    expect(markdownOf(editor)).toBe('**ship** now, then ship again');

    editor.commands.undo();
    expect(markdownOf(editor)).toBe('**deploy** now, then deploy again');
  });

  it('reads a regular expression’s groups in what replaces it', () => {
    mount('v1 and v22');
    target.setQuery(spec(String.raw`v(\d+)`, { regexp: true, replace: 'version $1' }));

    target.replaceAll();

    expect(markdownOf(editor)).toBe('version 1 and version 22');
  });

  it('answers the code field’s keys, and keeps Escape from the dialog behind it', () => {
    mount('text');
    const escape = new KeyboardEvent('keydown', { key: 'Escape', bubbles: true });
    const stop = vi.spyOn(escape, 'stopPropagation');

    expect(target.handleKey(escape)).toBe(true);
    expect(target.handleKey(new KeyboardEvent('keydown', { key: 'h', ctrlKey: true }))).toBe(true);
    expect(target.handleKey(new KeyboardEvent('keydown', { key: 'a' }))).toBe(false);

    expect(stop).toHaveBeenCalled();
    expect(hooks.close).toHaveBeenCalledTimes(1);
    expect(hooks.replace).toHaveBeenCalledTimes(1);
    expect(searchKeyOf(new KeyboardEvent('keydown', { key: 'F3', shiftKey: true }))).toBe('previous');
  });
});
