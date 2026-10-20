import { describe, expect, it } from 'vitest';
import { RowDrag } from './row-drag';

const ROW = { id: 'perf' };

function press(type: string, x: number, y: number, init: PointerEventInit = {}): PointerEvent {
  return new PointerEvent(type, { button: 0, pointerId: 1, clientX: x, clientY: y, ...init });
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

  it('is a click until the press has travelled, and swallows nothing then', () => {
    const drag = new RowDrag<typeof ROW, string>(() => 'target');

    drag.start(press('pointerdown', 0, 0), ROW);
    drag.move(press('pointermove', 2, 2));

    expect(drag.end(press('pointerup', 2, 2))).toBeNull();
    expect(drag.swallowsClick()).toBe(false);
  });

  it('answers the row and where it was let go, and swallows the click once', () => {
    const drag = new RowDrag<typeof ROW, string>((x) => (x > 20 ? 'far' : null));

    drag.start(press('pointerdown', 0, 0), ROW);
    drag.move(press('pointermove', 30, 0));
    expect(drag.dragged()).toBe('perf');
    expect(drag.target()).toBe('far');

    expect(drag.end(press('pointerup', 30, 0))).toEqual({ row: ROW, target: 'far' });
    expect(drag.target()).toBeNull();
    expect(drag.swallowsClick()).toBe(true);
    expect(drag.swallowsClick()).toBe(false);
  });
});

describe('RowDrag, once the drag is over', () => {
  it('lets the next click through when the one a drag ends with never came', async () => {
    const drag = new RowDrag<{ id: string }, string>(() => 'target');
    drag.start(new PointerEvent('pointerdown', { button: 0, pointerId: 1 }), { id: 'perf' });
    drag.move(new PointerEvent('pointermove', { pointerId: 1, clientX: 30 }));
    drag.end(new PointerEvent('pointerup', { pointerId: 1, clientX: 30 }));

    await new Promise((resolve) => setTimeout(resolve));

    expect(drag.swallowsClick()).toBe(false);
  });
});
