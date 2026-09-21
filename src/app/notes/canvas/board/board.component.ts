import {
  ChangeDetectionStrategy,
  Component,
  ElementRef,
  computed,
  inject,
  input,
  output,
  signal,
} from '@angular/core';
import { TranslocoPipe } from '@jsverse/transloco';
import { BoardFrame, BoardNote, BoardPoint, BoardZone } from '@core/model/board.model';
import { NoteActivation, NoteCardComponent } from '@notes/canvas/note-card/note-card.component';
import { FolderRecolouring, FolderRenaming } from '@notes/header/folder-editor/folder-editor.component';
import { BoardZoneComponent } from './board-zone/board-zone.component';
import {
  Gesture,
  asZone,
  cardPosition,
  drawnTo,
  hasTravelled,
  isCardControl,
  isWorthDrawing,
  movedTo,
  resizedTo,
  zoneAt,
} from './board-gesture';

const NOWHERE: BoardFrame = { x: 0, y: 0, width: 0, height: 0 };

/** What a drop asks the page to do. */
export interface CardDrop {
  readonly noteId: string;
  /** `null` is the free background, which takes the note out of its folder. */
  readonly folderId: string | null;
  readonly position: BoardPoint;
}

export interface ZoneMove {
  readonly folderId: string;
  readonly frame: BoardFrame;
}

/**
 * The second way to look at a space. Zones are placed from stored frames; cards flow
 * inside them and sit freely outside.
 *
 * ⚠️ Pan only, no zoom: a zoom is a second thing to persist and to reset, and full-size
 * cards are what makes the board worth panning in the first place.
 *
 * ⚠️ Every gesture here is pointer events — `pointerdown` + `setPointerCapture` +
 * `pointermove` + `pointerup`, as checklist reordering already is. HTML5 drag and drop
 * does not work in this WebView and cannot be turned on: `dragDropEnabled` has to stay
 * `true` for the native file drop that feeds attachments.
 */
