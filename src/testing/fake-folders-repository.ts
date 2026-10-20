import { guard } from './fail-next';
import { FoldersRepository } from '@core/data/folders.repository';
import { Folder, FolderColour, FolderDraft, FolderMove, NoteFiling } from '@core/model/folder.model';

/** The palette the real one rotates through, so a spec can assert the colour it assigns. */
const COLOURS: readonly FolderColour[] = ['blue', 'amber', 'purple', 'green', 'red'];

export class FakeFoldersRepository implements Pick<FoldersRepository, keyof FoldersRepository> {
  private folders: readonly Folder[];
  private nextId = 0;

  /** Which folder each note sits in, so filing and its undo can be asserted. */
  readonly filings = new Map<string, string | null>();

  /** When set, the next call to any method rejects with this error, then clears. */
  failNext: Error | null = null;

  constructor(folders: readonly Folder[] = []) {
    this.folders = folders;
  }

  loadAll(spaceId: string | null): Promise<readonly Folder[]> {
    return guard(this, () =>
      spaceId === null ? this.folders : this.folders.filter((folder) => folder.spaceId === spaceId),
    );
  }

  create(draft: FolderDraft): Promise<Folder> {
    return guard(this, () => {
      const siblings = this.folders.filter((folder) => folder.spaceId === draft.spaceId).length;
      const folder: Folder = {
        id: `fake-folder-${++this.nextId}`,
        spaceId: draft.spaceId,
        name: draft.name,
        colour: COLOURS[siblings % COLOURS.length] ?? 'blue',
        createdAt: new Date(),
        parentId: draft.parentId ?? null,
      };
      this.folders = [...this.folders, folder];
      return folder;
    });
  }

  rename(id: string, name: string): Promise<Folder> {
    return guard(this, () => this.adopt(id, (folder) => ({ ...folder, name })));
  }

  recolour(id: string, colour: FolderColour): Promise<Folder> {
    return guard(this, () => this.adopt(id, (folder) => ({ ...folder, colour })));
  }

  /** Moves whatever it is asked to: the refusals are Rust's, and the specs that need one fail it. */
  move(id: string, parentId: string | null): Promise<FolderMove> {
    return guard(this, () => {
      const folder = this.folders.find((candidate) => candidate.id === id);
      if (!folder) throw new Error(`Unknown folder: ${id}`);
      const was: FolderMove = { folderId: id, parentId: folder.parentId, frame: null };
      if (folder.parentId === parentId) return was;

      this.adopt(id, (current) => ({ ...current, parentId }));
      return was;
    });
  }

  moveBack(was: FolderMove): Promise<boolean> {
    return guard(this, () => {
      const parentId = was.parentId ?? null;
      if (parentId !== null && !this.folders.some((folder) => folder.id === parentId)) return false;

      this.adopt(was.folderId, (current) => ({ ...current, parentId }));
      return true;
    });
  }

  /** Hands what it held to its parent, as the real one does. */
  delete(id: string): Promise<void> {
    return guard(this, () => {
      const parentId = this.folders.find((folder) => folder.id === id)?.parentId ?? null;
      this.folders = this.folders
        .filter((folder) => folder.id !== id)
        .map((folder) => (folder.parentId === id ? { ...folder, parentId } : folder));
      for (const [noteId, folderId] of this.filings) {
        if (folderId === id) this.filings.set(noteId, parentId);
      }
    });
  }

  /** Answers only what actually moved, like the real one. */
  fileMany(ids: readonly string[], folderId: string | null): Promise<readonly NoteFiling[]> {
    return guard(this, () => {
      const moved: NoteFiling[] = [];
      for (const noteId of ids) {
        const current = this.filings.get(noteId) ?? null;
        if (current === folderId) continue;

        moved.push({ noteId, folderId: current });
        this.filings.set(noteId, folderId);
      }
      return moved;
    });
  }

  fileBack(filings: readonly NoteFiling[]): Promise<number> {
    return guard(this, () => {
      for (const filing of filings) {
        this.filings.set(filing.noteId, filing.folderId ?? null);
      }
      return filings.length;
    });
  }

  private adopt(id: string, change: (folder: Folder) => Folder): Folder {
    const existing = this.folders.find((folder) => folder.id === id);
    if (!existing) {
      throw new Error(`Unknown folder: ${id}`);
    }

    const updated = change(existing);
    this.folders = this.folders.map((folder) => (folder.id === id ? updated : folder));
    return updated;
  }
}
