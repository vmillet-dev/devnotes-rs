import {
  ChangeDetectionStrategy,
  Component,
  DestroyRef,
  ElementRef,
  Injector,
  afterNextRender,
  computed,
  effect,
  inject,
  input,
  output,
  signal,
  untracked,
  viewChild,
} from '@angular/core';
import { TranslocoPipe } from '@jsverse/transloco';
import type { Editor } from '@tiptap/core';
import { IconComponent, IconName } from '@shared/icon/icon.component';
import { createRichEditor } from './rich-text.engine';

type Action =
  | 'h1'
  | 'h2'
  | 'h3'
  | 'bold'
  | 'italic'
  | 'strike'
  | 'code'
  | 'bullet'
  | 'ordered'
  | 'tasks'
  | 'quote'
  | 'table'
  | 'rowAfter'
  | 'columnAfter'
  | 'deleteRow'
  | 'deleteColumn'
  | 'deleteTable';

interface Tool {
  readonly action: Action;
  readonly labelKey: string;
  readonly icon?: IconName;
  readonly text?: string;
  /** What `isActive` asks, when the tool has a state to show. */
  readonly active?: readonly [string, Record<string, unknown>?];
}

/** Grouped as they are separated on screen. */
const TOOLS: readonly (readonly Tool[])[] = [
  [
    { action: 'h1', labelKey: 'editor.rich.heading1', text: 'H1', active: ['heading', { level: 1 }] },
    { action: 'h2', labelKey: 'editor.rich.heading2', text: 'H2', active: ['heading', { level: 2 }] },
    { action: 'h3', labelKey: 'editor.rich.heading3', text: 'H3', active: ['heading', { level: 3 }] },
  ],
  [
    { action: 'bold', labelKey: 'editor.rich.bold', icon: 'bold', active: ['bold'] },
    { action: 'italic', labelKey: 'editor.rich.italic', icon: 'italic', active: ['italic'] },
    { action: 'strike', labelKey: 'editor.rich.strike', icon: 'strike', active: ['strike'] },
    { action: 'code', labelKey: 'editor.rich.code', icon: 'code', active: ['code'] },
  ],
  [
    { action: 'bullet', labelKey: 'editor.rich.bulletList', icon: 'list-bullet', active: ['bulletList'] },
    { action: 'ordered', labelKey: 'editor.rich.orderedList', icon: 'list-ordered', active: ['orderedList'] },
    { action: 'tasks', labelKey: 'editor.rich.taskList', icon: 'list-tasks', active: ['taskList'] },
    { action: 'quote', labelKey: 'editor.rich.quote', icon: 'quote', active: ['blockquote'] },
  ],
];

/** Shown while the caret is in a table, and only then. */
const TABLE_TOOLS: readonly Tool[] = [
  { action: 'rowAfter', labelKey: 'editor.rich.addRow' },
  { action: 'columnAfter', labelKey: 'editor.rich.addColumn' },
  { action: 'deleteRow', labelKey: 'editor.rich.deleteRow' },
  { action: 'deleteColumn', labelKey: 'editor.rich.deleteColumn' },
  { action: 'deleteTable', labelKey: 'editor.rich.deleteTable' },
];

/**
 * A Text note's body, formatted as it is typed and stored as Markdown.
 *
 * ⚠️ Holds no draft of its own: `content` is the editor overlay's, and what this emits is
 * written back into it. A `content` it did not emit — another note, a restored revision, a
 * paste read as prose — replaces the document, outside the undo history.
 */