@Component({
  selector: 'app-board',
  imports: [BoardZoneComponent, NoteCardComponent, TranslocoPipe],
  templateUrl: './board.component.html',
  styleUrl: './board.component.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class BoardComponent {
  private readonly host = inject<ElementRef<HTMLElement>>(ElementRef);

  readonly zones = input.required<readonly BoardZone[]>();
  readonly loose = input.required<readonly BoardNote[]>();
  readonly width = input.required<number>();
  readonly height = input.required<number>();
  /** Nothing can be drawn or dropped without a space to file it into. */
  readonly editable = input(true);

  readonly noteActivated = output<NoteActivation>();
  readonly folderOpened = output<string>();
  readonly cardDropped = output<CardDrop>();
  readonly zoneMoved = output<ZoneMove>();
  readonly zoneDrawn = output<BoardFrame>();
  readonly folderRenamed = output<FolderRenaming>();
  readonly folderRecoloured = output<FolderRecolouring>();
  readonly folderDeleted = output<string>();

  protected readonly gesture = signal<Gesture | null>(null);

  /** The zone a drop would land in right now, so it can say so before the pointer lifts. */
  protected readonly hoveredZone = computed<string | null>(() => {
    const drag = this.gesture();
    if (!drag || drag.kind !== 'card' || !drag.moved) return null;

    return zoneAt(this.zoneFrames(), { x: drag.to.x, y: drag.to.y });
  });

  /**
   * The card being dragged out of a zone, drawn on the surface at the pointer.
   *
   * ⚠️ A filed card **flows** inside its zone and has no coordinates of its own, so there
   * is nothing to move: without this the pointer carries nothing at all, and the only
   * feedback left is the zone lighting up under it. A loose card needs none — `positionOf`
   * already moves the real one.
   */
  protected readonly travelling = computed<BoardNote | null>(() => {
    const drag = this.gesture();
    if (drag?.kind !== 'card' || !drag.moved) return null;

    return (
      this.zones()
        .flatMap((zone) => zone.notes)
        .find((entry) => entry.note.id === drag.id) ?? null
    );
  });

  /** Where that card is right now, in surface coordinates. */
  protected readonly travellingAt = computed<BoardPoint>(() => {
    const drag = this.gesture();
    return drag ? { x: drag.to.x, y: drag.to.y } : { x: 0, y: 0 };
  });

  /** The band being drawn, drawn only once the pointer has actually travelled. */
  protected readonly band = computed<BoardFrame | null>(() => {
    const drag = this.gesture();
    return drag?.kind === 'draw-zone' && drag.moved ? drag.to : null;
  });

  /** Where the frames say the zones are *right now*, drag included. */
  private readonly zoneFrames = computed(() =>
    this.zones().map((zone) => ({ id: zone.folder.id, frame: this.frameOf(zone) })),
  );

  protected frameOf(zone: BoardZone): BoardFrame {
    const drag = this.gesture();
    const moving = drag?.moved && drag.id === zone.folder.id;
    return moving && (drag.kind === 'move-zone' || drag.kind === 'resize-zone') ? drag.to : zone.frame;
  }

  protected positionOf(entry: BoardNote): BoardPoint {
    const drag = this.gesture();
    if (drag?.kind === 'card' && drag.moved && drag.id === entry.note.id) {
      return { x: drag.to.x, y: drag.to.y };
    }
    return entry.position ?? { x: 16, y: 16 };
  }

  protected isDragging(id: string): boolean {
    const drag = this.gesture();
    return drag !== null && drag.moved && drag.id === id;
  }

  protected startCard(event: PointerEvent, entry: BoardNote, frame: BoardFrame): void {
    this.begin(event, 'card', entry.note.id, frame);
  }

  /** A loose card already knows where it sits; only its own controls are off limits. */
  protected grabLoose(event: PointerEvent, entry: BoardNote, at: BoardPoint): void {
    if (isCardControl(event.target)) return;

    this.startCard(event, entry, { x: at.x, y: at.y, width: 0, height: 0 });
  }

  /**
   * ⚠️ A drag ends with a click on the card it was dragging: the whole card is the handle
   * now, so the click the pointer leaves behind must not also open the note. Cleared on
   * the next press, or one abandoned drag would eat a legitimate click later.
   */
  protected onCardActivated(activation: NoteActivation): void {
    if (this.travelled) {
      this.travelled = false;
      return;
    }
    this.noteActivated.emit(activation);
  }

  private travelled = false;

  protected startZoneMove(event: PointerEvent, zone: BoardZone): void {
    this.begin(event, 'move-zone', zone.folder.id, this.frameOf(zone));
  }

  protected startZoneResize(event: PointerEvent, zone: BoardZone): void {
    this.begin(event, 'resize-zone', zone.folder.id, this.frameOf(zone));
  }

  /** ⚠️ Only on the background itself: a pointerdown that bubbled up from a zone or a
   *  card would start drawing a band under whatever the user actually grabbed. */
  protected startDraw(event: PointerEvent): void {
    if (!this.editable() || event.button !== 0 || event.target !== event.currentTarget) return;

    this.begin(event, 'draw-zone', '', NOWHERE);
  }

  protected onPointerMove(event: PointerEvent): void {
    const drag = this.gesture();
    if (!drag) return;

    const at = this.surfacePoint(event);
    const moved = drag.moved || hasTravelled(drag.origin, at);
    if (!moved) return;

    this.gesture.set({ ...drag, moved, to: this.frameFor(drag, at) });
  }

  /**
   * ⚠️ The only place a gesture is committed. A drag that never travelled writes nothing —
   * it was a click — and `pointercancel` throws the whole thing away rather than leaving a
   * card at coordinates nobody chose.
   */
  protected onPointerUp(event: PointerEvent): void {
    const drag = this.gesture();
    this.gesture.set(null);
    this.release(event);
    if (!drag || !drag.moved) return;

    const at = this.surfacePoint(event);

    switch (drag.kind) {
      case 'card': {
        // What tells the click that follows to be dropped rather than to open the note.
        this.travelled = true;
        const position = cardPosition(drag, at);
        this.cardDropped.emit({
          noteId: drag.id,
          folderId: zoneAt(this.zoneFrames(), position),
          position,
        });
        break;
      }
      case 'move-zone':
      case 'resize-zone':
        this.zoneMoved.emit({ folderId: drag.id, frame: this.frameFor(drag, at) });
        break;
      case 'draw-zone': {
        const band = drawnTo(drag.origin, at);
        if (isWorthDrawing(band)) {
          this.zoneDrawn.emit(asZone(band));
        }
        break;
      }
    }
  }

  /** A pointer the system took back — a right-click, a gesture, a window that lost focus. */
  protected onPointerCancel(event: PointerEvent): void {
    this.gesture.set(null);
    this.release(event);
  }

  private begin(event: PointerEvent, kind: Gesture['kind'], id: string, from: BoardFrame): void {
    this.travelled = false;
    if (!this.editable() || event.button !== 0) return;

    event.preventDefault();
    event.stopPropagation();
    // Optional-chained because jsdom lacks it and must not fail the drag.
    (event.target as HTMLElement).setPointerCapture?.(event.pointerId);

    const origin = this.surfacePoint(event);
    this.gesture.set({ kind, id, origin, from, to: from, moved: false });
  }

  private release(event: PointerEvent): void {
    (event.target as HTMLElement).releasePointerCapture?.(event.pointerId);
  }

  /** Exhaustive on purpose: a fourth kind of gesture stops this compiling until it says
   *  how it follows the pointer. */
  private frameFor(drag: Gesture, at: BoardPoint): BoardFrame {
    switch (drag.kind) {
      case 'card':
      case 'move-zone':
        return movedTo(drag, at);
      case 'resize-zone':
        return resizedTo(drag, at);
      case 'draw-zone':
        return drawnTo(drag.origin, at);
    }
  }

  /**
   * ⚠️ Surface coordinates, not viewport ones: the board scrolls, and a frame stored in
   * viewport space would move every time the user pans.
   */
  private surfacePoint(event: PointerEvent): BoardPoint {
    const surface = this.host.nativeElement.querySelector('.board-surface');
    const box = surface?.getBoundingClientRect();
    if (!box) return { x: 0, y: 0 };

    return { x: Math.round(event.clientX - box.left), y: Math.round(event.clientY - box.top) };
  }
}
