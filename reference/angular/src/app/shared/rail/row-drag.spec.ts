import { describe, expect, it } from 'vitest';
import { RowDrag } from './row-drag';

const ROW = { id: 'perf' };

function press(type: string, x: number, y: number, init: PointerEventInit = {}): PointerEvent {
  return new PointerEvent(type, { button: 0, pointerId: 1, clientX: x, clientY: y, ...init });
}

/** A pointer's click counts the clicks; a key's, or a screen reader's, says 0. */
const POINTER_CLICK = new MouseEvent('click', { detail: 1 });
const KEY_CLICK = new MouseEvent('click', { detail: 0 });

function dragged(drag: RowDrag<typeof ROW, string>): void {
  drag.start(press('pointerdown', 0, 0), ROW);
  drag.move(press('pointermove', 30, 0));
  drag.end(press('pointerup', 30, 0));
}

describe('RowDrag', () => {
  it('starts nothing from another button, nor follows another pointer', () => {
    const drag = new RowDrag<typeof ROW, string>(() => 'target');

    drag.start(press('pointerdown', 0, 0, { button: 2 }), ROW);
    drag.move(press('pointermove', 50, 50));
    expect(drag.dragged()).toBeNull();

    drag.start(press('pointerdown', 0, 0), ROW);
    drag.move(press('pointermove', 50, 50, { pointerId: 2 }));
    expect(drag.end(press('pointerup', 50, 50, { pointerId: 2 }))).toBeNull();
    expect(drag.dragged()).toBeNull();
  });

  it('is a click until the press has travelled, and that click chooses', () => {
    const drag = new RowDrag<typeof ROW, string>(() => 'target');

    drag.start(press('pointerdown', 0, 0), ROW);
    drag.move(press('pointermove', 2, 2));

    expect(drag.end(press('pointerup', 2, 2))).toBeNull();
    expect(drag.endsADrag(POINTER_CLICK)).toBe(false);
  });

  it('answers the row and where it was let go, and the click after it is the drag’s', () => {
    const drag = new RowDrag<typeof ROW, string>((x) => (x > 20 ? 'far' : null));

    drag.start(press('pointerdown', 0, 0), ROW);
    drag.move(press('pointermove', 30, 0));
    expect(drag.dragged()).toBe('perf');
    expect(drag.target()).toBe('far');

    expect(drag.end(press('pointerup', 30, 0))).toEqual({ row: ROW, target: 'far' });
    expect(drag.target()).toBeNull();
    expect(drag.endsADrag(POINTER_CLICK)).toBe(true);
  });

  it('lets a key choose right after a drag, whenever its click comes', () => {
    const drag = new RowDrag<typeof ROW, string>(() => 'target');
    dragged(drag);

    expect(drag.endsADrag(KEY_CLICK)).toBe(false);
  });

  it('forgets the drag at the next press, whose click chooses', () => {
    const drag = new RowDrag<typeof ROW, string>(() => 'target');
    dragged(drag);

    drag.start(press('pointerdown', 0, 0), ROW);
    drag.end(press('pointerup', 0, 0));

    expect(drag.endsADrag(POINTER_CLICK)).toBe(false);
  });

  it('keeps the drag past a press of another button, which no click follows', () => {
    const drag = new RowDrag<typeof ROW, string>(() => 'target');
    dragged(drag);

    drag.start(press('pointerdown', 0, 0, { button: 2 }), ROW);

    expect(drag.endsADrag(POINTER_CLICK)).toBe(true);
  });
});
