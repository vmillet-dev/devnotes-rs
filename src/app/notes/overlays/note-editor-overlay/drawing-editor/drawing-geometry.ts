/** What a shape's colour is called, so a theme changed afterwards draws it in its own. */
export const HUES = ['ink', 'blue', 'green', 'amber', 'red', 'purple'] as const;
export type Hue = (typeof HUES)[number];

export type ShapeTool = 'rectangle' | 'ellipse' | 'diamond';
export type Tool = 'select' | ShapeTool | 'arrow' | 'pen' | 'text';

export interface XY {
  readonly x: number;
  readonly y: number;
}

export interface Box {
  readonly left: number;
  readonly top: number;
  readonly width: number;
  readonly height: number;
}

/** An arrow's two ends, `[x1, y1, x2, y2]`, kept on it so it can be redrawn from them. */
export type Ends = readonly [number, number, number, number];

/** How long the head of an arrow is, along it. */
const HEAD = 12;

/** The shaft to the foot of the head, then the head as a closed triangle the fill paints. */
export function arrowPath([x1, y1, x2, y2]: Ends, head = HEAD): string {
  const length = Math.hypot(x2 - x1, y2 - y1) || 1;
  const ux = (x2 - x1) / length;
  const uy = (y2 - y1) / length;
  const reach = Math.min(head, length);
  const footX = x2 - ux * reach;
  const footY = y2 - uy * reach;
  const half = reach / 2;
  const leftX = footX - uy * half;
  const leftY = footY + ux * half;
  const rightX = footX + uy * half;
  const rightY = footY - ux * half;
  const at = (value: number): string => String(Math.round(value * 10) / 10);

  return [
    `M ${at(x1)} ${at(y1)} L ${at(footX)} ${at(footY)}`,
    `M ${at(x2)} ${at(y2)} L ${at(leftX)} ${at(leftY)} L ${at(rightX)} ${at(rightY)} Z`,
  ].join(' ');
}

export function centreOf(box: Box): XY {
  return { x: box.left + box.width / 2, y: box.top + box.height / 2 };
}

/** Where a line from the box's centre toward a point leaves the box: an arrow stops there. */
export function edgePoint(box: Box, toward: XY): XY {
  const centre = centreOf(box);
  const dx = toward.x - centre.x;
  const dy = toward.y - centre.y;
  if (dx === 0 && dy === 0) return centre;

  const halfWidth = box.width / 2;
  const halfHeight = box.height / 2;
  const scale = Math.min(
    dx === 0 ? Infinity : halfWidth / Math.abs(dx),
    dy === 0 ? Infinity : halfHeight / Math.abs(dy),
  );
  return { x: centre.x + dx * scale, y: centre.y + dy * scale };
}

/**
 * An arrow's ends once the shapes it starts or ends on have moved: each bound end on the edge
 * of its shape, facing the other end — the other shape's centre when both are bound.
 */
export function boundEnds(ends: Ends, from: Box | null, to: Box | null): Ends {
  const start: XY = from ? centreOf(from) : { x: ends[0], y: ends[1] };
  const end: XY = to ? centreOf(to) : { x: ends[2], y: ends[3] };
  const first = from ? edgePoint(from, end) : start;
  const last = to ? edgePoint(to, start) : end;
  return [first.x, first.y, last.x, last.y];
}

/** The box a drag from one corner to the other covers, whichever way it went. */
export function dragBox(from: XY, to: XY): Box {
  return {
    left: Math.min(from.x, to.x),
    top: Math.min(from.y, to.y),
    width: Math.abs(to.x - from.x),
    height: Math.abs(to.y - from.y),
  };
}

export function diamondPoints(width: number, height: number): XY[] {
  return [
    { x: width / 2, y: 0 },
    { x: width, y: height / 2 },
    { x: width / 2, y: height },
    { x: 0, y: height / 2 },
  ];
}

/** The scale a picture is taken at: its longest side brought to `longest`, never enlarged past `most`. */
export function pictureScale(box: Box, longest: number, most: number): number {
  const side = Math.max(box.width, box.height, 1);
  return Math.min(most, longest / side);
}

/** Room around what is drawn, in a picture taken of it. */
export const MARGIN = 24;

/** The box around every box, with room around it; `null` when nothing is drawn. */
export function sceneBox(boxes: readonly Box[], margin = MARGIN): Box | null {
  if (boxes.length === 0) return null;
  const left = Math.min(...boxes.map((box) => box.left)) - margin;
  const top = Math.min(...boxes.map((box) => box.top)) - margin;
  const right = Math.max(...boxes.map((box) => box.left + box.width)) + margin;
  const bottom = Math.max(...boxes.map((box) => box.top + box.height)) + margin;
  return { left, top, width: right - left, height: bottom - top };
}
