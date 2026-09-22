import { DOCUMENT } from '@angular/common';
import {
  ChangeDetectionStrategy,
  Component,
  DestroyRef,
  ElementRef,
  computed,
  effect,
  inject,
  input,
  output,
  signal,
} from '@angular/core';
import { TranslocoPipe } from '@jsverse/transloco';
import { BoardFrame, BoardNote, BoardPoint, BoardScope, BoardZone } from '@core/model/board.model';
import { NoteActivation, NoteCardComponent } from '@notes/canvas/note-card/note-card.component';
import { FolderRecolouring, FolderRenaming } from '@notes/header/folder-editor/folder-editor.component';
import { BoardTidyComponent } from './board-tidy/board-tidy.component';
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
  overlaps,
  resizedTo,
  zoneAt,
} from './board-gesture';

const NOWHERE: BoardFrame = { x: 0, y: 0, width: 0, height: 0 };

/** ⚠️ `PointerEvent.button`, where 2 is the right one — not `buttons`, which is a mask. */
const RIGHT_BUTTON = 2;

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
  imports: [BoardTidyComponent, BoardZoneComponent, NoteCardComponent, TranslocoPipe],
  // ⚠️ The whole board, not the surface alone: the tidy control and the ground past the
  // surface's edge are the board too, and a sweep can end over either.
  host: { '(contextmenu)': 'onContextMenu($event)' },
  templateUrl: './board.component.html',
  styleUrl: './board.component.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class BoardComponent {
  private readonly host = inject<ElementRef<HTMLElement>>(ElementRef);
  private readonly document = inject(DOCUMENT);

  readonly zones = input.required<readonly BoardZone[]>();
  readonly loose = input.required<readonly BoardNote[]>();
  readonly width = input.required<number>();
  readonly height = input.required<number>();
  /** Nothing can be drawn or dropped without a space to file it into. */
  readonly editable = input(true);
  /**
   * ⚠️ Two counters, not two booleans: what matters is that the value **changed**, and a
   * board arranged twice running has to pan twice. See `panTo` for what they are for.
   */
  readonly arrangements = input(0);
  readonly restorations = input(0);

  readonly noteActivated = output<NoteActivation>();
  readonly folderOpened = output<string>();
  readonly cardDropped = output<CardDrop>();
  readonly zoneMoved = output<ZoneMove>();
  readonly zoneDrawn = output<BoardFrame>();
  readonly folderRenamed = output<FolderRenaming>();
  readonly folderRecoloured = output<FolderRecolouring>();
  readonly folderDeleted = output<string>();
  readonly folderNotesSelected = output<string>();
  /** Every card a right-drag swept over; the page decides what selecting means. */
  readonly notesBanded = output<readonly string[]>();
  readonly tidyRequested = output<BoardScope>();
  readonly guideRequested = output<void>();

  protected readonly gesture = signal<Gesture | null>(null);

  /** Where the pan was before an arrangement sent it home, so the undo can put it back. */
  private pannedFrom: BoardPoint | null = null;

  constructor() {
    inject(DestroyRef).onDestroy(() => this.disarmMenuGuard());

    // ⚠️ The result of a whole-board arrangement happens **off screen** otherwise. The pan
    // is a native scroll on `.board` and nothing resets it, so on a wide board panned to
    // the right, "Réorganiser" lands everything back at the top left and leaves the user
    // looking at empty dotted ground with a banner announcing success — indistinguishable
    // from an erasure, and the reason nobody presses it a second time.
    effect(() => {
      if (this.arrangements() === 0) return;
      this.pannedFrom = this.scrollOffset();
      this.panTo({ x: 0, y: 0 });
    });

    // ⚠️ The same defect from the other end: undoing puts the board back where it was
    // dragged to, while the pan is at the origin the arrangement sent it to.
    effect(() => {
      if (this.restorations() === 0) return;
      const back = this.pannedFrom;
      this.pannedFrom = null;
      if (back) this.panTo(back);
    });
  }

  private board(): HTMLElement | null {
    return this.host.nativeElement.querySelector<HTMLElement>('.board');
  }

  private scrollOffset(): BoardPoint {
    const board = this.board();
    return { x: board?.scrollLeft ?? 0, y: board?.scrollTop ?? 0 };
  }

  /** Optional-chained because jsdom has no `scrollTo` and must not fail the arrangement. */
  private panTo(at: BoardPoint): void {
    this.board()?.scrollTo?.(at.x, at.y);
  }

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

  /** The same rectangle, when the right button is drawing it to select rather than to file. */
  protected readonly selectionBand = computed<BoardFrame | null>(() => {
    const drag = this.gesture();
    return drag?.kind === 'select-band' && drag.moved ? drag.to : null;
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

  /**
   * ⚠️ Only on the background itself: a pointerdown that bubbled up from a zone or a card
   * would start drawing a band under whatever the user actually grabbed. The **right**
   * button is exempt — it has no other meaning anywhere on the board, so a selection band
   * can start on a card as readily as on empty ground.
   */
  protected startDraw(event: PointerEvent): void {
    if (event.button === RIGHT_BUTTON) {
      this.begin(event, 'select-band', '', NOWHERE);
      return;
    }
    if (!this.editable() || event.button !== 0 || event.target !== event.currentTarget) return;

    this.begin(event, 'draw-zone', '', NOWHERE);
  }

  /**
   * ⚠️ Or the browser's own menu opens at the end of every selection, right where the
   * pointer was lifted. The application has no context menu of its own anywhere, so
   * nothing is being taken away.
   */
  protected onContextMenu(event: Event): void {
    event.preventDefault();
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
      case 'select-band': {
        const band = drawnTo(drag.origin, at);
        const swept = this.cardsUnder(band);
        if (swept.length > 0) {
          this.notesBanded.emit(swept);
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
    // ⚠️ The band is the one gesture that runs while nothing can be filed: it writes
    // nothing to the board, it only ticks what is already drawn.
    const banding = kind === 'select-band';
    if ((!this.editable() && !banding) || event.button !== (banding ? RIGHT_BUTTON : 0)) return;

    event.preventDefault();
    event.stopPropagation();
    // Optional-chained because jsdom lacks it and must not fail the drag.
    (event.target as HTMLElement).setPointerCapture?.(event.pointerId);

    if (banding) this.armMenuGuard();

    const origin = this.surfacePoint(event);
    this.gesture.set({ kind, id, origin, from, to: from, moved: false });
  }

  private disarmMenuGuard: () => void = () => undefined;

  /**
   * ⚠️ The menu opens on whatever is under the pointer when the button comes up, and a
   * sweep can end anywhere — over the header, off the board. So this one band swallows the
   * next menu wherever it lands, and the next press anywhere stands the guard down.
   */
  private armMenuGuard(): void {
    this.disarmMenuGuard();

    const swallow = (event: Event): void => {
      event.preventDefault();
      this.disarmMenuGuard();
    };
    const standDown = (): void => this.disarmMenuGuard();

    this.document.addEventListener('contextmenu', swallow, true);
    this.document.addEventListener('pointerdown', standDown, true);
    this.disarmMenuGuard = () => {
      this.document.removeEventListener('contextmenu', swallow, true);
      this.document.removeEventListener('pointerdown', standDown, true);
      this.disarmMenuGuard = () => undefined;
    };
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
      case 'select-band':
        return drawnTo(drag.origin, at);
    }
  }

  /**
   * Which cards a band swept over, measured off the screen.
   *
   * ⚠️ Measured and not computed: a card **filed into a zone flows** and has no
   * coordinates of its own, so there is nothing in the model to test a rectangle against.
   * The DOM is the only place a zone's cards have a position at all.
   */
  private cardsUnder(band: BoardFrame): readonly string[] {
    const surface = this.host.nativeElement.querySelector('.board-surface');
    const origin = surface?.getBoundingClientRect();
    if (!origin) return [];

    return Array.from(this.host.nativeElement.querySelectorAll<HTMLElement>('[data-note-id]'))
      .filter((element) => {
        const box = element.getBoundingClientRect();
        return overlaps(band, {
          x: Math.round(box.left - origin.left),
          y: Math.round(box.top - origin.top),
          width: Math.round(box.width),
          height: Math.round(box.height),
        });
      })
      .map((element) => element.dataset['noteId'] ?? '')
      .filter((id) => id !== '');
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
