import { ChangeDetectionStrategy, Component, inject } from '@angular/core';
import { Folder } from '@core/model/folder.model';
import { FoldersStore } from '@core/state/folders.store';
import { SpacesStore } from '@core/state/spaces.store';
import { SettingsStore } from '@core/services/settings/settings.store';
import { LibraryTreeComponent } from './library-tree/library-tree.component';

/** The library rail, while it is shown. */
@Component({
  selector: 'app-notes-sidebar',
  imports: [LibraryTreeComponent],
  templateUrl: './notes-sidebar.component.html',
  styles: ':host { display: contents; }',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class NotesSidebarComponent {
  protected readonly settings = inject(SettingsStore);
  protected readonly spaces = inject(SpacesStore);
  protected readonly folders = inject(FoldersStore);

  /** Choosing a space leaves whatever folder was open: the row means the space itself. */
  protected onSpaceChosen(id: string | null): void {
    this.folders.selectFolder(null);
    this.spaces.selectSpace(id);
  }

  /** ⚠️ The space first: a folder is resolved against the active space's folders. */
  protected onFolderOpened(folder: Folder): void {
    this.spaces.selectSpace(folder.spaceId);
    this.folders.selectFolder(folder.id);
  }
}
