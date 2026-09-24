import { Injectable, inject } from '@angular/core';
import { ErrorNotifier } from '@core/services/errors/error-notifier.service';
import { FoldersRepository } from '@core/data/folders.repository';
import { NotesRepository } from '@core/data/notes.repository';
import { BoardScope } from '@core/model/board.model';
import { BoardStore } from './board.store';
import { NoteSelectionStore } from './note-selection.store';
import { NotesRevision } from './notes-revision';
import { UndoStore } from './undo.store';

/**
 * The writes that act on many notes at once — the ticked selection, or the whole board —
 * each leaving what it changed with `UndoStore`.
 */
@Injectable({ providedIn: 'root' })
export class NoteBatchStore {
  private readonly notes = inject(NotesRepository);
  private readonly folders = inject(FoldersRepository);
  private readonly board = inject(BoardStore);
  private readonly selection = inject(NoteSelectionStore);
  private readonly notifier = inject(ErrorNotifier);
  private readonly revision = inject(NotesRevision);
  private readonly undo = inject(UndoStore);

  async moveSelection(spaceId: string): Promise<void> {
    const previous = await this.runOnSelection((ids) => this.notes.moveMany(ids, spaceId));
    if (previous === null) return;

    this.undo.record({ kind: 'move', previous, count: previous.length });
  }

  /** `folderId` of `null` takes the selection out of whatever folder it was in. */
  async fileSelection(folderId: string | null): Promise<void> {
    const previous = await this.runOnSelection((ids) => this.folders.fileMany(ids, folderId));
    if (previous === null) return;

    this.undo.record({ kind: 'file', previous, count: previous.length });
  }

  async tagSelection(tag: string): Promise<void> {
    if (!tag.trim()) return;

    // No normalisation here: `notes::model::normalize_tags` is its only keeper.
    const added = await this.runOnSelection((ids) => this.notes.tagMany(ids, [tag]));
    if (added === null) return;

    this.undo.record({ kind: 'tag', added, count: added.length });
  }

  async deleteSelection(): Promise<void> {
    const ids = this.selection.checkedNoteIds();
    if (ids.length === 0) return;

    const count = await this.notifier.attempt('errors.bulkActionFailed', () => this.notes.deleteMany(ids));
    if (count === null) return;

    this.selection.clearSelection();
    this.undo.record({ kind: 'deletion', ids, count });
    this.revision.bump();
  }

  /**
   * Puts the board back in order, and offers the previous arrangement back. The count is what
   * actually **moved**: a board already in order opens no undo window.
   */
  async arrangeBoard(scope: BoardScope): Promise<void> {
    const done = await this.board.arrange(scope);
    if (!done) return;

    this.undo.record({ kind: 'arrange', layout: done.previous, count: done.moved });
  }

  /** `null` when there was nothing to act on, or when the batch failed. */
  private async runOnSelection<T>(action: (ids: readonly string[]) => Promise<T>): Promise<T | null> {
    const ids = this.selection.checkedNoteIds();
    if (ids.length === 0) return null;

    const done = await this.notifier.attempt('errors.bulkActionFailed', () => action(ids));
    if (done !== null) this.revision.bump();

    return done;
  }
}
