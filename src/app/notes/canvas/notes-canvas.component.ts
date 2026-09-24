import { ChangeDetectionStrategy, Component, computed, inject } from '@angular/core';
import { TranslocoPipe } from '@jsverse/transloco';
import { BoardFrame } from '@core/model/board.model';
import { BoardStore } from '@core/state/board.store';
import { FoldersStore } from '@core/state/folders.store';
import { TransferStore } from '@core/state/transfer.store';
import { NoteBatchStore } from '@core/state/note-batch.store';
import { NoteSelectionStore } from '@core/state/note-selection.store';
import { NotesQueryStore } from '@core/state/notes-query.store';
import { NotesStore } from '@core/state/notes.store';
import { SpacesStore } from '@core/state/spaces.store';
import { HelpStore } from '@core/services/help/help.store';
import { SelectionBarComponent } from '../header/selection-bar/selection-bar.component';
import { BoardComponent, CardDrop } from './board/board.component';
import { NoteActivation } from './note-card/note-card.component';
import { NoteSectionComponent } from './note-section/note-section.component';

/** The cards, in whichever of the two views fills the region, and the legend under them. */
@Component({
  selector: 'app-notes-canvas',
  imports: [BoardComponent, NoteSectionComponent, SelectionBarComponent, TranslocoPipe],
  templateUrl: './notes-canvas.component.html',
  styleUrl: './notes-canvas.component.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class NotesCanvasComponent {
  protected readonly canvas = inject(NotesQueryStore);
  protected readonly selection = inject(NoteSelectionStore);
  protected readonly store = inject(NotesStore);
  protected readonly batch = inject(NoteBatchStore);
  protected readonly spaces = inject(SpacesStore);
  protected readonly folders = inject(FoldersStore);
  protected readonly board = inject(BoardStore);
  private readonly transfer = inject(TransferStore);
  /** The guide, opened at the chapter about whatever is empty on screen. */
  protected readonly help = inject(HelpStore);

  /**
   * The legend names keys that act on cards, so it stands down whenever the region holds
   * something else — loading, an error, or the empty state, which already owns the screen
   * and offers the one thing to do from there.
   */
  protected readonly showsKeyboardHint = computed(() => {
    if (this.board.isShowing()) {
      return !this.board.isLoading() && this.board.loadError() === undefined;
    }
    return !this.canvas.isLoading() && this.canvas.loadError() === undefined && !this.canvas.hasNoResults();
  });

  protected onNoteActivated({ noteId, toggleChecked, extendRange }: NoteActivation): void {
    if (extendRange) {
      this.selection.checkRangeTo(noteId);
      return;
    }
    if (toggleChecked) {
      this.selection.toggleChecked(noteId);
      this.selection.focusNote(noteId);
      return;
    }
    this.store.openNote(noteId);
  }

  protected onCardDropped({ noteId, folderId, position }: CardDrop): void {
    void this.board.dropCard(noteId, folderId, position);
  }

  protected onZoneDrawn(frame: BoardFrame): void {
    this.board.proposeZone(frame);
  }

  protected onCopySelection(): void {
    void this.transfer.copyAsMarkdown(this.selection.checkedNoteIds());
  }
}
