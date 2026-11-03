import { Injectable, computed, inject, signal } from '@angular/core';
import { NotesRepository } from '@core/data/notes.repository';
import { DiffLine, Revision } from '@core/model/revision.model';
import { ErrorNotifier } from '@core/services/errors/error-notifier.service';
import { NotesRevision } from './notes-revision';
import { NotesStore } from './notes.store';

/** A kept body opened for a look, against the text going back to it would replace. */
export interface RevisionPreview {
  readonly revision: Revision;
  /** How many kept bodies are newer than this one — they go, with the current text. */
  readonly newer: number;
  readonly lines: readonly DiffLine[];
}

/**
 * The bodies kept beside the open note. Going back to one is irreversible — the current text
 * and every newer version go — so a row opens a preview, and only the preview restores.
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
  private readonly _preview = signal<RevisionPreview | null>(null);

  readonly revisions = this._revisions.asReadonly();
  readonly isOpen = this._isOpen.asReadonly();
  readonly isWorking = this._isWorking.asReadonly();
  readonly preview = this._preview.asReadonly();

  /**
   * Bumped by every restore; the editor's body draft keys on it, being a `linkedSignal` on the
   * note id, which a restore does not change.
   */
  readonly restored = this._restored.asReadonly();

  /** An empty panel is not offered: the editor is dense enough. */
  readonly hasHistory = computed(() => this._revisions().length > 0);

  /**
   * Called whenever the editor changes note, the draft included. ⚠️ It reloads even for the note
   * it is already on: the editor is destroyed when it closes, so nothing calls this with `null`
   * on the way out, and skipping a matching id shows a stale history.
   */
  async openFor(noteId: string | null): Promise<void> {
    const moved = noteId !== this._noteId();
    this._noteId.set(noteId);

    if (moved) {
      this._revisions.set([]);
      this._isOpen.set(false);
    }
    this._preview.set(null);

    if (noteId === null) return;

    await this.reload();
  }

  toggle(): void {
    this._isOpen.update((open) => !open);
    this._preview.set(null);
  }

  async openPreview(revisionId: string): Promise<void> {
    const noteId = this._noteId();
    const newer = this._revisions().findIndex((revision) => revision.id === revisionId);
    const revision = this._revisions()[newer];
    if (noteId === null || revision === undefined) return;

    const lines = await this.notifier.attempt('errors.revisionDiffFailed', () =>
      this.repository.revisionDiff(noteId, revisionId),
    );
    // The editor may have moved to another note while the comparison was out.
    if (lines === null || this._noteId() !== noteId) return;

    this._preview.set({ revision, newer, lines });
  }

  closePreview(): void {
    this._preview.set(null);
  }

  /** Goes back to the version being previewed, and to nothing else. */
  async restore(): Promise<void> {
    const noteId = this._noteId();
    const preview = this._preview();
    if (noteId === null || preview === null || this._isWorking()) return;

    await this.notifier.attemptWhile(this._isWorking, 'errors.revisionRestoreFailed', async () => {
      // Adopted before the counter bumps, which re-seeds the editor's draft from the open note.
      this.notes.adoptRestored(await this.repository.restoreRevision(noteId, preview.revision.id));
      this._preview.set(null);
      this._restored.update((count) => count + 1);
      // The canvas and the board both read this: a body put back reaches whichever is showing.
      this.revision.bump();
      await this.reload();
    });
  }

  private async reload(): Promise<void> {
    const noteId = this._noteId();
    if (noteId === null) return;

    const revisions = await this.notifier.attempt('errors.revisionsListFailed', () =>
      this.repository.listRevisions(noteId),
    );
    if (revisions !== null) this._revisions.set(revisions);
  }
}
