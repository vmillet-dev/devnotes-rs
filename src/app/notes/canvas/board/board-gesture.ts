import { BoardFrame, BoardPoint } from '@core/model/board.model';

/**
 * The geometry a pointer drag needs, as plain functions: no signals, no DOM. ⚠️ HTML5 drag and
 * drop never reaches this WebView — `dragDropEnabled` stays `true` for the native file drop — so
 * every gesture is pointer events.
 */

/** Below this, a pointer that moved is a click that wobbled. */
export const DRAG_THRESHOLD_PX = 4;

/**
 * The lattice everything a gesture writes lands on: the surface's dotted background is 20px
 * (`board.component.scss`), keep the two in step. Nothing snaps before the pointer has passed
 * `DRAG_THRESHOLD_PX`, so a click never becomes a move.
 */
export const GRID_PX = 20;

export function snap(value: number): number {
  return Math.round(value / GRID_PX) * GRID_PX;
}

/** Mirrors `folders::board`, which clamps again before writing. */
export const MIN_ZONE_WIDTH = 264;
export const MIN_ZONE_HEIGHT = 212;

/** A zone drawn smaller than this was a click on the background, not a gesture. */
export const MIN_DRAWN_ZONE = 40;

export type GestureKind = 'card' | 'move-zone' | 'resize-zone' | 'draw-zone' | 'select-band';

export interface Gesture {
  readonly kind: GestureKind;
  /** The folder or note being moved; empty while a new zone is being drawn. */
  readonly id: string;
  /** Where the pointer went down, in surface coordinates. */
  readonly origin: BoardPoint;
  /** Where the thing being moved started, so a drag keeps its grab offset. */
  readonly from: BoardFrame;
  readonly to: BoardFrame;
  /** Stays false until the pointer has actually travelled: a click must not persist. */
  readonly moved: boolean;
}

/**
 * The card's own controls, which a drag must not start on: the press is aimed at them, and
 * a gesture beginning there would have to swallow the click they are waiting for.
 */
const CARD_CONTROLS = '.card-check, .card-fill, .card-item, app-copy-button, app-note-card-menu';

export function isCardControl(target: EventTarget | null): boolean {
  return target instanceof Element && target.closest(CARD_CONTROLS) !== null;
}

export function hasTravelled(origin: BoardPoint, at: BoardPoint): boolean {
  return Math.abs(at.x - origin.x) >= DRAG_THRESHOLD_PX || Math.abs(at.y - origin.y) >= DRAG_THRESHOLD_PX;
}

/** Keeps the grab offset: a frame follows the pointer, it does not jump under it. */
export function movedTo(gesture: Gesture, at: BoardPoint): BoardFrame {
  return {
    x: Math.max(0, snap(gesture.from.x + (at.x - gesture.origin.x))),
    y: Math.max(0, snap(gesture.from.y + (at.y - gesture.origin.y))),
    width: gesture.from.width,
    height: gesture.from.height,
  };
}

/** Resizing captures and releases nothing: membership comes from the drop alone. */
export function resizedTo(gesture: Gesture, at: BoardPoint): BoardFrame {
  return {
    x: gesture.from.x,
    y: gesture.from.y,
    // Snapped then clamped: the minimum is `folders::board`'s and not a multiple of the grid.
    width: Math.max(MIN_ZONE_WIDTH, snap(gesture.from.width + (at.x - gesture.origin.x))),
    height: Math.max(MIN_ZONE_HEIGHT, snap(gesture.from.height + (at.y - gesture.origin.y))),
  };
}

/**
 * The rubber band, the same whichever corner it started from. Both corners snap, not the
 * size, which would leave the far edge between two dots.
 */
export function drawnTo(origin: BoardPoint, at: BoardPoint): BoardFrame {
  const left = Math.max(0, snap(Math.min(origin.x, at.x)));
  const top = Math.max(0, snap(Math.min(origin.y, at.y)));

  return {
    x: left,
    y: top,
    width: Math.max(0, snap(Math.max(origin.x, at.x)) - left),
    height: Math.max(0, snap(Math.max(origin.y, at.y)) - top),
  };
}

/** A band barely dragged was a click on the background, and creates nothing. */
export function isWorthDrawing(frame: BoardFrame): boolean {
  return frame.width >= MIN_DRAWN_ZONE && frame.height >= MIN_DRAWN_ZONE;
}

/**
 * Whether a band and a card box overlap at all: touching is enough, as with any rubber band.
 * Half-open on the far edges, like `contains`.
 */
export function overlaps(band: BoardFrame, card: BoardFrame): boolean {
  return (
    band.x < card.x + card.width &&
    card.x < band.x + band.width &&
    band.y < card.y + card.height &&
    card.y < band.y + band.height
  );
}

/** What a drawn band becomes once it is a zone: never too small to drop a card into. */
export function asZone(frame: BoardFrame): BoardFrame {
  return {
    x: frame.x,
    y: frame.y,
    width: Math.max(MIN_ZONE_WIDTH, frame.width),
    height: Math.max(MIN_ZONE_HEIGHT, frame.height),
  };
}

export function contains(frame: BoardFrame, point: BoardPoint): boolean {
  return (
    point.x >= frame.x &&
    point.x < frame.x + frame.width &&
    point.y >= frame.y &&
    point.y < frame.y + frame.height
  );
}

/** Which zone a drop lands in, topmost first; `null` is the background, where a drop unfiles. */
export function zoneAt(
  frames: readonly { readonly id: string; readonly frame: BoardFrame }[],
  point: BoardPoint,
): string | null {
  for (let index = frames.length - 1; index >= 0; index -= 1) {
    const candidate = frames[index];
    if (candidate && contains(candidate.frame, point)) return candidate.id;
  }
  return null;
}

/** Where a card dropped on the background sits: under the pointer, by its grab offset. */
export function cardPosition(gesture: Gesture, at: BoardPoint): BoardPoint {
  const frame = movedTo(gesture, at);
  return { x: frame.x, y: frame.y };
}
