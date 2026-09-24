import { FolderColour } from '@core/model/folder.model';
import { FoldersStore } from '@core/state/folders.store';
import { SpacesStore } from '@core/state/spaces.store';

/** What a folder editor asked of its store, for a host drawn without a library behind it. */
export class RecordingFolderActions implements Pick<
  FoldersStore,
  'renameFolder' | 'recolourFolder' | 'deleteFolder'
> {
  readonly renamed: { id: string; name: string }[] = [];
  readonly recoloured: { id: string; colour: FolderColour }[] = [];
  readonly deleted: string[] = [];

  renameFolder(id: string, name: string): Promise<boolean> {
    this.renamed.push({ id, name });
    return Promise.resolve(true);
  }

  recolourFolder(id: string, colour: FolderColour): Promise<boolean> {
    this.recoloured.push({ id, colour });
    return Promise.resolve(true);
  }

  deleteFolder(id: string): Promise<boolean> {
    this.deleted.push(id);
    return Promise.resolve(true);
  }
}

/** The same for a space editor. */
export class RecordingSpaceActions implements Pick<
  SpacesStore,
  'renameSpace' | 'togglePinned' | 'deleteSpace'
> {
  readonly renamed: { id: string; name: string }[] = [];
  readonly pinned: string[] = [];
  readonly deleted: { id: string; targetSpaceId: string }[] = [];

  renameSpace(id: string, name: string): Promise<boolean> {
    this.renamed.push({ id, name });
    return Promise.resolve(true);
  }

  togglePinned(id: string): Promise<boolean> {
    this.pinned.push(id);
    return Promise.resolve(true);
  }

  deleteSpace(id: string, targetSpaceId: string): Promise<boolean> {
    this.deleted.push({ id, targetSpaceId });
    return Promise.resolve(true);
  }
}
