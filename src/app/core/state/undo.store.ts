import { Injectable, computed, inject, signal } from '@angular/core';
import { ErrorNotifier } from '@core/services/errors/error-notifier.service';
import { debounced } from '@core/services/time/debounce';
import { FoldersRepository } from '../data/folders.repository';
import { NotesRepository } from '../data/notes.repository';
import { BoardLayout } from '../model/board.model';
import { NoteFiling } from '../model/folder.model';
import { NotePlacement, NoteTag } from '../model/note.model';
import { BoardStore } from './board.store';
import { NotesRevision } from './notes-revision';

/** The note is not lost after that: it stays in the trash for 30 days. */
export const UNDO_WINDOW_MS = 8000;

/**
 * ⚠️ Each variant carries what the back end answered, never what the front end guessed.
 * Rebuilding the pairs from the selection would undo a tag the note already carried.
 */
export type Reversible =
  | { readonly kind: 'deletion'; readonly ids: readonly string[]; readonly count: number }
  | { readonly kind: 'move'; readonly previous: readonly NotePlacement[]; readonly count: number }
  | { readonly kind: 'tag'; readonly added: readonly NoteTag[]; readonly count: number }
  | { readonly kind: 'file'; readonly previous: readonly NoteFiling[]; readonly count: number }
  | { readonly kind: 'arrange'; readonly layout: BoardLayout; readonly count: number };

/**
 * The last write that can be put back, and the bar offering it.
 *
 * ⚠️ Two things: the banner is what the timer hides, the record is what `Ctrl+Z` reads.
 * Hiding a suggestion is not withdrawing it; only `dismiss()` gives up for good.
 */
@Injectable({ providedIn: 'root' })
export class UndoStore {
  private readonly notes = inject(NotesRepository);
  private readonly folders = inject(FoldersRepository);
  private readonly board = inject(BoardStore);
  private readonly notifier = inject(ErrorNotifier);
  private readonly revision = inject(NotesRevision);

  private readonly _last = signal<Reversible | null>(null);
  private readonly _bannerShown = signal(false);

  private readonly hideBanner = debounced<void>(() => this._bannerShown.set(false), UNDO_WINDOW_MS);

  readonly last = this._last.asReadonly();
  readonly banner = computed<Reversible | null>(() => (this._bannerShown() ? this._last() : null));

  /** Nothing moved is nothing to offer: a bar saying "0 notes" is noise, not an undo. */
  record(action: Reversible): void {
    if (action.count === 0) return;

    this._last.set(action);
    this._bannerShown.set(true);
    this.hideBanner();
  }

  async revert(): Promise<void> {
    const action = this._last();
    if (!action) return;

    this.dismiss();
    const reverted = await this.notifier.attempt('errors.undoFailed', () => this.reverse(action));
    if (reverted !== null) this.revision.bump();
  }

  dismiss(): void {
    this.hideBanner.cancel();
    this._bannerShown.set(false);
    this._last.set(null);
  }

  /** Exhaustive by construction: a new kind of undo stops this compiling. */
  private reverse(action: Reversible): Promise<number> {
    switch (action.kind) {
      case 'deletion':
        return this.notes.restore(action.ids);
      case 'move':
        return this.notes.moveBack(action.previous);
      case 'tag':
        return this.notes.untagMany(action.added);
      case 'file':
        return this.folders.fileBack(action.previous);
      case 'arrange':
        return this.board.restoreLayout(action.layout);
    }
  }
}
