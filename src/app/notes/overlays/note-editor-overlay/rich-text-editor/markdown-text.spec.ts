import { afterEach, describe, expect, it } from 'vitest';
import type { Editor } from '@tiptap/core';
import { escapeMarkdownText } from './markdown-text';
import { createRichEditor, markdownOf } from './rich-text.engine';

const NO_HOOKS = {
  label: '',
  placeholder: '',
  change: () => undefined,
  blur: () => undefined,
  transaction: () => undefined,
  paste: () => false,
  click: () => false,
  keydown: () => false,
};

describe('escapeMarkdownText', () => {
  it.each([
    ['snake_case_name', 'snake_case_name'],
    ['2 * 3', '2 * 3'],
    ['~/.ssh/config', '~/.ssh/config'],
    ['a -> b && c < d', 'a -> b && c < d'],
    ['C:\\Users\\me', 'C:\\Users\\me'],
    ['{{db_host}} and {{_private}}', '{{db_host}} and {{_private}}'],
  ])('leaves %j as typed', (text, stored) => {
    expect(escapeMarkdownText(text)).toBe(stored);
  });

  it.each([
    ['*not emphasis*', '\\*not emphasis\\*'],
    ['_not emphasis_', '\\_not emphasis\\_'],
    ['~~not struck~~', '\\~\\~not struck\\~~'],
    ['`not code`', '\\`not code\\`'],
    ['[not a link](x)', '\\[not a link](x)'],
    ['<div> tag', '\\<div> tag'],
    ['&amp; entity', '\\&amp; entity'],
    ['\\* escaped', '\\\\\\* escaped'],
  ])('escapes %j, which would read as syntax', (text, stored) => {
    expect(escapeMarkdownText(text)).toBe(stored);
  });

  it('writes a tab starting a line as an entity, and leaves the others as they are', () => {
    expect(escapeMarkdownText('\t\tindented', true)).toBe('&#9;&#9;indented');
    expect(escapeMarkdownText('a\tb', true)).toBe('a\tb');
    expect(escapeMarkdownText('\tafter bold', false)).toBe('\tafter bold');
  });
});

describe('the rich editor round trip', () => {
  let editors: Editor[] = [];

  afterEach(() => {
    editors.forEach((editor) => editor.destroy());
    editors = [];
  });

  function open(markdown: string): Editor {
    const editor = createRichEditor(document.createElement('div'), markdown, NO_HOOKS);
    editors.push(editor);
    return editor;
  }

  /** Typed text survives being stored and read again, character for character. */
  it.each([
    'ssh {{user}}@{{db_host}} -p {{port=22}}',
    'snake_case and 2 * 3 and ~/.ssh',
    'a -> b && c < d, <div> & &amp;',
    '*stars* _underscores_ ~~tildes~~ `ticks` [brackets]',
    'C:\\Users\\me and \\* a backslash',
    '\tindented, and\ta tab inside',
  ])('keeps %j as it was typed', (text) => {
    const editor = open('');
    editor.commands.setContent({
      type: 'doc',
      content: [{ type: 'paragraph', content: [{ type: 'text', text }] }],
    });

    const reread = open(markdownOf(editor));

    expect(reread.getText()).toBe(text);
  });

  it('keeps a tab that starts the line after a line break', () => {
    const editor = open('');
    editor.commands.setContent({
      type: 'doc',
      content: [
        {
          type: 'paragraph',
          content: [
            { type: 'text', text: 'first' },
            { type: 'hardBreak' },
            { type: 'text', text: '\tsecond' },
          ],
        },
      ],
    });

    expect(open(markdownOf(editor)).getText()).toBe('first\n\tsecond');
  });

  /** ⚠️ Fails if an upgrade of `@tiptap/markdown` stops going through the encoder this replaces. */
  it('stores a field and a snake_case name without a backslash', () => {
    const editor = open('');
    editor.commands.setContent({
      type: 'doc',
      content: [{ type: 'paragraph', content: [{ type: 'text', text: 'psql {{db_host}} snake_case' }] }],
    });

    expect(markdownOf(editor)).toBe('psql {{db_host}} snake_case');
  });

  it.each([
    '# Title\n\nSome **bold**, *italic*, ~~struck~~ and `code`.',
    '- one\n- two\n\n1. first\n2. second',
    '- [ ] to do\n- [x] done',
    '> quoted',
    'A [link](https://example.com) here.',
  ])('writes %j back as it read it', (markdown) => {
    expect(markdownOf(open(markdown))).toBe(markdown);
  });

  it('writes a table back as a table', () => {
    const stored = markdownOf(open('| host | port |\n| --- | --- |\n| db | 5432 |'));

    expect(markdownOf(open(stored))).toBe(stored);
    expect(stored).toContain('| host');
    expect(stored).toContain('| db');
  });
});
