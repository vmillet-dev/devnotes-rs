import { Injectable, computed, inject, signal } from '@angular/core';
import { NotesRepository } from '@core/data/notes.repository';
import { Revision } from '@core/model/revision.model';
import { ErrorNotifier } from '@core/services/errors/error-notifier.service';
import { NotesRevision } from './notes-revision';
import { NotesStore } from './notes.store';

/**
 * The bodies kept beside the open note.
 *
 * ⚠️ Restoring one needs no confirmation, unlike every other thing in this application
 * that replaces content: the body it replaces is kept **first**, so a restore is as
 * undoable as the edit that made it necessary. Putting a guard in front of a reversible
 * gesture is how a safety net becomes a nuisance.
 */
@Injectable({ providedIn: 'root' })
export class NoteRevisionsStore {
  private readonly repository = inject(NotesRepository);
  private readonly notifier = inject(ErrorNotifier);
  private readonly revision = inject(NotesRevision);
  private readonly notes = inject(NotesStore);

  private readonly _noteId = signal<string | null>(null);
  private readonly _revisions = signal<readonly Revision[]>([]);
  private readonly _isOpen = signal(false);
  private readonly _isWorking = signal(false);
  private readonly _restored = signal(0);

  readonly revisions = this._revisions.asReadonly();
  readonly isOpen = this._isOpen.asReadonly();
  readonly isWorking = this._isWorking.asReadonly();

  /**
   * Bumped by every restore, and the editor's body draft keys on it.
   *
   * ⚠️ That draft is a `linkedSignal` on the note **id**, which does not change when a
   * body is put back underneath it — so the field kept showing the version that had just
   * been replaced, and closing the editor committed it straight back over the restored
   * one. The restore undid itself.
   */
  readonly restored = this._restored.asReadonly();

  /** ⚠️ Nothing to show is nothing to offer: a panel always there and always empty is a
   *  feature nobody uses, and the editor is already dense. */
  readonly hasHistory = computed(() => this._revisions().length > 0);

  /**
   * ⚠️ Called whenever the editor changes note, the draft included — a history left over
   * from the note before would offer to paste its body into this one.
   *
   * ⚠️ It reloads even for the note it is already on, and must: the editor is **destroyed**
   * when it closes, so nothing ever calls this with `null` on the way out. Skipping the
   * work for a matching id left the panel showing what the first open found — which for a
   * note edited since is an empty history that will not come back.
   */
  async openFor(noteId: string | null): Promise<void> {
    const moved = noteId !== this._noteId();
    this._noteId.set(noteId);

    if (moved) {
      this._revisions.set([]);
      this._isOpen.set(false);
    }

    if (noteId === null) return;

    await this.reload();
  }

  toggle(): void {
    this._isOpen.update((open) => !open);
  }

  async restore(revisionId: string): Promise<void> {
    const noteId = this._noteId();
    if (noteId === null || this._isWorking()) return;

    this._isWorking.set(true);
    try {
      // ⚠️ Adopted before the counter bumps: the editor re-seeds its body draft from the
      // open note, and a counter that moved first would re-seed it from the stale row.
      this.notes.adoptRestored(await this.repository.restoreRevision(noteId, revisionId));
      this._restored.update((count) => count + 1);
      // ⚠️ Bumped rather than reloaded by hand: the canvas and the board both read this,
      // and a body put back has to reach whichever one is on screen.
      this.revision.bump();
      await this.reload();
    } catch (error) {
      this.notifier.reportFailure('errors.revisionRestoreFailed', error);
    } finally {
      this._isWorking.set(false);
    }
  }

  private async reload(): Promise<void> {
    const noteId = this._noteId();
    if (noteId === null) return;

    try {
      this._revisions.set(await this.repository.listRevisions(noteId));
    } catch (error) {
      this.notifier.reportFailure('errors.revisionsListFailed', error);
    }
  }
}
