import {
  ChangeDetectionStrategy,
  Component,
  DestroyRef,
  ElementRef,
  computed,
  effect,
  inject,
  input,
  linkedSignal,
  signal,
  untracked,
  viewChild,
} from '@angular/core';
import { TranslocoPipe } from '@jsverse/transloco';
import { FALLBACK_LANGUAGE, LANGUAGE_LABELS, isLanguageTag } from '@core/model/language.model';
import { checklistProgress } from '@core/model/checklist.model';
import { Note, NotePatch } from '@core/model/note.model';
import { PRIORITIES } from '@core/model/priority.model';
import { AttachmentsStore } from '@core/state/attachments.store';
import { FoldersStore } from '@core/state/folders.store';
import { NotesStore } from '@core/state/notes.store';
import { SpacesStore } from '@core/state/spaces.store';
import { NoteRevisionsStore } from '@core/state/note-revisions.store';
import { PlaceholderFillStore } from '@core/state/placeholder-fill.store';
import { FormatAnswer } from '@core/services/format/format.model';
import { FormatterService } from '@core/services/format/formatter.service';
import { PrettierSettingsStore } from '@core/services/format/prettier-settings.store';
import { HelpStore } from '@core/services/help/help.store';
import { ExternalLinksService } from '@core/services/links/external-links.service';
import { PreferencesService } from '@core/services/preferences/preferences.service';
import { SettingsStore } from '@core/services/settings/settings.store';
import { ClockService } from '@core/services/time/clock.service';
import { endOfLocalDay, toDateInputValue } from '@core/utils/local-day.util';
import { relativeTimeRef } from '@core/utils/relative-time.util';
import { DialogComponent } from '@shared/layout/dialog/dialog.component';
import { CodeViewerComponent } from '@notes/ui/code-viewer/code-viewer.component';
import { AttachmentStripComponent } from './attachment-strip/attachment-strip.component';
import { ChecklistEditorComponent } from './checklist-editor/checklist-editor.component';
import { FormatButtonComponent } from './format-button/format-button.component';
import { CopyButtonComponent } from '@notes/ui/copy-button/copy-button.component';
import { LifecycleBadgeComponent } from './lifecycle-badge/lifecycle-badge.component';
import { PlaceholderPanelComponent } from './placeholder-panel/placeholder-panel.component';
import { RevisionPanelComponent } from './revision-panel/revision-panel.component';
import { RichTextEditorComponent } from './rich-text-editor/rich-text-editor.component';
import { applyEdit, indent, indentUnit, outdent, rewrite } from './indentation';
import { countWords } from './word-count';
import { ChoiceMenuComponent, ChoiceOption } from '@shared/controls/choice-menu/choice-menu.component';
import { TagPillComponent } from '@notes/ui/tag-pill/tag-pill.component';

const TEXT_ENCODER = new TextEncoder();

const FULLSCREEN_STORAGE_KEY = 'devnotes.editorFullscreen';

const FIELDS_PANEL_STORAGE_KEY = 'devnotes.editorFieldsPanel';

/** How long a format's notice stays; the marked lines stay until the next keystroke. */
const FORMAT_NOTICE_MS = 6000;

const NO_LINES: ReadonlySet<number> = new Set();

type FormatNotice =
  Exclude<FormatAnswer, { kind: 'formatted' }> | { readonly kind: 'formatted'; readonly changed: number };

/** By the printed key, and by position where Alt prints another (macOS). */
function isFormatShortcut(event: KeyboardEvent): boolean {
  if (!event.shiftKey || !event.altKey || event.ctrlKey || event.metaKey) return false;
  return event.key.toLowerCase() === 'f' || (!/^[a-z]$/i.test(event.key) && event.code === 'KeyF');
}

/** Prettier's one-based line and column, as an offset into the text. */
function offsetOf(text: string, line: number, column: number): number {
  const lines = text.split('\n');
  const before = lines.slice(0, line - 1).reduce((sum, each) => sum + each.length + 1, 0);
  return Math.min(text.length, before + column - 1);
}

const LANGUAGE_CHOICES: readonly ChoiceOption[] = Object.entries(LANGUAGE_LABELS).map(([id, name]) => ({
  id,
  name,
}));

/** "None" is the menu's own entry, and what the trigger says while no level is set. */
const PRIORITY_CHOICES: readonly ChoiceOption[] = PRIORITIES.filter((level) => level !== 'none').map(
  (level) => ({ id: level, name: `notes.priority.${level}`, nameIsKey: true }),
);

