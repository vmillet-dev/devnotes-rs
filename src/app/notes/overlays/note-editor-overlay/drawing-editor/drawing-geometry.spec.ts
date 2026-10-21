import { describe, expect, it } from 'vitest';
import {
  arrowPath,
  boundEnds,
  diamondPoints,
  dragBox,
  edgePoint,
  pictureScale,
  sceneBox,
} from './drawing-geometry';

const BOX = { left: 0, top: 0, width: 100, height: 50 };

describe('drawing geometry', () => {
  it('draws an arrow as its shaft to the head, then the head closed', () => {
    expect(arrowPath([0, 0, 100, 0])).toBe('M 0 0 L 88 0 M 100 0 L 88 6 L 88 -6 Z');
  });

  it('keeps the head within an arrow shorter than it, and a dot an arrow', () => {
    expect(arrowPath([0, 0, 0, 4])).toBe('M 0 0 L 0 0 M 0 4 L -2 0 L 2 0 Z');
    expect(arrowPath([5, 5, 5, 5])).toBe('M 5 5 L 5 5 M 5 5 L 5 5 L 5 5 Z');
  });

  it('stops a line from the centre where it leaves the box', () => {
    expect(edgePoint(BOX, { x: 200, y: 25 })).toEqual({ x: 100, y: 25 });
    expect(edgePoint(BOX, { x: 50, y: -100 })).toEqual({ x: 50, y: 0 });
    expect(edgePoint(BOX, { x: 100, y: 75 })).toEqual({ x: 75, y: 50 });
    expect(edgePoint(BOX, { x: 150, y: 75 })).toEqual({ x: 100, y: 50 });
    expect(edgePoint(BOX, { x: 50, y: 25 })).toEqual({ x: 50, y: 25 });
  });

  it('puts each bound end on its shape, facing the other', () => {
    const right = { left: 200, top: 0, width: 100, height: 50 };

    expect(boundEnds([0, 0, 0, 0], BOX, right)).toEqual([100, 25, 200, 25]);
    expect(boundEnds([0, 0, 50, 300], BOX, null)).toEqual([50, 50, 50, 300]);
    expect(boundEnds([400, 25, 0, 0], null, right)).toEqual([400, 25, 300, 25]);
    expect(boundEnds([1, 2, 3, 4], null, null)).toEqual([1, 2, 3, 4]);
  });

  it('reads a drag whichever way it went', () => {
    expect(dragBox({ x: 50, y: 40 }, { x: 10, y: 60 })).toEqual({ left: 10, top: 40, width: 40, height: 20 });
  });

  it('puts a diamond’s corners at the middle of each side', () => {
    expect(diamondPoints(40, 20)).toEqual([
      { x: 20, y: 0 },
      { x: 40, y: 10 },
      { x: 20, y: 20 },
      { x: 0, y: 10 },
    ]);
  });

  it('takes a picture at the scale its longest side asks, never past the most', () => {
    expect(pictureScale({ left: 0, top: 0, width: 800, height: 200 }, 400, 2)).toBe(0.5);
    expect(pictureScale({ left: 0, top: 0, width: 100, height: 50 }, 1600, 2)).toBe(2);
  });

  it('frames everything drawn, with room around it, or nothing', () => {
    expect(sceneBox([BOX, { left: 150, top: -20, width: 10, height: 10 }], 10)).toEqual({
      left: -10,
      top: -30,
      width: 180,
      height: 90,
    });
    expect(sceneBox([])).toBeNull();
  });
});
