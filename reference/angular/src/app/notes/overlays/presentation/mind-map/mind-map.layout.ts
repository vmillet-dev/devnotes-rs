import { MapNode } from '@core/model/note.model';

/** A heading placed on the map, in percent of the map's width and height. */
export interface PlacedNode {
  readonly key: string;
  readonly text: string;
  /** The body's slide it is on; `null` for the Note's title, which is the cover. */
  readonly slide: number | null;
  readonly depth: number;
  readonly left: number;
  readonly top: number;
}

export interface PlacedLink {
  readonly key: string;
  readonly from: PlacedNode;
  readonly to: PlacedNode;
}

export interface MindMapLayout {
  readonly nodes: readonly PlacedNode[];
  readonly links: readonly PlacedLink[];
}

/** How far from the edge the outermost headings sit, in percent. */
const MARGIN = 9;

interface Point {
  readonly key: string;
  readonly text: string;
  readonly slide: number | null;
  readonly depth: number;
  readonly x: number;
  readonly y: number;
}

/**
 * The headings around the Note's title, each ring a level deeper, each heading given a share
 * of its parent's sector by how many leaves it holds. A Note under one heading has that
 * heading in the middle. The rings are stretched to the screen, so it reads wide.
 */
export function layoutMindMap(title: string, map: readonly MapNode[]): MindMapLayout {
  const [only] = map;
  const root: Point =
    map.length === 1 && only
      ? { key: 'root', text: only.text, slide: only.slide, depth: 0, x: 0, y: 0 }
      : { key: 'root', text: title, slide: null, depth: 0, x: 0, y: 0 };
  const children = map.length === 1 && only ? only.children : map;

  const points: Point[] = [root];
  const pairs: [Point, Point][] = [];
  place(children, -Math.PI / 2, (3 * Math.PI) / 2, 1, root, points, pairs);

  const spanX = Math.max(...points.map((point) => Math.abs(point.x)));
  const spanY = Math.max(...points.map((point) => Math.abs(point.y)));
  const scale = (value: number, span: number): number =>
    span === 0 ? 50 : 50 + (value / span) * (50 - MARGIN);
  const at = ({ key, text, slide, depth, x, y }: Point): PlacedNode => ({
    key,
    text,
    slide,
    depth,
    left: scale(x, spanX),
    top: scale(y, spanY),
  });

  return {
    nodes: points.map(at),
    links: pairs.map(([from, to]) => ({ key: to.key, from: at(from), to: at(to) })),
  };
}

function place(
  nodes: readonly MapNode[],
  from: number,
  to: number,
  depth: number,
  parent: Point,
  points: Point[],
  pairs: [Point, Point][],
): void {
  const total = nodes.reduce((sum, node) => sum + leaves(node), 0);
  let angle = from;
  nodes.forEach((node, index) => {
    const span = ((to - from) * leaves(node)) / total;
    const middle = angle + span / 2;
    const point: Point = {
      key: `${parent.key}.${index}`,
      text: node.text,
      slide: node.slide,
      depth,
      x: Math.cos(middle) * depth,
      y: Math.sin(middle) * depth,
    };
    points.push(point);
    pairs.push([parent, point]);
    place(node.children, angle, angle + span, depth + 1, point, points, pairs);
    angle += span;
  });
}

function leaves(node: MapNode): number {
  return node.children.length === 0 ? 1 : node.children.reduce((sum, child) => sum + leaves(child), 0);
}
