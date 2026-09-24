import { Injectable, computed, inject, signal } from '@angular/core';
import { ErrorNotifier } from '@core/services/errors/error-notifier.service';
import { NotesRepository } from '@core/data/notes.repository';
import { TagUsage } from '@core/model/note.model';
import { NotesRevision } from './notes-revision';

/**
 * A corpus-wide change, waiting to be confirmed. `notes` is counted distinctly by the back end,
 * never summed from the counts on screen: a note carrying two of the tags is one note.
 */
export interface PendingTagChange {
  readonly kind: 'rename' | 'merge' | 'delete';
  readonly tags: readonly string[];
  /** Empty for a deletion. */
  readonly into: string;
  readonly notes: number;
}

/** Scoped to the whole corpus: a tag that drifts drifts everywhere. */
@Injectable({ providedIn: 'root' })
export class TagsStore {
  private readonly repository = inject(NotesRepository);
  private readonly notifier = inject(ErrorNotifier);
  private readonly revision = inject(NotesRevision);

  private readonly _tags = signal<readonly TagUsage[]>([]);
  private readonly _isLoading = signal(false);
  private readonly _isOpen = signal(false);
  private readonly _selected = signal<ReadonlySet<string>>(new Set());
  private readonly _pending = signal<PendingTagChange | null>(null);

  readonly tags = this._tags.asReadonly();
  readonly isLoading = this._isLoading.asReadonly();
  readonly isOpen = this._isOpen.asReadonly();
  readonly selected = this._selected.asReadonly();

  readonly pending = this._pending.asReadonly();
  readonly isEmpty = computed(() => !this._isLoading() && this._tags().length === 0);

  /** A merge needs at least two tags; a rename exactly one. */
  readonly selectedCount = computed(() => this._selected().size);

  async open(): Promise<void> {
    this._isOpen.set(true);
    this._selected.set(new Set());
    await this.load();
  }

  close(): void {
    this._isOpen.set(false);
    this._pending.set(null);
  }

  async load(): Promise<void> {
    const tags = await this.notifier.attemptWhile(this._isLoading, 'errors.tagsLoadFailed', () =>
      this.repository.loadTags(),
    );
    if (tags !== null) this._tags.set(tags);
  }

  toggle(tag: string): void {
    // Changing the selection withdraws a proposal made about the old one.
    this._pending.set(null);
    this._selected.update((selection) => {
      const next = new Set(selection);
      if (!next.delete(tag)) {
        next.add(tag);
      }
      return next;
    });
  }

  /** Renaming onto an existing tag is a merge: a note cannot carry one twice. */
  async proposeRename(into: string): Promise<void> {
    const selection = [...this._selected()];
    if (selection.length === 0 || !into.trim()) return;

    await this.propose(selection.length === 1 ? 'rename' : 'merge', selection, into.trim());
  }

  async proposeDelete(): Promise<void> {
    const selection = [...this._selected()];
    if (selection.length === 0) return;

    await this.propose('delete', selection, '');
  }

  cancel(): void {
    this._pending.set(null);
  }

  async confirm(): Promise<boolean> {
    const change = this._pending();
    if (!change) return false;

    this._pending.set(null);

    return this.run(() =>
      change.kind === 'delete'
        ? this.repository.deleteTags(change.tags)
        : this.repository.renameTags(change.tags, change.into),
    );
  }

  /** A blast radius that cannot be read leaves nothing pending, so nothing runs. */
  private async propose(
    kind: PendingTagChange['kind'],
    tags: readonly string[],
    into: string,
  ): Promise<void> {
    const notes = await this.notifier.attempt('errors.tagActionFailed', () =>
      this.repository.countNotesTagged(tags),
    );
    if (notes === null) return;

    this._pending.set({ kind, tags, into, notes });
  }

  private async run(action: () => Promise<number>): Promise<boolean> {
    if ((await this.notifier.attempt('errors.tagActionFailed', action)) === null) return false;

    // Retagging rewrites the corpus: the rail and the cards are both stale.
    this.revision.bump();
    this._selected.set(new Set());
    this._pending.set(null);
    await this.load();
    return true;
  }
}
