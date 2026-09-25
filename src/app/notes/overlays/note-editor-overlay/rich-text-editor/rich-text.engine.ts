import { Editor, Extension, JSONContent } from '@tiptap/core';
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
    parent: { type?: string; content?: readonly unknown[] } | null,
  ): string;
}

/**
 * Reads back the `&#9;` a line starts with (see `escapeMarkdownText`): the parser decodes
 * four named entities and leaves the others as typed.
 */
const MarkdownWithTabs = Markdown.extend({
  onBeforeCreate(event) {
    this.parent?.(event);
    const manager = this.editor.markdown;
    if (!manager) return;

    const parse = manager.parse.bind(manager);
    manager.parse = (markdown: string) => withTabs(parse(markdown));
    // The first document is parsed by the call above, before `parse` is wrapped.
    const content = this.editor.options.content;
    if (content && typeof content === 'object' && !Array.isArray(content)) {
      this.editor.options.content = withTabs(content);
    }
  },
});

function withTabs(node: JSONContent): JSONContent {
  return {
    ...node,
    ...(node.text !== undefined && { text: node.text.replaceAll('&#9;', '	') }),
    ...(node.content && { content: node.content.map(withTabs) }),
  };
}

/**
 * Tab once lists (nesting) and tables (next cell) have passed on it: a character in the text,
 * not a way out of the field. Shift+Tab takes back the one a line starts with.
 */
const TabCharacter = Extension.create({
  name: 'tabCharacter',
  priority: 50,
  addKeyboardShortcuts() {
    return {
      Tab: () => this.editor.commands.insertContent({ type: 'text', text: '\t' }),
      'Shift-Tab': () =>
        this.editor.commands.command(({ tr, state }) => {
          const { $from } = state.selection;
          if ($from.parent.firstChild?.text?.startsWith('\t')) {
            tr.delete($from.start(), $from.start() + 1);
          }
          return true;
        }),
    };
  },
});

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
      MarkdownWithTabs,
      TableKit.configure({ table: { resizable: false } }),
      TaskList,
      TaskItem.configure({ nested: true }),
      TabCharacter,
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

  // A tick runs TaskItem's own `chain().focus()`: captured here, the focus comes before it.
  editor.view.dom.addEventListener('change', () => focusFirst(editor), { capture: true });
  encodeMinimally(editor);
  return editor;
}

/**
 * ⚠️ On WebKit, which Linux runs, `chain().focus()` focuses synchronously and the focus dispatches
 * before the chain: a note ending in a list gains its trailing paragraph, and the chain throws
 * "Applying a mismatched transaction". Focused beforehand, `focus()` has nothing left to do.
 */
export function focusFirst(editor: Editor): void {
  if (!editor.view.hasFocus()) {
    editor.view.focus();
  }
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

    return inCode ? text : escapeMarkdownText(text, startsLine(node, parent));
  };
}

/** First in its block, or after a line break: where Markdown reads leading blanks as layout. */
function startsLine(node: unknown, parent: { content?: readonly unknown[] } | null): boolean {
  const siblings = parent?.content ?? [];
  const at = siblings.indexOf(node);
  return at === 0 || (at > 0 && (siblings[at - 1] as { type?: string }).type === 'hardBreak');
}
