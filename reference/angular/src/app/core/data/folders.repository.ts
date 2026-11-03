import { Injectable } from '@angular/core';
import { commands } from '@core/ipc/bindings';
import { unwrap } from '@core/ipc/ipc.error';
import type { Folder as WireFolder } from '@core/ipc/bindings';
import { Folder, FolderColour, FolderDraft, FolderMove, NoteFiling } from '@core/model/folder.model';

/** The seams the wire shape has: an optional `colour` and `parentId`, and `createdAt` as an ISO string. */
export function toFolder(dto: WireFolder): Folder {
  return {
    id: dto.id,
    spaceId: dto.spaceId,
    name: dto.name,
    colour: dto.colour ?? 'blue',
    createdAt: new Date(dto.createdAt),
    parentId: dto.parentId ?? null,
  };
}

/**
 * `delete` takes no refuge, unlike `SpacesRepository.delete`: what the folder held goes to
 * its parent, or loose at the top, and "no folder" is a legitimate state rather than data loss.
 */
@Injectable({ providedIn: 'root' })
export class FoldersRepository {
  /** `null` = every space, the same choice `NotesQuery.spaceId` makes. */
  async loadAll(spaceId: string | null): Promise<readonly Folder[]> {
    return unwrap('list_folders', await commands.listFolders(spaceId)).map(toFolder);
  }

  async create(draft: FolderDraft): Promise<Folder> {
    const wire = { ...draft, parentId: draft.parentId ?? null };
    return toFolder(unwrap('create_folder', await commands.createFolder(wire)));
  }

  /** Answers where it was — its parent and its frame — the only thing that can put it back. */
  async move(id: string, parentId: string | null): Promise<FolderMove> {
    return unwrap('move_folder', await commands.moveFolder(id, parentId));
  }

  /** `false` when its old parent is gone, or no longer a place for it. */
  async moveBack(was: FolderMove): Promise<boolean> {
    return unwrap('move_folder_back', await commands.moveFolderBack(was));
  }

  async rename(id: string, name: string): Promise<Folder> {
    return toFolder(unwrap('rename_folder', await commands.renameFolder(id, name)));
  }

  async recolour(id: string, colour: FolderColour): Promise<Folder> {
    return toFolder(unwrap('recolour_folder', await commands.recolourFolder(id, colour)));
  }

  async delete(id: string): Promise<void> {
    unwrap('delete_folder', await commands.deleteFolder(id));
  }

  /** Answers where each note was filed, which is the only thing that can put it back. */
  async fileMany(ids: readonly string[], folderId: string | null): Promise<readonly NoteFiling[]> {
    return unwrap('file_notes', await commands.fileNotes([...ids], folderId));
  }

  async fileBack(filings: readonly NoteFiling[]): Promise<number> {
    return unwrap('file_notes_back', await commands.fileNotesBack([...filings]));
  }
}
