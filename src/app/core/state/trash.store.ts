import { Injectable, computed, inject, signal } from '@angular/core';
import { ErrorNotifier } from '@core/services/errors/error-notifier.service';
import { NotesRepository } from '@core/data/notes.repository';
import { TrashedNote } from '@core/model/note.model';
import { SEARCH_DEBOUNCE_MS, debounced } from '@core/services/time/debounce';
import { NotesRevision } from './notes-revision';

/** Loaded when the panel opens: a permanent resource would query on every deletion. */
@Injectable({ providedIn: 'root' })
export class TrashStore {
  private readonly repository = inject(NotesRepository);
  private readonly notifier = inject(ErrorNotifier);
  private readonly revision = inject(NotesRevision);

  private readonly _notes = signal<readonly TrashedNote[]>([]);
  private readonly _search = signal('');
  private readonly _isLoading = signal(false);
  private readonly _isOpen = signal(false);

  readonly notes = this._notes.asReadonly();
  readonly isLoading = this._isLoading.asReadonly();
  readonly isOpen = this._isOpen.asReadonly();
  readonly isEmpty = computed(() => !this._isLoading() && this._notes().length === 0);
  /** What the trash is searched for; the list on screen is what Rust matched. */
  readonly search = this._search.asReadonly();

  private readonly searchLater = debounced<void>(() => void this.load(), SEARCH_DEBOUNCE_MS);

  async open(): Promise<void> {
    this._search.set('');
    this._isOpen.set(true);
    await this.load();
  }

  close(): void {
    this.searchLater.cancel();
    this._isOpen.set(false);
  }

  /** Asked as it is typed, after the canvas search's pause. */
  setSearch(query: string): void {
    this._search.set(query);
    this.searchLater();
  }

  /** The back end purges what retention has caught up with before answering. */
  async load(): Promise<void> {
    const notes = await this.notifier.attemptWhile(this._isLoading, 'errors.trashLoadFailed', () =>
      this.repository.loadTrash(this._search()),
    );
    if (notes !== null) this._notes.set(notes);
  }

  /** `true` when a note came back, which is what bumps the canvas revision. */
  async restore(id: string): Promise<boolean> {
    return this.run(() => this.repository.restore([id]));
  }

  async purge(id: string): Promise<boolean> {
    return this.run(() => this.repository.purge([id]));
  }

  /** What the search found, or everything, back at once: the way out of a mistaken bulk deletion. */
  async restoreAll(): Promise<boolean> {
    this.searchLater.cancel();
    return this.run(() => this.repository.restoreTrash(this._search()));
  }

  async emptyTrash(): Promise<boolean> {
    return this.run(() => this.repository.emptyTrash());
  }

  private async run(action: () => Promise<number>): Promise<boolean> {
    if ((await this.notifier.attempt('errors.trashActionFailed', action)) === null) return false;

    this.revision.bump();
    await this.load();
    return true;
  }
}
