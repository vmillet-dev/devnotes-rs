import { DestroyRef, Injectable, computed, inject, signal } from '@angular/core';
import { Note } from '../model/note.model';
import { NotesQueryStore } from './notes-query.store';

/**
 * How long a note stays armed. ⚠️ Long enough to press the key twice on purpose, short
 * enough that a note armed and walked away from is not still armed on the way back.
 */
export const ARM_TTL_MS = 4000;

/** Both are positions in the visible list, and neither survives a note leaving the view. */
@Injectable({ providedIn: 'root' })
export class NoteSelectionStore {
  private readonly notes = inject(NotesQueryStore);

  private readonly _focusedNoteId = signal<string | null>(null);
  private readonly _checkedIds = signal<ReadonlySet<string>>(new Set());
  private readonly _armedForDeletion = signal<string | null>(null);

  readonly focusedNoteId = this._focusedNoteId.asReadonly();
  readonly checkedIds = this._checkedIds.asReadonly();

  /**
   * The note one more Delete would trash, which the card draws in red.
   *
   * ⚠️ Here and not on the card: the ring can move and a reload can rebuild the card, and
   * neither is a reason to forget what the first press said. It is the keyboard's half of
   * the two clicks the card's own menu asks for.
   */
  readonly armedForDeletion = this._armedForDeletion.asReadonly();

  private armTimeout: ReturnType<typeof setTimeout> | null = null;

  constructor() {
    inject(DestroyRef).onDestroy(() => this.cancelArmTimeout());
  }

  /** Derived from what is visible: an id ticked then gone must not reach a bulk action. */
  readonly checkedNotes = computed<readonly Note[]>(() => {
    const checked = this._checkedIds();
    return this.notes.visibleNotes().filter((note) => checked.has(note.id));
  });

  readonly checkedCount = computed(() => this.checkedNotes().length);
  readonly hasSelection = computed(() => this.checkedCount() > 0);

  readonly checkedNoteIds = computed<readonly string[]>(() => this.checkedNotes().map((note) => note.id));

  /** `null` takes focus off the canvas — when a modal opens, for instance. */
  focusNote(id: string | null): void {
    // Pointing somewhere else is answering the question the armed note was asking.
    this.disarm();
    this._focusedNoteId.set(id);
  }

  armForDeletion(id: string): void {
    this.cancelArmTimeout();
    this._armedForDeletion.set(id);
    this.armTimeout = setTimeout(() => {
      this.armTimeout = null;
      this._armedForDeletion.set(null);
    }, ARM_TTL_MS);
  }

  disarm(): void {
    this.cancelArmTimeout();
    this._armedForDeletion.set(null);
  }

  private cancelArmTimeout(): void {
    if (this.armTimeout !== null) {
      clearTimeout(this.armTimeout);
      this.armTimeout = null;
    }
  }

  focusedIndex(): number {
    const focused = this._focusedNoteId();
    return focused === null ? -1 : this.notes.visibleNotes().findIndex((note) => note.id === focused);
  }

  focusedNote(): Note | null {
    const index = this.focusedIndex();
    return index < 0 ? null : (this.notes.visibleNotes()[index] ?? null);
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
    const visible = this.notes.visibleNotes();
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

  clearSelection(): void {
    this._checkedIds.set(new Set());
  }
}
