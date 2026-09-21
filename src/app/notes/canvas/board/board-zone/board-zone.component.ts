import { ChangeDetectionStrategy, Component, computed, input, output, signal } from '@angular/core';
import { TranslocoPipe } from '@jsverse/transloco';
import { BoardFrame, BoardNote, BoardZone } from '@core/model/board.model';
import { isCardControl } from '../board-gesture';
import { NoteActivation, NoteCardComponent } from '@notes/canvas/note-card/note-card.component';
import {
  FolderEditorComponent,
  FolderRecolouring,
  FolderRenaming,
} from '@notes/header/folder-editor/folder-editor.component';

/** A card grabbed inside a zone, with where it sits so the drag keeps its offset. */
export interface CardGrab {
  readonly event: PointerEvent;
  readonly entry: BoardNote;
  readonly frame: BoardFrame;
}

/**
 * A folder drawn as a titled, coloured region. Its notes **flow** inside it rather than
 * carrying coordinates of their own: the inside of a folder is already sorted by the fact
 * of being there, and a second set of positions would be a second thing to keep straight.
 *
 * ⚠️ Moving a zone carries its notes, and resizing one captures and releases nothing —
 * both fall out of the flow rather than being coded. Unreal's own rule, where a comment
 * owns whatever it overlaps, was considered and refused: it silently refiles notes the day
 * a frame is stretched.
 */
@Component({
  selector: 'app-board-zone',
  imports: [NoteCardComponent, TranslocoPipe, FolderEditorComponent],
  templateUrl: './board-zone.component.html',
  styleUrl: './board-zone.component.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class BoardZoneComponent {
  readonly zone = input.required<BoardZone>();
  /** The frame as it is right now — the board's, so a drag in progress shows. */
  readonly frame = input.required<BoardFrame>();
  readonly editable = input(true);
  readonly isDropTarget = input(false);
  readonly isMoving = input(false);
  /** The card being dragged out of here, if any: it is drawn on the surface instead. */
  readonly draggingNoteId = input<string | null>(null);

  readonly noteActivated = output<NoteActivation>();
  readonly opened = output<string>();
  readonly headGrabbed = output<PointerEvent>();
  readonly resizeGrabbed = output<PointerEvent>();
  readonly cardGrabbed = output<CardGrab>();
  readonly renamed = output<FolderRenaming>();
  readonly recoloured = output<FolderRecolouring>();
  readonly deleted = output<string>();
  readonly selectRequested = output<string>();

  /** The same three actions the breadcrumb and the switcher offer, in the same panel. */
  protected readonly menuOpen = signal(false);

  protected readonly headingId = computed(() => `board-zone-${this.zone().folder.id}`);

  protected onSelectRequested(folderId: string): void {
    this.menuOpen.set(false);
    this.selectRequested.emit(folderId);
  }

  protected onRenamed(renaming: FolderRenaming): void {
    this.menuOpen.set(false);
    this.renamed.emit(renaming);
  }

  protected onDeleted(id: string): void {
    this.menuOpen.set(false);
    this.deleted.emit(id);
  }

  /**
   * A card flows here, so its drag starts from where the zone is rather than from itself.
   *
   * ⚠️ Not from its own controls: a press on the tick, the copy, the ⋯ or a checklist item
   * is aimed at that control, and starting a gesture there would swallow its click.
   */
  protected grabCard(event: PointerEvent, entry: BoardNote): void {
    if (isCardControl(event.target)) return;

    const box = (event.currentTarget as HTMLElement).closest('.zone-card')?.getBoundingClientRect();
    const surface = (event.currentTarget as HTMLElement).closest('.board-surface')?.getBoundingClientRect();

    const frame: BoardFrame =
      box && surface
        ? {
            x: Math.round(box.left - surface.left),
            y: Math.round(box.top - surface.top),
            width: 0,
            height: 0,
          }
        : { x: this.frame().x, y: this.frame().y, width: 0, height: 0 };

    this.cardGrabbed.emit({ event, entry, frame });
  }
}
