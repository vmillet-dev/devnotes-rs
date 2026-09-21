import { describe, expect, it } from 'vitest';
import { BoardFrame } from '@core/model/board.model';
import {
  DRAG_THRESHOLD_PX,
  GRID_PX,
  Gesture,
  MIN_ZONE_HEIGHT,
  MIN_ZONE_WIDTH,
  asZone,
  cardPosition,
  contains,
  drawnTo,
  hasTravelled,
  isWorthDrawing,
  movedTo,
  overlaps,
  resizedTo,
  snap,
  zoneAt,
} from './board-gesture';

function gesture(overrides: Partial<Gesture> = {}): Gesture {
  return {
    kind: 'card',
    id: 'n-1',
    origin: { x: 100, y: 100 },
    from: { x: 90, y: 80, width: 300, height: 200 },
    to: { x: 90, y: 80, width: 300, height: 200 },
    moved: true,
    ...overrides,
  };
}

const ZONE: BoardFrame = { x: 0, y: 0, width: 100, height: 100 };

describe('board gesture geometry', () => {
  describe('telling a drag from a click', () => {
    it('ignores a pointer that barely wobbled', () => {
      expect(hasTravelled({ x: 10, y: 10 }, { x: 11, y: 12 })).toBe(false);
    });

    it('counts a pointer that travelled the threshold', () => {
      expect(hasTravelled({ x: 10, y: 10 }, { x: 10 + DRAG_THRESHOLD_PX, y: 10 })).toBe(true);
      expect(hasTravelled({ x: 10, y: 10 }, { x: 10, y: 10 - DRAG_THRESHOLD_PX })).toBe(true);
    });
  });

  /** Nothing ever lands between two dots: that is what makes a board look arranged. */
  describe('snapping to the grid', () => {
    it('rounds to the nearest dot, both ways', () => {
      expect(snap(0)).toBe(0);
      expect(snap(9)).toBe(0);
      expect(snap(11)).toBe(GRID_PX);
      expect(snap(-9)).toBe(-0);
      expect(snap(207)).toBe(200);
    });

    /** Two cards dropped roughly side by side come out exactly aligned. */
    it('brings two rough drops onto the same line', () => {
      const first = movedTo(gesture({ from: { x: 0, y: 0, width: 240, height: 150 } }), { x: 303, y: 298 });
      const second = movedTo(gesture({ from: { x: 0, y: 0, width: 240, height: 150 } }), { x: 297, y: 305 });

      expect(first.x).toBe(second.x);
      expect(first.y).toBe(second.y);
    });

    /** ⚠️ A gesture under the threshold never reaches this, so a click still moves nothing. */
    it('leaves a frame already on the grid exactly where it is', () => {
      const moved = movedTo(gesture({ from: { x: 100, y: 100, width: 240, height: 150 } }), {
        x: 100,
        y: 100,
      });

      expect(moved).toEqual({ x: 100, y: 100, width: 240, height: 150 });
    });
  });

  describe('moving', () => {
    /** ⚠️ The grab offset is the whole point: a frame follows the pointer, it does not
     *  jump so its corner sits under it. */
    it('keeps the offset the pointer was grabbed at', () => {
      const moved = movedTo(gesture(), { x: 300, y: 300 });

      // ⚠️ Snapped onto the lattice the background draws: the offset is kept, then
      // rounded — 290 lands on 300 and 280 was already on a dot.
      expect(moved.x).toBe(300);
      expect(moved.y).toBe(280);
    });

    it('carries the size along unchanged', () => {
      const moved = movedTo(gesture(), { x: 300, y: 300 });

      expect(moved.width).toBe(300);
      expect(moved.height).toBe(200);
    });

    it('never goes off the top or the left of the surface', () => {
      const moved = movedTo(gesture(), { x: -900, y: -900 });

      expect(moved.x).toBe(0);
      expect(moved.y).toBe(0);
    });
  });

  describe('resizing', () => {
    /** ⚠️ Resizing captures and releases nothing — Unreal's own rule was refused. */
    it('moves the far corner and leaves the near one where it is', () => {
      const resized = resizedTo(gesture({ kind: 'resize-zone' }), { x: 200, y: 150 });

      expect(resized.x).toBe(90);
      expect(resized.y).toBe(80);
      expect(resized.width).toBe(400);
      expect(resized.height).toBe(260);
    });

    /** A zone nothing can be dropped into is not a zone. */
    it('keeps room for one card however hard it is squashed', () => {
      const resized = resizedTo(gesture({ kind: 'resize-zone' }), { x: -900, y: -900 });

      expect(resized.width).toBe(MIN_ZONE_WIDTH);
      expect(resized.height).toBe(MIN_ZONE_HEIGHT);
    });
  });

  describe('drawing a zone', () => {
    it('reads the same whichever corner it was started from', () => {
      const forwards = drawnTo({ x: 10, y: 10 }, { x: 110, y: 90 });
      const backwards = drawnTo({ x: 110, y: 90 }, { x: 10, y: 10 });

      expect(forwards).toEqual(backwards);
      expect(forwards).toEqual({ x: 20, y: 20, width: 100, height: 80 });
    });

    /** A band barely dragged was a click on the background, and creates nothing. */
    it('refuses a band too small to have been meant', () => {
      expect(isWorthDrawing(drawnTo({ x: 10, y: 10 }, { x: 18, y: 18 }))).toBe(false);
      expect(isWorthDrawing(drawnTo({ x: 10, y: 10 }, { x: 200, y: 200 }))).toBe(true);
    });

    it('grows a small band up to something a card fits in', () => {
      const zone = asZone(drawnTo({ x: 10, y: 10 }, { x: 70, y: 70 }));

      expect(zone.width).toBe(MIN_ZONE_WIDTH);
      expect(zone.height).toBe(MIN_ZONE_HEIGHT);
      expect(zone.x).toBe(20);
    });

    it('leaves a band already big enough alone', () => {
      const zone = asZone({ x: 0, y: 0, width: 600, height: 400 });

      expect(zone.width).toBe(600);
      expect(zone.height).toBe(400);
    });
  });

  describe('where a drop lands', () => {
    it('names the zone the point falls in', () => {
      expect(zoneAt([{ id: 'a', frame: ZONE }], { x: 50, y: 50 })).toBe('a');
    });

    /** ⚠️ The free background is a legitimate answer: it takes the note out of its folder. */
    it('names nothing on the background', () => {
      expect(zoneAt([{ id: 'a', frame: ZONE }], { x: 150, y: 50 })).toBeNull();
    });

    it('treats the far edges as outside, so two zones cannot both claim a point', () => {
      expect(contains(ZONE, { x: 0, y: 0 })).toBe(true);
      expect(contains(ZONE, { x: 100, y: 50 })).toBe(false);
      expect(contains(ZONE, { x: 50, y: 100 })).toBe(false);
    });

    /** Drawn later means drawn on top, so it is what a drop lands in. */
    it('hands an overlap to the topmost zone', () => {
      const frames = [
        { id: 'under', frame: { x: 0, y: 0, width: 200, height: 200 } },
        { id: 'over', frame: { x: 50, y: 50, width: 100, height: 100 } },
      ];

      expect(zoneAt(frames, { x: 100, y: 100 })).toBe('over');
    });

    it('drops a card where the grab offset puts it, not under the cursor', () => {
      expect(cardPosition(gesture(), { x: 300, y: 300 })).toEqual({ x: 300, y: 280 });
    });
  });
});

describe('the selection band', () => {
  const band = { x: 100, y: 100, width: 200, height: 200 };

  it('takes a card it merely touches', () => {
    expect(overlaps(band, { x: 280, y: 280, width: 240, height: 150 })).toBe(true);
  });

  it('leaves a card it stops exactly short of', () => {
    expect(overlaps(band, { x: 300, y: 100, width: 240, height: 150 })).toBe(false);
  });

  it('takes a card it covers whole', () => {
    expect(overlaps(band, { x: 120, y: 120, width: 40, height: 40 })).toBe(true);
  });

  it('takes a card that covers it whole', () => {
    expect(overlaps(band, { x: 0, y: 0, width: 600, height: 600 })).toBe(true);
  });

  it('leaves a card beside it on one axis only', () => {
    expect(overlaps(band, { x: 150, y: 400, width: 240, height: 150 })).toBe(false);
  });
});
