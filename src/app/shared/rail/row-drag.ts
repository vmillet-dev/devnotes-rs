import { signal } from '@angular/core';

/** Past this, a press on a row is a drag and no longer a click. */
export const DRAG_THRESHOLD_PX = 4;

/** The row of a rail under a point, as its template marks it: `data-rail-row` and `data-id`. */
export function railRowAt(document: Document, x: number, y: number): HTMLElement | null {
  // Optional: absent from jsdom, and from nothing else this runs on.
  return document.elementFromPoint?.(x, y)?.closest<HTMLElement>('[data-rail-row]') ?? null;
}

/** How far down a row a point is, from 0 at its top to 1 at its bottom. */
export function shareOf(element: HTMLElement, y: number): number {
  const box = element.getBoundingClientRect();
  return (y - box.top) / Math.max(box.height, 1);
}

interface Press<Row> {
  readonly row: Row;
  readonly pointerId: number;
  readonly x: number;
  readonly y: number;
  moved: boolean;
}

/**
 * A row dragged in a rail: pointer events with capture, as every drag here — HTML5 drag and
 * drop does not reach this WebView. `targetAt` says where a drop at a point would land, `null`
 * where it cannot; what to do with it is the rail's.
 */
export class RowDrag<Row extends { readonly id: string }, Target> {
  private press: Press<Row> | null = null;
  private swallow = false;

  private readonly _dragged = signal<string | null>(null);
  private readonly _target = signal<Target | null>(null);

  /** The row being dragged, once the press has travelled. */
  readonly dragged = this._dragged.asReadonly();
  /** Where it would land, drawn on the rows. */
  readonly target = this._target.asReadonly();

  constructor(private readonly targetAt: (x: number, y: number, row: Row) => Target | null) {}

  start(event: PointerEvent, row: Row): void {
    if (event.button !== 0) return;
    this.swallow = false;
    this.press = { row, pointerId: event.pointerId, x: event.clientX, y: event.clientY, moved: false };
    (event.currentTarget as HTMLElement | null)?.setPointerCapture?.(event.pointerId);
  }

  move(event: PointerEvent): void {
    const press = this.press;
    if (press?.pointerId !== event.pointerId) return;
    if (!press.moved) {
      if (Math.hypot(event.clientX - press.x, event.clientY - press.y) < DRAG_THRESHOLD_PX) return;
      press.moved = true;
      this._dragged.set(press.row.id);
    }
    this._target.set(this.targetAt(event.clientX, event.clientY, press.row));
  }

  /** The row and where it was let go — `null` where nothing would take it — or `null` for a click. */
  end(event: PointerEvent): { readonly row: Row; readonly target: Target | null } | null {
    const press = this.press;
    if (press?.pointerId !== event.pointerId) return null;
    const target = this._target();
    this.cancel();
    if (!press.moved) return null;

    // ⚠️ The click a drag ends with comes in the same task; one that never comes must not
    // swallow the next real click, which a later press does not always precede.
    this.swallow = true;
    setTimeout(() => (this.swallow = false));
    return { row: press.row, target };
  }

  cancel(): void {
    this.press = null;
    this._target.set(null);
    this._dragged.set(null);
  }

  /** The click a drag ends with is not a click on the row it started from: answered once. */
  swallowsClick(): boolean {
    const swallowed = this.swallow;
    this.swallow = false;
    return swallowed;
  }
}