/**
 * Talks to stores, like every other editor: `NotesStore` persists the note, and the
 * attachments, the history and the `{{field}}` filling each have a store of their own.
 * Title and body are local drafts — one round trip per keystroke otherwise — committed on
 * blur and on every closing path, none of which produces a `blur`.
 */
@Component({
  selector: 'app-note-editor-overlay',
  imports: [
    DialogComponent,
    AttachmentStripComponent,
    ChecklistEditorComponent,
    CopyButtonComponent,
    FormatButtonComponent,
    TagPillComponent,
    LifecycleBadgeComponent,
    PlaceholderPanelComponent,
    RevisionPanelComponent,
    ChoiceMenuComponent,
    CodeViewerComponent,
    RichTextEditorComponent,
    TranslocoPipe,
  ],
  templateUrl: './note-editor-overlay.component.html',
  styleUrl: './note-editor-overlay.component.scss',
  host: { '(keydown)': 'onEditorKeydown($event)' },
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class NoteEditorOverlayComponent {
  private readonly clock = inject(ClockService);
  private readonly preferences = inject(PreferencesService);
  private readonly store = inject(NotesStore);
  protected readonly attachments = inject(AttachmentsStore);
  protected readonly revisions = inject(NoteRevisionsStore);
  private readonly spaces = inject(SpacesStore);
  private readonly folders = inject(FoldersStore);
  protected readonly fill = inject(PlaceholderFillStore);
  private readonly help = inject(HelpStore);
  private readonly links = inject(ExternalLinksService);
  private readonly settings = inject(SettingsStore);
  private readonly formatter = inject(FormatterService);
  protected readonly prettierSettings = inject(PrettierSettingsStore);

  readonly note = input<Note | null>(null);

  /**
   * ⚠️ What the drafts are keyed on, rather than the note's id: committing the first field
   * of a new note materialises it, which changes its id — and drafts keyed on the id were
   * then replayed from a note whose body was not written yet, emptying it.
   */
  readonly session = input(0);

  protected readonly languageChoices = LANGUAGE_CHOICES;
  protected readonly priorityChoices = PRIORITY_CHOICES;

  protected readonly spaceOptions = computed<readonly ChoiceOption[]>(() =>
    this.spaces.spaces().map((space) => ({ id: space.id, name: space.name })),
  );

  protected readonly folderOptions = computed<readonly ChoiceOption[]>(() => {
    const spaceId = this.note()?.spaceId;
    return this.folders
      .allFolders()
      .filter((folder) => folder.spaceId === spaceId)
      .map((folder) => ({ id: folder.id, name: folder.name, colour: folder.colour }));
  });

  protected readonly draftTitle = linkedSignal({
    source: this.session,
    computation: () => untracked(() => this.note()?.title ?? ''),
  });

  /**
   * Keyed on the restore counter as well as the id: putting a body back does not change the
   * id, and the commit on close would write the replaced text back.
   */
  protected readonly draftContent = linkedSignal({
    source: () => [this.session(), this.revisions.restored()] as const,
    computation: () => untracked(() => this.note()?.content ?? ''),
  });

  protected readonly draftSource = linkedSignal({
    source: this.session,
    computation: () => untracked(() => this.note()?.source ?? ''),
  });

  /** Two steps rather than a native `confirm()`, which blocks the whole WebView. */
  protected readonly confirmingDelete = linkedSignal({ source: this.session, computation: () => false });

  protected readonly tagInputValue = signal('');

  /** A display preference, not note state: it survives moving to the next note. */
  protected readonly fullscreen = signal(this.preferences.read(FULLSCREEN_STORAGE_KEY) === 'true');

  /** Open by default: folded away, it hides the feature from anyone who does not know it. */
  protected readonly fieldsPanelOpen = signal(this.preferences.read(FIELDS_PANEL_STORAGE_KEY) !== 'false');

  protected readonly previewingFilled = linkedSignal({
    source: this.session,
    computation: () => false,
  });

  private readonly bodyEditor = viewChild<ElementRef<HTMLTextAreaElement>>('bodyEditor');
  /** By its template name: a class query would pull TipTap out of its deferred chunk. */
  private readonly richEditor = viewChild<RichTextEditorComponent>('richEditor');
  private readonly checklistEditor = viewChild(ChecklistEditorComponent);
  private readonly fieldsPanel = viewChild(PlaceholderPanelComponent);

  protected readonly isChecklist = computed(() => this.note()?.kind === 'checklist');
  /** A Note is written in the rich editor; a snippet keeps the code field. */
  protected readonly isRichText = computed(() => this.note()?.kind === 'note');
  /** Only a snippet's body is code: a format picker anywhere else would do nothing. */
  protected readonly hasLanguage = computed(() => this.note()?.kind === 'snippet');
  protected readonly checklistStats = computed(() => checklistProgress(this.note()?.items ?? []));

  /** The draft, so copying before leaving the field yields what is on screen. */
  protected readonly copyText = computed(() =>
    this.isChecklist() ? (this.note()?.copyText ?? '') : this.draftContent(),
  );

  protected readonly placeholders = computed(() => this.note()?.placeholders ?? []);
  protected readonly hasPlaceholders = computed(() => this.placeholders().length > 0);

  protected readonly showingPreview = computed(() => this.previewingFilled() && this.fill.preview() !== null);

  protected readonly languageLabel = computed(
    () => LANGUAGE_LABELS[this.note()?.language ?? FALLBACK_LANGUAGE],
  );
  protected readonly lineCount = computed(() => (this.note() ? this.draftContent().split('\n').length : 0));
  protected readonly wordCount = computed(() => countWords(this.draftContent()));
  protected readonly codeIndent = computed(() => this.settings.codeIndent());
  protected readonly byteSize = computed(() => TEXT_ENCODER.encode(this.draftContent()).length);
  protected readonly lineEnding = computed(() => (this.draftContent().includes('\r\n') ? 'CRLF' : 'LF'));

  protected readonly canFormat = computed(
    () => this.hasLanguage() && this.formatter.canFormat(this.note()?.language ?? FALLBACK_LANGUAGE),
  );
  protected readonly formatting = signal(false);
  protected readonly formatNotice = linkedSignal<number, FormatNotice | null>({
    source: this.session,
    computation: () => null,
  });
  protected readonly markedLines = linkedSignal<number, ReadonlySet<number>>({
    source: this.session,
    computation: () => NO_LINES,
  });
  private noticeTimer: ReturnType<typeof setTimeout> | undefined;
  /** A close waiting on Prettier: a second Escape must not commit and close twice. */
  private closing = false;
  protected readonly modifiedRef = computed(() => {
    const note = this.note();
    return note ? relativeTimeRef(note.updatedAt, this.clock.now()) : null;
  });

  protected readonly expiryInputValue = computed(() => {
    const lifecycle = this.note()?.lifecycle;
    return lifecycle?.kind === 'expires' ? toDateInputValue(lifecycle.at) : '';
  });

  constructor() {
    inject(DestroyRef).onDestroy(() => clearTimeout(this.noticeTimer));

    // The help panels are drawn over the editor: one left up would hide the note just opened,
    // whatever opened it — a global shortcut, the palette.
    effect(() => {
      this.session();
      untracked(() => this.help.close());
    });

    // Never a checklist's: its items live in `note_items`, a second table to snapshot, and a
    // panel always empty for one kind in three says nothing.
    effect(() => {
      const note = this.note();
      void this.revisions.openFor(note && note.kind !== 'checklist' ? note.id : null);
    });
  }

  /** The note's **own** space, not the active one: a note open from "all spaces"
   *  belongs to a space of its own, and another one's folders would file it nowhere. */
  protected onSpaceChosen(spaceId: string | null): void {
    // A note always has a space: the menu carries no "none" entry, so this cannot be null.
    if (spaceId !== null) this.requestPatch({ spaceId });
  }

  /** Folding closes the preview: a read-only body without the button that caused it is a trap. */
  protected toggleFieldsPanel(): void {
    const next = !this.fieldsPanelOpen();
    this.fieldsPanelOpen.set(next);
    this.preferences.write(FIELDS_PANEL_STORAGE_KEY, String(next));

    if (!next) {
      this.previewingFilled.set(false);
    }
  }

  protected togglePreview(): void {
    const next = !this.previewingFilled();
    this.previewingFilled.set(next);

    if (next) {
      this.requestFillPreview();
    }
  }

  /** The preview follows the typing; the panel is what commits, on field exit. */
  protected onPlaceholderValuesChanged(): void {
    if (this.previewingFilled()) {
      this.requestFillPreview();
    }
  }

  private requestFillPreview(): void {
    void this.fill.refreshPreview({
      content: this.draftContent(),
      values: this.placeholderValues(),
    });
  }

  protected requestFilledCopy(): void {
    void this.fill.copyFilled({
      content: this.draftContent(),
      values: this.placeholderValues(),
    });
  }

  private placeholderValues(): Record<string, string> {
    return this.fieldsPanel()?.values() ?? {};
  }

  protected toggleFullscreen(): void {
    const next = !this.fullscreen();
    this.fullscreen.set(next);
    this.preferences.write(FULLSCREEN_STORAGE_KEY, String(next));
  }

  protected onBodyInput(value: string): void {
    this.draftContent.set(value);
    this.clearFormat();
  }

  protected onEditorKeydown(event: KeyboardEvent): void {
    if (!isFormatShortcut(event)) return;
    event.preventDefault();
    void this.formatBody();
  }

  /**
   * Through the field's own editing, like indentation: Ctrl+Z undoes a format like a
   * keystroke. The worker answers asynchronously, so the text is compared before applying.
   */
  protected async formatBody(): Promise<void> {
    const field = this.bodyEditor()?.nativeElement;
    const note = this.note();
    if (!field || !note || !this.canFormat() || this.formatting()) return;

    const before = field.value;
    const indentation = indentUnit(this.settings.codeIndent(), note.language);
    this.formatting.set(true);
    const answer = await this.formatter.format(before, note.language, field.selectionStart, indentation);
    this.formatting.set(false);
    if (field.value !== before || this.bodyEditor()?.nativeElement !== field) return;

    field.focus();
    if (answer.kind === 'formatted') {
      applyEdit(field, rewrite(before, answer.text, answer.cursor));
      this.markedLines.set(new Set(answer.changedLines));
      this.showFormatNotice({ kind: 'formatted', changed: answer.changedLines.length });
      return;
    }
    if (answer.kind === 'syntax') {
      const at = offsetOf(before, answer.line, answer.column);
      field.setSelectionRange(at, at);
    }
    this.showFormatNotice(answer);
  }

  /** The field's own undo, so the notice's button and Ctrl+Z are one and the same step. */
  protected undoFormat(): void {
    const field = this.bodyEditor()?.nativeElement;
    if (!field) return;
    field.focus();
    document.execCommand('undo');
    this.clearFormat();
  }

  private showFormatNotice(notice: FormatNotice): void {
    clearTimeout(this.noticeTimer);
    this.formatNotice.set(notice);
    this.noticeTimer = setTimeout(() => this.formatNotice.set(null), FORMAT_NOTICE_MS);
  }

  private clearFormat(): void {
    clearTimeout(this.noticeTimer);
    this.formatNotice.set(null);
    this.markedLines.set(NO_LINES);
  }

  /** Tab stays in the code: Escape is how the keyboard leaves the field. */
  protected onBodyKeydown(event: KeyboardEvent, field: HTMLTextAreaElement): void {
    if (event.key !== 'Tab' || event.ctrlKey || event.altKey || event.metaKey || event.isComposing) return;

    event.preventDefault();
    const unit = indentUnit(this.settings.codeIndent(), this.note()?.language ?? FALLBACK_LANGUAGE);
    const { value, selectionStart, selectionEnd } = field;
    const edit = event.shiftKey
      ? outdent(value, selectionStart, selectionEnd, unit)
      : indent(value, selectionStart, selectionEnd, unit);
    if (edit) applyEdit(field, edit);
  }

  /**
   * A pasted image becomes an attachment: the body is a `<textarea>`. Only the content
   * type is read here — the bytes are re-read natively and never cross the bridge.
   */
  protected onPaste(event: ClipboardEvent): void {
    const data = event.clipboardData;
    if (!data || data.types.includes('text/plain')) return;

    const hasImage =
      data.types.some((type) => type.startsWith('image/')) ||
      [...data.files].some((file) => file.type.startsWith('image/'));
    if (!hasImage) return;

    event.preventDefault();
    void this.attachments.addPastedImage();
  }

  /**
   * Plain text pasted into an empty Note. Code keeps its characters and gets its language: the
   * note becomes a snippet, in the code field. Prose stays here, read as Markdown.
   */
  protected async onRichPaste(text: string): Promise<void> {
    const language = await this.store.detectLanguage(text);
    this.draftContent.set(text);
    if (language === 'txt') {
      this.commitContent();
    } else {
      this.requestPatch({ content: text, language, kind: 'snippet' });
    }
  }

  protected onImagePasted(): void {
    void this.attachments.addPastedImage();
  }

  protected openLink(url: string): void {
    void this.links.open(url);
  }

  /** Every field goes through here: whether a value moved is `NotesStore`'s call. */
  protected requestPatch(patch: NotePatch): void {
    const note = this.note();
    if (note) {
      void this.store.applyPatch(note.id, patch);
    }
  }

  /** Not a patch: filing goes through `file_notes`, which answers what it changed. */
  protected fileInto(folderId: string | null): void {
    const note = this.note();
    if (note) {
      void this.store.fileNote(note.id, folderId);
    }
  }

  /** Not a patch either: `set_priority` leaves `updatedAt` alone. */
  protected choosePriority(level: string | null): void {
    const note = this.note();
    if (note) {
      void this.store.setPriority(note.id, PRIORITIES.find((each) => each === level) ?? 'none');
    }
  }

  protected saveFieldValues(values: Record<string, string>): void {
    const note = this.note();
    if (note) {
      void this.store.setPlaceholderValues(note.id, values);
    }
  }

  protected commitContent(): void {
    this.requestPatch({ content: this.draftContent() });
  }

  /**
   * Formatted first when the library asks for it — without the field's undo: the field has
   * lost the focus its editing commands need. Dropped if the editor closed meanwhile, the
   * close having committed on its own, or if the body was typed into again.
   */
  protected async commitBodyOnBlur(): Promise<void> {
    const text = this.draftContent();
    const formatting = this.formatForSave(text);
    if (formatting === null) {
      this.commitContent();
      return;
    }

    const session = this.session();
    const formatted = await formatting;
    if (this.session() !== session || this.draftContent() !== text) return;
    if (formatted !== null) this.draftContent.set(formatted);
    this.commitContent();
  }

  /** `null` at once when there is nothing to format; a text Prettier refuses is saved as it is. */
  private formatForSave(text: string): Promise<string | null> | null {
    const note = this.note();
    if (
      !note ||
      !this.canFormat() ||
      !this.prettierSettings.settings().formatOnSave ||
      text === note.content
    ) {
      return null;
    }

    const indentation = indentUnit(this.settings.codeIndent(), note.language);
    return this.formatter
      .format(text, note.language, 0, indentation)
      .then((answer) => (answer.kind === 'formatted' ? answer.text : null));
  }

  protected commitTitle(): void {
    this.requestPatch({ title: this.draftTitle() });
  }

  protected commitSource(): void {
    this.requestPatch({ source: this.draftSource() });
  }

  protected togglePin(): void {
    const note = this.note();
    if (note) {
      this.requestPatch({ pinned: !note.pinned });
    }
  }

  protected removeTag(tag: string): void {
    const note = this.note();
    if (note) {
      this.requestPatch({ tags: note.tags.filter((existing) => existing !== tag) });
    }
  }

  /** An unreadable date is ignored rather than sent on as an `Invalid Date`. */
  protected onExpiryChange(value: string): void {
    if (!value) {
      this.requestPatch({ lifecycle: { kind: 'permanent' } });
      return;
    }

    const at = endOfLocalDay(value);
    if (at) {
      this.requestPatch({ lifecycle: { kind: 'expires', at } });
    }
  }

  protected onLanguageChange(value: string): void {
    // The guard covers the option table and the type drifting apart.
    if (isLanguageTag(value)) {
      this.requestPatch({ language: value });
    }
  }

  protected submitTag(event: Event): void {
    event.preventDefault();
    const value = this.tagInputValue();
    this.tagInputValue.set('');

    const note = this.note();
    if (note && value.trim()) {
      this.requestPatch({ tags: [...note.tags, value] });
    }
  }

  protected onDeleteClick(): void {
    const note = this.note();
    if (note && this.confirmingDelete()) {
      void this.store.deleteNote(note.id);
      return;
    }
    this.confirmingDelete.set(true);
  }

  /**
   * Escape leaves the body first, then closes the modal: otherwise a keystroke meant
   * for the field makes the whole editor disappear. The `blur` commits on the way.
   */
  protected onDismiss(): void {
    const editor = this.bodyEditor()?.nativeElement;
    if (editor && document.activeElement === editor) {
      editor.blur();
      return;
    }
    const rich = this.richEditor();
    if (rich?.hasFocus()) {
      rich.blur();
      return;
    }
    this.requestClose();
  }

  /**
   * ⚠️ The only closing path, and it commits the drafts first: Escape, the backdrop and
   * the close button produce no `blur`, so the last line typed would be lost. Formatting on
   * save holds the close until Prettier answers; without it, nothing waits.
   */
  protected requestClose(): void {
    const formatting = this.formatForSave(this.draftContent());
    if (formatting === null) {
      this.close();
      return;
    }
    if (this.closing) return;

    this.closing = true;
    void formatting.then((formatted) => {
      if (formatted !== null) this.draftContent.set(formatted);
      this.closing = false;
      this.close();
    });
  }

  private close(): void {
    this.commitTitle();
    this.commitSource();
    this.commitContent();
    this.checklistEditor()?.commit();
    this.fieldsPanel()?.commit();
    this.store.closeOverlay();
  }
}
