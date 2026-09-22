import {
  ChangeDetectionStrategy,
  Component,
  ElementRef,
  computed,
  inject,
  input,
  linkedSignal,
  output,
  signal,
  untracked,
  viewChild,
} from '@angular/core';
import { TranslocoPipe } from '@jsverse/transloco';
import { FALLBACK_LANGUAGE, LANGUAGE_LABELS, LanguageTag, isLanguageTag } from '@core/model/language.model';
import { checklistProgress } from '@core/model/checklist.model';
import { Note, NotePatch } from '@core/model/note.model';
import { AttachmentsStore } from '@core/state/attachments.store';
import { FoldersStore } from '@core/state/folders.store';
import { SpacesStore } from '@core/state/spaces.store';
import { PlaceholderFillStore } from '@core/state/placeholder-fill.store';
import { PreferencesService } from '@core/services/preferences/preferences.service';
import { ClockService } from '@core/services/time/clock.service';
import { relativeTimeRef } from '@core/utils/relative-time.util';
import { DialogComponent } from '@shared/layout/dialog/dialog.component';
import { CodeViewerComponent } from '@notes/ui/code-viewer/code-viewer.component';
import { AttachmentStripComponent } from './attachment-strip/attachment-strip.component';
import { ChecklistEditorComponent } from './checklist-editor/checklist-editor.component';
import { CopyButtonComponent } from '@notes/ui/copy-button/copy-button.component';
import { LifecycleBadgeComponent } from './lifecycle-badge/lifecycle-badge.component';
import { PlaceholderPanelComponent } from './placeholder-panel/placeholder-panel.component';
import { ChoiceMenuComponent, ChoiceOption } from '@notes/ui/choice-menu/choice-menu.component';
import { TagPillComponent } from '@notes/ui/tag-pill/tag-pill.component';

const TEXT_ENCODER = new TextEncoder();

const FULLSCREEN_STORAGE_KEY = 'devnotes.editorFullscreen';

const FIELDS_PANEL_STORAGE_KEY = 'devnotes.editorFieldsPanel';

const LANGUAGE_OPTIONS = Object.entries(LANGUAGE_LABELS).map(([value, label]) => ({
  value: value as LanguageTag,
  label,
}));

/**
 * ⚠️ End of the local day, not midnight: a note dated today would otherwise be
 * expired the moment it is typed. Built explicitly because `new Date(value)` reads
 * as UTC, and west of Greenwich the deadline would slip back a day.
 */
function endOfLocalDay(value: string): Date | null {
  const [year, month, day] = value.split('-').map(Number);
  if (!year || !month || !day) return null;

  return new Date(year, month - 1, day, 23, 59, 59, 999);
}

function toDateInputValue(date: Date): string {
  const pad = (value: number): string => String(value).padStart(2, '0');
  return `${date.getFullYear()}-${pad(date.getMonth() + 1)}-${pad(date.getDate())}`;
}

/**
 * Mutates nothing: it emits, `NotesStore` persists. Title and body are local drafts
 * — one round trip per keystroke otherwise — committed on blur and on every closing
 * path, none of which produces a `blur`.
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
    ChoiceMenuComponent,
    CodeViewerComponent,
    TranslocoPipe,
  ],
  templateUrl: './note-editor-overlay.component.html',
  styleUrl: './note-editor-overlay.component.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class NoteEditorOverlayComponent {
  private readonly clock = inject(ClockService);
  private readonly preferences = inject(PreferencesService);

  /** Attachments and `{{field}}` filling have a write cycle of their own. */
  protected readonly attachments = inject(AttachmentsStore);
  private readonly spaces = inject(SpacesStore);
  private readonly folders = inject(FoldersStore);
  protected readonly fill = inject(PlaceholderFillStore);

  readonly note = input<Note | null>(null);

  readonly session = input(0);

  readonly closed = output<void>();
  /** One output for every field: whether a value moved is `NotesStore`'s call. */
  readonly patchRequested = output<NotePatch>();
  /** ⚠️ Not a patch: filing goes through `file_notes`, which answers what it changed. */
  readonly fileRequested = output<string | null>();
  readonly deleteRequested = output<void>();
  readonly placeholderValuesChanged = output<Record<string, string>>();

  protected readonly languageOptions = LANGUAGE_OPTIONS;

  protected readonly languageChoices: readonly ChoiceOption[] = LANGUAGE_OPTIONS.map((option) => ({
    id: option.value,
    name: option.label,
  }));

  protected readonly spaceOptions = computed<readonly ChoiceOption[]>(() =>
    this.spaces.spaces().map((space) => ({ id: space.id, name: space.name })),
  );

  /** ⚠️ The note's **own** space, not the active one: a note open from "all spaces"
   *  belongs to a space of its own, and another one's folders would file it nowhere. */
  protected onSpaceChosen(spaceId: string | null): void {
    // A note always has a space: the menu carries no "none" entry, so this cannot be null.
    if (spaceId !== null) this.patchRequested.emit({ spaceId });
  }

  protected readonly folderOptions = computed<readonly ChoiceOption[]>(() => {
    const spaceId = this.note()?.spaceId;
    return this.folders
      .allFolders()
      .filter((folder) => folder.spaceId === spaceId)
      .map((folder) => ({ id: folder.id, name: folder.name, colour: folder.colour }));
  });

  /**
   * ⚠️ The session, not the note's id. Committing the first field of a new note
   * materialises it, which changes its id — and drafts keyed on the id are then
   * replayed from a note whose content is not written yet, emptying the body.
   */
  private readonly noteId = computed(() => this.session());

  protected readonly draftTitle = linkedSignal({
    source: this.noteId,
    computation: () => untracked(() => this.note()?.title ?? ''),
  });

  protected readonly draftContent = linkedSignal({
    source: this.noteId,
    computation: () => untracked(() => this.note()?.content ?? ''),
  });

  protected readonly draftSource = linkedSignal({
    source: this.noteId,
    computation: () => untracked(() => this.note()?.source ?? ''),
  });

  /** Two steps rather than a native `confirm()`, which blocks the whole WebView. */
  protected readonly confirmingDelete = linkedSignal({ source: this.noteId, computation: () => false });

  protected readonly tagInputValue = signal('');

  /** A display preference, not note state: it survives moving to the next note. */
  protected readonly fullscreen = signal(this.preferences.read(FULLSCREEN_STORAGE_KEY) === 'true');

  /** Open by default: folded away, it hides the feature from anyone who does not know it. */
  protected readonly fieldsPanelOpen = signal(this.preferences.read(FIELDS_PANEL_STORAGE_KEY) !== 'false');

  protected readonly previewingFilled = linkedSignal({
    source: this.noteId,
    computation: () => false,
  });

  private readonly bodyEditor = viewChild<ElementRef<HTMLTextAreaElement>>('bodyEditor');
  private readonly checklistEditor = viewChild(ChecklistEditorComponent);
  private readonly fieldsPanel = viewChild(PlaceholderPanelComponent);

  protected readonly isChecklist = computed(() => this.note()?.kind === 'checklist');
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

  /**
   * A paste commits at once, typing stays deferred to blur: the paste is what gives an
   * empty note its language, and waiting would leave the badge reading TXT.
   */
  protected onBodyInput(event: Event, value: string): void {
    this.draftContent.set(value);

    if ((event as InputEvent).inputType === 'insertFromPaste') {
      this.commitContent();
    }
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

  protected requestPatch(patch: NotePatch): void {
    if (this.note()) {
      this.patchRequested.emit(patch);
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
    if (this.confirmingDelete()) {
      this.deleteRequested.emit();
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
    this.closed.emit();
  }
}
