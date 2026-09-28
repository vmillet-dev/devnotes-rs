import { Editor, Extension, JSONContent } from '@tiptap/core';
import { TableKit } from '@tiptap/extension-table';
import { TaskItem, TaskList } from '@tiptap/extension-list';
import { Placeholder } from '@tiptap/extensions';
import { Markdown } from '@tiptap/markdown';
import type { Node as ProseMirrorNode } from '@tiptap/pm/model';
import { Plugin, PluginKey } from '@tiptap/pm/state';
import { Decoration, DecorationSet } from '@tiptap/pm/view';
import StarterKit from '@tiptap/starter-kit';
import { LANGUAGE_LABELS, LanguageTag, isLanguageTag } from '@core/model/language.model';
import { highlightRanges } from '@notes/ui/code-viewer/highlighter';
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
  /** One level of indentation in a code block of that language, as the code field has it. */
  indent(language: LanguageTag): string;
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
  // A fence is never escaped: its tabs are tabs, and a `&#9;` typed in it is text.
  if (node.type === 'codeBlock') return node;

  return {
    ...node,
    ...(node.text !== undefined && { text: node.text.replaceAll('&#9;', '	') }),
    ...(node.content && { content: node.content.map(withTabs) }),
  };
}

export function languageOf(node: ProseMirrorNode): LanguageTag {
  const language: unknown = node.attrs['language'];
  return isLanguageTag(language) ? language : 'txt';
}

/**
 * Tab once lists (nesting) and tables (next cell) have passed on it: a character in the text,
 * not a way out of the field — in a code block, the code field's level. Shift+Tab takes back
 * the tab a line starts with.
 */
const tabCharacter = (indent: RichEditorHooks['indent']): Extension =>
  Extension.create({
    name: 'tabCharacter',
    priority: 50,
    addKeyboardShortcuts() {
      return {
        Tab: () => {
          const { parent } = this.editor.state.selection.$from;
          const text = parent.type.name === 'codeBlock' ? indent(languageOf(parent)) : '\t';
          return this.editor.commands.insertContent({ type: 'text', text });
        },
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

/**
 * A code block coloured by the code field's grammars. TipTap holds text, not HTML, so the
 * colours are decorations over it, and the language a label on the block.
 */
const CodeHighlight = Extension.create({
  name: 'codeHighlight',
  addProseMirrorPlugins() {
    return [
      new Plugin<DecorationSet>({
        key: new PluginKey('codeHighlight'),
        state: {
          init: (_, { doc }) => highlightCode(doc),
          apply: (tr, previous) => (tr.docChanged ? highlightCode(tr.doc) : previous),
        },
        props: {
          decorations(state) {
            return this.getState(state);
          },
        },
      }),
    ];
  },
});

function highlightCode(doc: ProseMirrorNode): DecorationSet {
  const decorations: Decoration[] = [];

  doc.descendants((node, position) => {
    if (node.type.name !== 'codeBlock') return true;

    const language = languageOf(node);
    decorations.push(
      Decoration.node(position, position + node.nodeSize, { 'data-language': LANGUAGE_LABELS[language] }),
    );
    for (const range of highlightRanges(node.textContent, language)) {
      const start = position + 1 + range.from;
      decorations.push(Decoration.inline(start, position + 1 + range.to, { class: range.classes }));
    }
    return false;
  });

  return DecorationSet.create(doc, decorations);
}

/** What the rich editor stores. Tables come out wrapped in blank lines; the body does not keep them. */
export function markdownOf(editor: Editor): string {
  return editor.getMarkdown().replace(/^\n+|\n+$/g, '');
}

/**
 * The editor of a Note: what GitHub's Markdown can hold and nothing more — no underline. A
 * code block keeps its language in its fence.
 */
export function createRichEditor(element: HTMLElement, markdown: string, hooks: RichEditorHooks): Editor {
  const editor = new Editor({
    element,
    extensions: [
      StarterKit.configure({
        underline: false,
        link: { openOnClick: false, autolink: true, defaultProtocol: 'https' },
      }),
      Placeholder.configure({ placeholder: hooks.placeholder }),
      MarkdownWithTabs,
      TableKit.configure({ table: { resizable: false } }),
      TaskList,
      TaskItem.configure({ nested: true }),
      tabCharacter((language) => hooks.indent(language)),
      CodeHighlight,
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
