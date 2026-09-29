import { ChangeDetectionStrategy, Component, inject } from '@angular/core';
import { Folder } from '@core/model/folder.model';
import { AreaStore } from '@core/services/areas/area.store';
import { FoldersStore } from '@core/state/folders.store';
import { SpacesStore } from '@core/state/spaces.store';
import { SettingsStore } from '@core/services/settings/settings.store';
import { AreaSwitchComponent } from '@shared/controls/area-switch/area-switch.component';
import { LibraryTreeComponent } from './library-tree/library-tree.component';

/** The library rail, while it is shown, headed by the switch between areas. */
@Component({
  selector: 'app-notes-sidebar',
  imports: [LibraryTreeComponent, AreaSwitchComponent],
  templateUrl: './notes-sidebar.component.html',
  styles: ':host { display: contents; } .rail-switch { margin-bottom: 10px; }',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class NotesSidebarComponent {
  protected readonly settings = inject(SettingsStore);
  protected readonly areas = inject(AreaStore);
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
