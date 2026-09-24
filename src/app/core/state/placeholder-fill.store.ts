import { Injectable, computed, effect, inject, signal } from '@angular/core';
import { StatusNotifier } from '@core/services/notifications/status.service';
import { noteCopyText } from '../model/checklist.model';
import { Note } from '../model/note.model';
import { NoteCopyService } from '@core/services/clipboard/note-copy.service';
import { NotesQueryStore } from './notes-query.store';
import { NotesStore } from './notes.store';
import { PaletteStore } from './palette.store';

/** Composed at the moment of the click: anything computed ahead would be stale. */
export interface FillRequest {
  readonly content: string;
  readonly values: Record<string, string>;
}

/** The filling belongs to `notes::placeholder::fill`; what is here is fill, copy, keep. */
@Injectable({ providedIn: 'root' })
export class PlaceholderFillStore {
  private readonly notes = inject(NotesStore);
  private readonly canvas = inject(NotesQueryStore);
  private readonly palette = inject(PaletteStore);
  private readonly copier = inject(NoteCopyService);
  private readonly status = inject(StatusNotifier);

  private readonly _target = signal<Note | null>(null);
  private readonly _preview = signal<string | null>(null);

  readonly target = this._target.asReadonly();

  /** The filled body the editor preview shows; `null` before the first request. */
  readonly preview = this._preview.asReadonly();

  readonly placeholders = computed(() => this._target()?.placeholders ?? []);

  /** Two answers can come back out of order: only the current request may render. */
  private latestRequest: FillRequest | null = null;

  constructor() {
    // A preview belongs to the note that asked for it.
    effect(() => {
      this.notes.selectedNoteId();
      this.latestRequest = null;
      this._preview.set(null);
    });
  }

  openFor(noteId: string): void {
    this._target.set(this.canvas.visibleNotes().find((note) => note.id === noteId) ?? null);
  }

  /**
   * What copying a note means when nothing is pointing at a particular control: a snippet
   * with fields asks for them first, a todo list gives its Markdown — it has no `content`
   * at all — and anything else goes as it is.
   *
   * ⚠️ It says which note it took. The card's own button paints a tick on itself; the
   * keyboard has no such surface, and the ring may be on a card that is scrolled away.
   */
  async copyNote(note: Note): Promise<void> {
    if (note.placeholders.length > 0) {
      this.openFor(note.id);
      return;
    }

    if (!(await this.copier.copy(noteCopyText(note)))) return;

    this.status.notify(
      note.title
        ? { key: 'notes.copiedNote', params: { title: note.title } }
        : { key: 'notes.copiedUntitled' },
    );
  }

  cancel(): void {
    this._target.set(null);
  }

  async submit(values: Record<string, string>): Promise<void> {
    const note = this._target();
    if (!note) return;

    this._target.set(null);
    await this.copier.copy(await this.notes.fillPlaceholders(note.content, values));
    await this.notes.setPlaceholderValues(note.id, values);
  }

  /** "Copy as is", for a note that holds template code without being a snippet. */
  async copyRaw(): Promise<void> {
    const note = this._target();
    this._target.set(null);
    if (note) {
      await this.copier.copy(note.content);
    }
  }

  async refreshPreview(request: FillRequest): Promise<void> {
    this.latestRequest = request;
    const filled = await this.notes.fillPlaceholders(request.content, request.values);

    if (this.latestRequest === request) {
      this._preview.set(filled);
    }
  }

  /** The status banner and not the button's tick: the text exists only after a round trip. */
  async copyFilled(request: FillRequest): Promise<void> {
    const filled = await this.notes.fillPlaceholders(request.content, request.values);

    if (await this.copier.copy(filled)) {
      this.status.notify({ key: 'placeholders.copiedFilled' });
    }
  }

  async submitForPalette(values: Record<string, string>): Promise<void> {
    const note = this.palette.pendingFill();
    if (!note) return;

    await this.palette.copyAndDismiss(await this.notes.fillPlaceholders(note.content, values), note.title);
    await this.notes.setPlaceholderValues(note.id, values);
  }
}
