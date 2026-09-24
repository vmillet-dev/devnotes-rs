import { Editor } from '@tiptap/core';
import { TableKit } from '@tiptap/extension-table';
import { TaskItem, TaskList } from '@tiptap/extension-list';
import { Placeholder } from '@tiptap/extensions';
import { Markdown } from '@tiptap/markdown';
import StarterKit from '@tiptap/starter-kit';
import { escapeMarkdownText } from './markdown-text';

export interface RichEditorHooks {
  readonly label: string;
  readonly placeholder: string;
  change(markdown: string): void;
  blur(): void;
  transaction(): void;
  /** `true` when handled, which keeps ProseMirror from pasting on its own. */
  paste(event: ClipboardEvent): boolean;
  click(event: MouseEvent, href: string | null): boolean;
  keydown(event: KeyboardEvent): boolean;
}

/** The private encoder this overrides, as the Markdown manager holds it. */
interface MarkdownEncoder {
  codeTypes: Set<string>;
  encodeTextForMarkdown(
    text: string,
    node: { marks?: (string | { type: string })[] },
    parent: { type?: string } | null,
  ): string;
}

/** What the rich editor stores. Tables come out wrapped in blank lines; the body does not keep them. */
export function markdownOf(editor: Editor): string {
  return editor.getMarkdown().replace(/^\n+|\n+$/g, '');
}

/**
 * The editor of a Text note: what GitHub's Markdown can hold and nothing more — no underline,
 * no code block, which the note's language already offers.
 */
export function createRichEditor(element: HTMLElement, markdown: string, hooks: RichEditorHooks): Editor {
  const editor = new Editor({
    element,
    extensions: [
      StarterKit.configure({
        codeBlock: false,
        underline: false,
        link: { openOnClick: false, autolink: true, defaultProtocol: 'https' },
      }),
      Placeholder.configure({ placeholder: hooks.placeholder }),
      Markdown,
      TableKit.configure({ table: { resizable: false } }),
      TaskList,
      TaskItem.configure({ nested: true }),
    ],
    content: markdown,
    contentType: 'markdown',
    editorProps: {
      attributes: {
        class: 'rich-text',
        role: 'textbox',
        'aria-multiline': 'true',
        'aria-label': hooks.label,
        spellcheck: 'false',
        'data-testid': 'editor-rich',
      },
      handlePaste: (_view, event) => hooks.paste(event),
      handleClick: (_view, _pos, event) =>
        hooks.click(event, (event.target as HTMLElement | null)?.closest('a')?.getAttribute('href') ?? null),
      handleKeyDown: (_view, event) => hooks.keydown(event),
    },
    onUpdate: ({ editor: updated }) => hooks.change(markdownOf(updated)),
    onTransaction: () => hooks.transaction(),
    onBlur: () => hooks.blur(),
  });

  encodeMinimally(editor);
  return editor;
}

/** ⚠️ A private method of `@tiptap/markdown`: `markdown-text.spec.ts` fails if an upgrade renames it. */
function encodeMinimally(editor: Editor): void {
  const encoder = editor.markdown as unknown as MarkdownEncoder | undefined;
  if (!encoder || typeof encoder.encodeTextForMarkdown !== 'function') {
    throw new Error('@tiptap/markdown no longer encodes text through encodeTextForMarkdown');
  }

  encoder.encodeTextForMarkdown = (text, node, parent) => {
    const inCode =
      (parent?.type !== undefined && encoder.codeTypes.has(parent.type)) ||
      (node.marks ?? []).some((mark) => encoder.codeTypes.has(typeof mark === 'string' ? mark : mark.type));

    return inCode ? text : escapeMarkdownText(text);
  };
}
