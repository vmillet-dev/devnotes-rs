import {
  ChangeDetectionStrategy,
  Component,
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
import { AttachmentsStore } from '@core/state/attachments.store';
import { FoldersStore } from '@core/state/folders.store';
import { NotesStore } from '@core/state/notes.store';
import { SpacesStore } from '@core/state/spaces.store';
import { NoteRevisionsStore } from '@core/state/note-revisions.store';
import { PlaceholderFillStore } from '@core/state/placeholder-fill.store';
import { HelpStore } from '@core/services/help/help.store';
import { ExternalLinksService } from '@core/services/links/external-links.service';
import { PreferencesService } from '@core/services/preferences/preferences.service';
import { ClockService } from '@core/services/time/clock.service';
import { endOfLocalDay, toDateInputValue } from '@core/utils/local-day.util';
import { relativeTimeRef } from '@core/utils/relative-time.util';
import { DialogComponent } from '@shared/layout/dialog/dialog.component';
import { CodeViewerComponent } from '@notes/ui/code-viewer/code-viewer.component';
import { AttachmentStripComponent } from './attachment-strip/attachment-strip.component';
import { ChecklistEditorComponent } from './checklist-editor/checklist-editor.component';
import { CopyButtonComponent } from '@notes/ui/copy-button/copy-button.component';
import { LifecycleBadgeComponent } from './lifecycle-badge/lifecycle-badge.component';
import { PlaceholderPanelComponent } from './placeholder-panel/placeholder-panel.component';
import { RevisionPanelComponent } from './revision-panel/revision-panel.component';
import { RichTextEditorComponent } from './rich-text-editor/rich-text-editor.component';
import { ChoiceMenuComponent, ChoiceOption } from '@notes/ui/choice-menu/choice-menu.component';
import { TagPillComponent } from '@notes/ui/tag-pill/tag-pill.component';

const TEXT_ENCODER = new TextEncoder();

const FULLSCREEN_STORAGE_KEY = 'devnotes.editorFullscreen';

const FIELDS_PANEL_STORAGE_KEY = 'devnotes.editorFieldsPanel';

const LANGUAGE_CHOICES: readonly ChoiceOption[] = Object.entries(LANGUAGE_LABELS).map(([id, name]) => ({
  id,
  name,
}));

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

  readonly note = input<Note | null>(null);

  /**
   * ⚠️ What the drafts are keyed on, rather than the note's id: committing the first field
   * of a new note materialises it, which changes its id — and drafts keyed on the id were
   * then replayed from a note whose body was not written yet, emptying it.
   */
  readonly session = input(0);

  protected readonly languageChoices = LANGUAGE_CHOICES;

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
  /** A Text note is written in the rich editor; every other language keeps the code field. */
  protected readonly isRichText = computed(() => {
    const note = this.note();
    return note?.kind === 'snippet' && note.language === 'txt';
  });
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
  protected readonly byteSize = computed(() => TEXT_ENCODER.encode(this.draftContent()).length);
  protected readonly modifiedRef = computed(() => {
    const note = this.note();
    return note ? relativeTimeRef(note.updatedAt, this.clock.now()) : null;
  });

  protected readonly expiryInputValue = computed(() => {
    const lifecycle = this.note()?.lifecycle;
    return lifecycle?.kind === 'expires' ? toDateInputValue(lifecycle.at) : '';
  });

  constructor() {
    // The help panels are drawn over the editor: one left up would hide the note just opened,
    // whatever opened it — a global shortcut, the palette.
    effect(() => {
      this.session();
      untracked(() => this.help.close());
    });

    // A snippet's history only: a checklist's items live in `note_items`, a second table
    // to snapshot, and a panel always empty for half the note kinds says nothing.
    effect(() => {
      const note = this.note();
      void this.revisions.openFor(note && note.kind === 'snippet' ? note.id : null);
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
   * Plain text pasted into an empty Text note. Code keeps its characters and its language, and
   * the note leaves for the code field; prose stays here, read as Markdown.
   */
  protected async onRichPaste(text: string): Promise<void> {
    const language = await this.store.detectLanguage(text);
    this.draftContent.set(text);
    if (language === 'txt') {
      this.commitContent();
    } else {
      this.requestPatch({ content: text, language });
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

  protected saveFieldValues(values: Record<string, string>): void {
    const note = this.note();
    if (note) {
      void this.store.setPlaceholderValues(note.id, values);
    }
  }

  protected commitContent(): void {
    this.requestPatch({ content: this.draftContent() });
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
   * the close button produce no `blur`, so the last line typed would be lost.
   */
  protected requestClose(): void {
    this.commitTitle();
    this.commitSource();
    this.commitContent();
    this.checklistEditor()?.commit();
    this.fieldsPanel()?.commit();
    this.store.closeOverlay();
  }
}