@Component({
  selector: 'app-rich-text-editor',
  imports: [TranslocoPipe, IconComponent],
  templateUrl: './rich-text-editor.component.html',
  styleUrl: './rich-text-editor.component.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class RichTextEditorComponent {
  readonly content = input.required<string>();
  readonly label = input('');
  readonly placeholder = input('');

  readonly changed = output<string>();
  readonly blurred = output<void>();
  /** Plain text pasted into an empty note: whether it is code is Rust's to say. */
  readonly pastedIntoEmpty = output<string>();
  readonly imagePasted = output<void>();
  /** A Ctrl+click on a link: a plain click is for editing its text. */
  readonly linkOpened = output<string>();

  protected readonly tools = TOOLS;
  protected readonly tableTools = TABLE_TOOLS;

  /** The address being typed, while the link form is open. */
  protected readonly linkDraft = signal<string | null>(null);

  /** Bumped on every transaction, which is what makes the toolbar's state follow the caret. */
  private readonly revision = signal(0);
  private readonly surface = viewChild.required<ElementRef<HTMLElement>>('surface');
  private readonly linkField = viewChild<ElementRef<HTMLInputElement>>('linkField');
  private readonly injector = inject(Injector);
  private editor: Editor | null = null;
  /** The Markdown the document last held: what it emitted or was given. */
  private written = '';

  protected readonly inTable = computed(() => {
    this.revision();
    return this.editor?.isActive('table') ?? false;
  });

  constructor() {
    afterNextRender(() => {
      this.written = this.content();
      this.editor = createRichEditor(this.surface().nativeElement, this.written, {
        label: this.label(),
        placeholder: this.placeholder(),
        change: (markdown) => {
          this.written = markdown;
          this.changed.emit(markdown);
        },
        blur: () => this.blurred.emit(),
        transaction: () => this.revision.update((count) => count + 1),
        paste: (event) => this.onPaste(event),
        click: (event, href) => this.onClick(event, href),
        keydown: (event) => this.onKeydown(event),
      });
      this.revision.update((count) => count + 1);
    });

    effect(() => {
      const content = this.content();
      untracked(() => this.load(content));
    });

    inject(DestroyRef).onDestroy(() => this.editor?.destroy());
  }

  isActive(tool: Tool): boolean {
    this.revision();
    return tool.active ? (this.editor?.isActive(tool.active[0], tool.active[1]) ?? false) : false;
  }

  hasFocus(): boolean {
    return this.editor?.isFocused ?? false;
  }

  blur(): void {
    this.editor?.commands.blur();
  }

  protected run(action: Action): void {
    const chain = this.editor?.chain().focus();
    if (!chain) return;

    switch (action) {
      case 'h1':
        chain.toggleHeading({ level: 1 }).run();
        break;
      case 'h2':
        chain.toggleHeading({ level: 2 }).run();
        break;
      case 'h3':
        chain.toggleHeading({ level: 3 }).run();
        break;
      case 'bold':
        chain.toggleBold().run();
        break;
      case 'italic':
        chain.toggleItalic().run();
        break;
      case 'strike':
        chain.toggleStrike().run();
        break;
      case 'code':
        chain.toggleCode().run();
        break;
      case 'bullet':
        chain.toggleBulletList().run();
        break;
      case 'ordered':
        chain.toggleOrderedList().run();
        break;
      case 'tasks':
        chain.toggleTaskList().run();
        break;
      case 'quote':
        chain.toggleBlockquote().run();
        break;
      case 'table':
        chain.insertTable({ rows: 3, cols: 3, withHeaderRow: true }).run();
        break;
      case 'rowAfter':
        chain.addRowAfter().run();
        break;
      case 'columnAfter':
        chain.addColumnAfter().run();
        break;
      case 'deleteRow':
        chain.deleteRow().run();
        break;
      case 'deleteColumn':
        chain.deleteColumn().run();
        break;
      case 'deleteTable':
        chain.deleteTable().run();
        break;
    }
  }

  protected openLinkForm(): void {
    const href = (this.editor?.getAttributes('link')['href'] as string | undefined) ?? '';
    this.linkDraft.set(href || 'https://');
    afterNextRender(() => this.linkField()?.nativeElement.select(), { injector: this.injector });
  }

  /** An empty address takes the link off, which is how one is removed. */
  protected applyLink(): void {
    const href = this.linkDraft()?.trim() ?? '';
    const chain = this.editor?.chain().focus().extendMarkRange('link');
    if (chain) {
      (href && href !== 'https://' ? chain.setLink({ href }) : chain.unsetLink()).run();
    }
    this.linkDraft.set(null);
  }

  /** Escape closes the form, and only the form: the editor stays open behind it. */
  protected closeLinkForm(event: Event): void {
    event.stopPropagation();
    this.linkDraft.set(null);
    this.editor?.commands.focus();
  }

  private load(content: string): void {
    if (!this.editor || content === this.written) return;

    this.written = content;
    this.editor
      .chain()
      .setMeta('addToHistory', false)
      .setContent(content, { contentType: 'markdown', emitUpdate: false })
      .run();
  }

  private onPaste(event: ClipboardEvent): boolean {
    const data = event.clipboardData;
    if (!data) return false;

    const text = data.getData('text/plain');
    const hasImage =
      data.types.some((type) => type.startsWith('image/')) ||
      [...data.files].some((file) => file.type.startsWith('image/'));
    if (!text && hasImage) {
      this.imagePasted.emit();
      return true;
    }
    if (text.trim() && this.editor?.isEmpty) {
      this.pastedIntoEmpty.emit(text);
      return true;
    }

    return false;
  }

  private onClick(event: MouseEvent, href: string | null): boolean {
    if (!href || !(event.ctrlKey || event.metaKey)) return false;

    event.preventDefault();
    this.linkOpened.emit(href);
    return true;
  }

  private onKeydown(event: KeyboardEvent): boolean {
    if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === 'k') {
      event.preventDefault();
      this.openLinkForm();
      return true;
    }

    return false;
  }
}
