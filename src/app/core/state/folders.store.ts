import { Injectable, Signal, computed, effect, inject, linkedSignal, resource, signal } from '@angular/core';
import { ErrorNotifier } from '@core/services/errors/error-notifier.service';
import { FoldersRepository } from '@core/data/folders.repository';
import { Folder, FolderColour } from '@core/model/folder.model';
import { NotesRevision } from './notes-revision';
import { SpacesStore } from './spaces.store';

/**
 * The folders of the active space, and which one narrows the canvas.
 *
 * `null` on both counts is a choice, not a waiting state: no active space means the
 * user asked for all of them, and no active folder means every note, filed or not.
 */
@Injectable({ providedIn: 'root' })
export class FoldersStore {
  private readonly repository = inject(FoldersRepository);
  private readonly notifier = inject(ErrorNotifier);
  private readonly revision = inject(NotesRevision);
  private readonly spaces = inject(SpacesStore);

  /**
   * ⚠️ Every space's folders and not the active space's: the rail draws the whole library
   * as a tree. Narrowing back to one space is a partition of a list already in hand, which
   * is also what lets a folder in another space be opened without a round trip first.
   */
  private readonly foldersResource = resource({
    // Keyed on the revision: the seeding writes its folders after this has read an empty
    // database, and nothing else would tell the rail they exist.
    params: () => ({ revision: this.revision.current() }),
    // No `defaultValue`: it would make `hasValue()` true with an empty list while a
    // reload is in flight, and the retained value below would be that empty list.
    loader: (): Promise<readonly Folder[]> => this.repository.loadAll(null),
  });

  /**
   * Retained across a reload, like `NotesQueryStore.view`: a write bumps the revision
   * and the rail would blink empty each time. Writable on purpose — a write adopts what
   * persistence answered, and the reload it triggers replaces it with the same thing.
   */
  private readonly known = linkedSignal<readonly Folder[] | undefined, readonly Folder[]>({
    source: () => (this.foldersResource.hasValue() ? this.foldersResource.value() : undefined),
    computation: (fresh, previous) => fresh ?? previous?.value ?? [],
  });

  /** In creation order, which is the order the board lays its zones out in. */
  readonly allFolders: Signal<readonly Folder[]> = this.known.asReadonly();

  /** `null` = every space, so every folder: the choice the switcher already made. */
  readonly folders = computed<readonly Folder[]>(() => {
    const spaceId = this.spaces.activeSpaceId();
    return spaceId === null ? this.allFolders() : this.foldersOf(spaceId);
  });

  readonly isLoading = this.foldersResource.isLoading;
  readonly loadError: Signal<Error | undefined> = this.foldersResource.error;

  private readonly _activeFolderId = signal<string | null>(null);

  /** An id the current space does not hold falls back to "every folder". */
  readonly activeFolderId = computed<string | null>(() => this.activeFolder()?.id ?? null);

  readonly activeFolder = computed<Folder | null>(() => {
    const activeId = this._activeFolderId();
    return activeId === null ? null : (this.folders().find((folder) => folder.id === activeId) ?? null);
  });

  constructor() {
    // A failure here empties no screen, so without a banner it would go unnoticed.
    effect(() => {
      const error = this.loadError();
      if (error) {
        this.notifier.notify({ ref: { key: 'errors.foldersLoadFailed' }, detail: error.message });
      }
    });
  }

  foldersOf(spaceId: string): readonly Folder[] {
    return this.allFolders().filter((folder) => folder.spaceId === spaceId);
  }

  reload(): void {
    this.foldersResource.reload();
  }

  selectFolder(id: string | null): void {
    this._activeFolderId.set(id);
  }

  /** Uniqueness is not checked here: only storage sees the real state of the database. */
  async createFolder(name: string): Promise<Folder | null> {
    const trimmed = name.trim();
    const spaceId = this.spaces.activeSpaceId();
    if (!trimmed || spaceId === null) return null;

    const created = await this.notifier.attempt(
      'errors.folderCreateFailed',
      () => this.repository.create({ spaceId, name: trimmed }),
      { name: trimmed },
    );
    if (!created) return null;

    this.known.set([...this.allFolders(), created]);
    // The rail shows it at once from the line above; the **board** would not draw its
    // zone until something else happened to reload it. `createZone` — the same folder made
    // by drawing a band — has always bumped for exactly that reason.
    this.revision.bump();
    return created;
  }

  /** Not optimistic: the list adopts only what persistence returned. */
  async renameFolder(id: string, name: string): Promise<boolean> {
    const trimmed = name.trim();
    const current = this.allFolders().find((folder) => folder.id === id);
    if (!trimmed || !current || current.name === trimmed) return false;

    const renamed = await this.notifier.attempt(
      'errors.folderRenameFailed',
      () => this.repository.rename(id, trimmed),
      { name: trimmed },
    );
    if (!renamed) return false;

    this.adopt(renamed);
    // The chips on the cards carry the name, and the back end resolved them.
    this.revision.bump();
    return true;
  }

  async recolourFolder(id: string, colour: FolderColour): Promise<boolean> {
    const recoloured = await this.notifier.attempt('errors.folderRecolourFailed', () =>
      this.repository.recolour(id, colour),
    );
    if (!recoloured) return false;

    this.adopt(recoloured);
    this.revision.bump();
    return true;
  }

  /** No refuge to choose: the notes stay where they are and come out loose. */
  async deleteFolder(id: string): Promise<boolean> {
    const deleted = await this.notifier.attempt('errors.folderDeleteFailed', () =>
      this.repository.delete(id),
    );
    if (deleted === null) return false;

    this.known.set(this.allFolders().filter((folder) => folder.id !== id));
    if (this._activeFolderId() === id) {
      this.selectFolder(null);
    }
    // Its notes lost their chip, which nothing else would tell the canvas.
    this.revision.bump();
    return true;
  }

  private adopt(folder: Folder): void {
    this.known.set(this.allFolders().map((current) => (current.id === folder.id ? folder : current)));
  }
}
