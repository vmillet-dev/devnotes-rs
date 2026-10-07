import { Injectable, computed, inject, signal } from '@angular/core';
import { HttpRepository } from '@core/data/http.repository';
import { HttpContents, HttpItem, HttpTree } from '@core/model/http.model';
import { ErrorNotifier } from '@core/services/errors/error-notifier.service';
import { LibraryPreferencesService } from '@core/services/preferences/library-preferences.service';
import { RailMove, holdsRequest, railRows } from './http-tree';

/** In the library's preferences: which folds were left closed belongs to its collections. */
const COLLAPSED_KEY = 'devnotes.notes.http.collapsed';

/**
 * The collections as the rail shows them. Every write is persisted, then the tree read again:
 * Rust renumbers what a move or a copy touched, and the front never guesses the result.
 */
@Injectable({ providedIn: 'root' })
export class HttpCollectionsStore {
  private readonly repository = inject(HttpRepository);
  private readonly errors = inject(ErrorNotifier);
  private readonly preferences = inject(LibraryPreferencesService);

  private readonly _tree = signal<HttpTree>({ collections: [] });
  readonly tree = this._tree.asReadonly();

  private readonly _collapsed = signal<ReadonlySet<string>>(this.readCollapsed());
  readonly collapsed = this._collapsed.asReadonly();

  private readonly _openId = signal<string | null>(null);
  /** The request highlighted in the rail and shown beside it. */
  readonly openId = this._openId.asReadonly();

  readonly rows = computed(() => railRows(this._tree(), this._collapsed()));

  async load(): Promise<void> {
    await this.attempt(async () => this._tree.set(await this.repository.tree()));
  }

  toggle(id: string): void {
    const next = new Set(this._collapsed());
    if (!next.delete(id)) next.add(id);
    this._collapsed.set(next);
    this.preferences.write(COLLAPSED_KEY, JSON.stringify([...next]));
  }

  open(id: string | null): void {
    this._openId.set(id);
  }

  async createCollection(name: string): Promise<void> {
    await this.write(() => this.repository.createCollection(name));
  }

  /** Under a collection's root when `parentId` is `null`; the parent unfolds to show it. */
  async createFolder(collectionId: string, parentId: string | null, name: string): Promise<void> {
    this.unfold(parentId ?? collectionId);
    await this.write(() => this.repository.createFolder(collectionId, parentId, name));
  }

  async createRequest(collectionId: string, folderId: string | null, name: string): Promise<void> {
    this.unfold(folderId ?? collectionId);
    const created = await this.write(() =>
      this.repository.createRequest({
        collectionId,
        folderId,
        name,
        kind: 'http',
        method: 'GET',
        document: { url: '', description: '' },
      }),
    );
    if (created) this._openId.set(created.id);
  }

  async rename(item: HttpItem, name: string): Promise<void> {
    await this.write(() => this.repository.rename(item, name));
  }

  async duplicate(item: HttpItem, name: string): Promise<void> {
    await this.write(() => this.repository.duplicate(item, name));
  }

  /** What a deletion would take with it, read before the confirmation is shown. */
  async contents(item: HttpItem): Promise<HttpContents | null> {
    return this.attempt(() => this.repository.contents(item));
  }

  async delete(item: HttpItem): Promise<void> {
    const done = await this.write(async () => {
      await this.repository.delete(item);
      return true;
    });
    const open = this._openId();
    if (done && open !== null && !holdsRequest(this._tree(), open)) this._openId.set(null);
  }

  async apply(move: RailMove): Promise<void> {
    if (move.kind === 'reorder') {
      await this.write(() => this.repository.reorderCollection(move.id, move.index));
    } else {
      if (move.place.folderId !== null) this.unfold(move.place.folderId);
      await this.write(() => this.repository.move(move.item, move.place));
    }
  }

  private unfold(id: string): void {
    if (this._collapsed().has(id)) this.toggle(id);
  }

  private async write<T>(operation: () => Promise<T>): Promise<T | null> {
    const answer = await this.attempt(operation);
    await this.load();
    return answer;
  }

  private attempt<T>(operation: () => Promise<T>): Promise<T | null> {
    return this.errors.attempt('http.failed', operation);
  }

  private readCollapsed(): ReadonlySet<string> {
    try {
      const stored: unknown = JSON.parse(this.preferences.read(COLLAPSED_KEY) ?? '[]');
      return new Set(Array.isArray(stored) ? stored.filter((id) => typeof id === 'string') : []);
    } catch {
      return new Set();
    }
  }
}
