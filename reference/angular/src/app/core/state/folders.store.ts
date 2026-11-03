import { Injectable, Signal, computed, effect, inject, linkedSignal, resource, signal } from '@angular/core';
import { ErrorNotifier } from '@core/services/errors/error-notifier.service';
import { LibraryPreferencesService } from '@core/services/preferences/library-preferences.service';
import { FoldersRepository } from '@core/data/folders.repository';
import { Folder, FolderColour, FolderMove, childrenOf, pathOf } from '@core/model/folder.model';
import { NotesRevision } from './notes-revision';
import { SpacesStore } from './spaces.store';

/** Kept with the space it belongs to: a folder of another space falls back on every note. */
const ACTIVE_FOLDER_KEY = 'devnotes.notes.activeFolder';

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
  private readonly preferences = inject(LibraryPreferencesService);

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
  private readonly _naming = signal(false);

  /** « Nouveau dossier » from the palette: the name is asked over the canvas, in the space open. */
  readonly naming = this._naming.asReadonly();

  /** An id the current space does not hold falls back to "every folder". */
  readonly activeFolderId = computed<string | null>(() => this.activeFolder()?.id ?? null);

  readonly activeFolder = computed<Folder | null>(() => {
    const activeId = this._activeFolderId();
    return activeId === null ? null : (this.folders().find((folder) => folder.id === activeId) ?? null);
  });

  /** The open folder and the ones above it, the top first: the breadcrumb's. */
  readonly activePath = computed<readonly Folder[]>(() => {
    const active = this.activeFolder();
    return active === null ? [] : pathOf(this.allFolders(), active.id);
  });

  /** Whether the open folder holds folders: its board is then a board, and not a flat grid. */
  readonly activeHasChildren = computed(() => {
    const active = this.activeFolderId();
    return active !== null && this.allFolders().some((folder) => folder.parentId === active);
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

  /** The folder the library was left in, read with the space when the notes page opens. */
  restoreRemembered(): void {
    const id = this.preferences.read(ACTIVE_FOLDER_KEY);
    if (id) this._activeFolderId.set(id);
  }

  selectFolder(id: string | null): void {
    this._activeFolderId.set(id);
    this.preferences.write(ACTIVE_FOLDER_KEY, id ?? '');
  }

  /** One level up from the open folder, and out at the top: what Escape does last. */
  goUp(): boolean {
    const active = this.activeFolder();
    if (active === null) return false;

    this.selectFolder(active.parentId);
    return true;
  }

  childrenOf(parentId: string | null, spaceId: string): readonly Folder[] {
    return childrenOf(this.foldersOf(spaceId), parentId);
  }

  askName(): void {
    this._naming.set(this.spaces.activeSpaceId() !== null);
  }

  async createNamed(name: string): Promise<void> {
    this._naming.set(false);
    await this.createFolder(name);
  }

  dismissNaming(): void {
    this._naming.set(false);
  }

  /**
   * Uniqueness is not checked here: only storage sees the real state of the database. A
   * sub-folder is in its parent's space, which the rail offers whether or not it is open.
   */
  async createFolder(name: string, parentId: string | null = null): Promise<Folder | null> {
    const trimmed = name.trim();
    const spaceId =
      parentId === null
        ? this.spaces.activeSpaceId()
        : (this.allFolders().find((folder) => folder.id === parentId)?.spaceId ?? null);
    if (!trimmed || spaceId === null) return null;

    const created = await this.notifier.attempt(
      'errors.folderCreateFailed',
      () => this.repository.create({ spaceId, name: trimmed, parentId }),
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

  /**
   * Under another folder of its space, or at its top, with what it holds; the answer is where it
   * was, which the caller records for the undo. Not optimistic: the list adopts the move once
   * persistence has made it.
   */
  async moveFolder(id: string, parentId: string | null): Promise<FolderMove | null> {
    const moved = await this.notifier.attempt('errors.folderMoveFailed', () =>
      this.repository.move(id, parentId),
    );
    if (moved === null) return null;

    this.known.set(this.allFolders().map((folder) => (folder.id === id ? { ...folder, parentId } : folder)));
    this.revision.bump();
    return moved;
  }

  /** No refuge to choose: what it held goes to its parent, or loose at the top of the space. */
  async deleteFolder(id: string): Promise<boolean> {
    const parentId = this.allFolders().find((folder) => folder.id === id)?.parentId ?? null;
    const deleted = await this.notifier.attempt('errors.folderDeleteFailed', () =>
      this.repository.delete(id),
    );
    if (deleted === null) return false;

    this.known.set(
      this.allFolders()
        .filter((folder) => folder.id !== id)
        .map((folder) => (folder.parentId === id ? { ...folder, parentId } : folder)),
    );
    if (this._activeFolderId() === id) {
      this.selectFolder(parentId);
    }
    // Its notes changed folder, which nothing else would tell the canvas.
    this.revision.bump();
    return true;
  }

  private adopt(folder: Folder): void {
    this.known.set(this.allFolders().map((current) => (current.id === folder.id ? folder : current)));
  }
}
