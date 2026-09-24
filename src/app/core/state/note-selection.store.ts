import { Injectable, computed, inject, signal } from '@angular/core';
import { debounced } from '@core/services/time/debounce';
import { Note } from '@core/model/note.model';
import { BoardStore } from './board.store';
import { NotesQueryStore } from './notes-query.store';

/** How long a note stays armed: two deliberate presses, not a note armed and walked away from. */
export const ARM_TTL_MS = 4000;

/** Both are positions in the visible list, and neither survives a note leaving the view. */
@Injectable({ providedIn: 'root' })
export class NoteSelectionStore {
  private readonly notes = inject(NotesQueryStore);
  private readonly board = inject(BoardStore);

  private readonly _focusedNoteId = signal<string | null>(null);
  private readonly _checkedIds = signal<ReadonlySet<string>>(new Set());
  private readonly _armedForDeletion = signal<string | null>(null);

  readonly focusedNoteId = this._focusedNoteId.asReadonly();
  readonly checkedIds = this._checkedIds.asReadonly();

  /**
   * The note one more Delete would trash, drawn in red. Here rather than on the card, which a
   * reload rebuilds: the keyboard's half of the two clicks the card's menu asks for.
   */
  readonly armedForDeletion = this._armedForDeletion.asReadonly();

  private readonly disarmLater = debounced<void>(() => this._armedForDeletion.set(null), ARM_TTL_MS);

  /**
   * The cards on screen, whichever view draws them: the board dims where the canvas narrows,
   * so a card filtered out of the date view can still be ticked on the board.
   */
  private readonly onScreen = computed<readonly Note[]>(() =>
    this.board.isShowing() ? this.board.visibleNotes() : this.notes.visibleNotes(),
  );

  /** Derived from what is visible: an id ticked then gone must not reach a bulk action. */
  readonly checkedNotes = computed<readonly Note[]>(() => {
    const checked = this._checkedIds();
    return this.onScreen().filter((note) => checked.has(note.id));
  });

  readonly checkedCount = computed(() => this.checkedNotes().length);
  readonly hasSelection = computed(() => this.checkedCount() > 0);

  readonly checkedNoteIds = computed<readonly string[]>(() => this.checkedNotes().map((note) => note.id));

  /** The note a card on screen is showing, in whichever view is drawing it. */
  noteOnScreen(id: string): Note | null {
    return this.onScreen().find((note) => note.id === id) ?? null;
  }

  /** `null` takes focus off the canvas — when a modal opens, for instance. */
  focusNote(id: string | null): void {
    // Pointing somewhere else is answering the question the armed note was asking.
    this.disarm();
    this._focusedNoteId.set(id);
  }

  armForDeletion(id: string): void {
    this._armedForDeletion.set(id);
    this.disarmLater();
  }

  disarm(): void {
    this.disarmLater.cancel();
    this._armedForDeletion.set(null);
  }

  focusedIndex(): number {
    const focused = this._focusedNoteId();
    return focused === null ? -1 : this.onScreen().findIndex((note) => note.id === focused);
  }

  focusedNote(): Note | null {
    const index = this.focusedIndex();
    return index < 0 ? null : (this.onScreen()[index] ?? null);
  }

  toggleChecked(id: string): void {
    this._checkedIds.update((checked) => {
      const next = new Set(checked);
      if (!next.delete(id)) {
        next.add(id);
      }
      return next;
    });
  }

  /** With no anchor, this ticks the named note alone. */
  checkRangeTo(id: string): void {
    const visible = this.onScreen();
    const anchor = this.focusedIndex();
    const target = visible.findIndex((note) => note.id === id);
    if (target < 0) return;

    const from = anchor < 0 ? target : Math.min(anchor, target);
    const to = anchor < 0 ? target : Math.max(anchor, target);

    this._checkedIds.update((checked) => {
      const next = new Set(checked);
      for (const note of visible.slice(from, to + 1)) {
        next.add(note.id);
      }
      return next;
    });
    this._focusedNoteId.set(id);
  }

  /** Ticks a set of notes at once, what a band swept over. It adds, so two sweeps build one selection. */
  checkMany(ids: readonly string[]): void {
    const onScreen = new Set(this.onScreen().map((note) => note.id));
    const wanted = ids.filter((id) => onScreen.has(id));
    if (wanted.length === 0) return;

    this._checkedIds.update((checked) => {
      const next = new Set(checked);
      for (const id of wanted) {
        next.add(id);
      }
      return next;
    });
  }

  /**
   * Ticks every note of one folder in one gesture: whatever is on screen in that folder — on
   * the board the zone's cards, dimmed ones included.
   */
  checkFolder(folderId: string): void {
    this.checkMany(
      this.onScreen()
        .filter((note) => note.folderId === folderId)
        .map((note) => note.id),
    );
  }

  clearSelection(): void {
    this._checkedIds.set(new Set());
  }
}
