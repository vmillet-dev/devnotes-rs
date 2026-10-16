import { ChangeDetectionStrategy, Component, inject } from '@angular/core';
import { AttachmentsStore } from '@core/state/attachments.store';
import { BoardStore } from '@core/state/board.store';
import { NotesStore } from '@core/state/notes.store';
import { FoldersStore } from '@core/state/folders.store';
import { PlaceholderFillStore } from '@core/state/placeholder-fill.store';
import { TagsStore } from '@core/state/tags.store';
import { TrashStore } from '@core/state/trash.store';
import { UndoStore } from '@core/state/undo.store';
import { FolderNamePromptComponent } from './folder-name-prompt/folder-name-prompt.component';
import { ImageLightboxComponent } from './image-lightbox/image-lightbox.component';
import { NoteEditorOverlayComponent } from './note-editor-overlay/note-editor-overlay.component';
import { PlaceholderFormComponent } from '@app/placeholder-form/placeholder-form.component';
import { TagManagerComponent } from './tag-manager/tag-manager.component';
import { TrashPanelComponent } from './trash-panel/trash-panel.component';
import { UndoBarComponent } from './undo-bar/undo-bar.component';

/** Everything drawn over the page, each shown while its store says so. */
@Component({
  selector: 'app-notes-overlays',
  imports: [
    NoteEditorOverlayComponent,
    ImageLightboxComponent,
    PlaceholderFormComponent,
    TrashPanelComponent,
    TagManagerComponent,
    FolderNamePromptComponent,
    UndoBarComponent,
  ],
  templateUrl: './notes-overlays.component.html',
  styles: ':host { display: contents; }',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class NotesOverlaysComponent {
  protected readonly store = inject(NotesStore);
  protected readonly attachments = inject(AttachmentsStore);
  protected readonly folders = inject(FoldersStore);
  protected readonly fill = inject(PlaceholderFillStore);
  protected readonly trash = inject(TrashStore);
  protected readonly tags = inject(TagsStore);
  protected readonly board = inject(BoardStore);
  protected readonly undo = inject(UndoStore);

  protected onRestore(id: string): void {
    void this.trash.restore(id);
  }

  protected onRenameTag(into: string): void {
    void this.tags.proposeRename(into);
  }

  protected onDeleteTags(): void {
    void this.tags.proposeDelete();
  }
}
