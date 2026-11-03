import { describe, expect, it } from 'vitest';
import { MapNode } from '@core/model/note.model';
import { layoutMindMap } from './mind-map.layout';

function node(text: string, slide: number, children: MapNode[] = []): MapNode {
  return { text, slide, children };
}

describe('layoutMindMap', () => {
  it('puts the title alone in the middle when there are no headings', () => {
    const layout = layoutMindMap('Plan', []);

    expect(layout.nodes).toEqual([{ key: 'root', text: 'Plan', slide: null, depth: 0, left: 50, top: 50 }]);
    expect(layout.links).toEqual([]);
  });

  it('rings the headings around the title, the first at the top, each linked to its parent', () => {
    const layout = layoutMindMap('Plan', [
      node('Build', 0, [node('Rust', 0), node('Front', 0)]),
      node('Ship', 1),
    ]);
    const at = (text: string) => layout.nodes.find((each) => each.text === text)!;

    expect(layout.nodes.map((each) => [each.text, each.depth])).toEqual([
      ['Plan', 0],
      ['Build', 1],
      ['Rust', 2],
      ['Front', 2],
      ['Ship', 1],
    ]);
    expect(at('Plan')).toMatchObject({ left: 50, top: 50 });
    // « Build » holds two leaves of three: its sector runs from the top through the right.
    expect(at('Build').left).toBeGreaterThan(50);
    expect(at('Ship').left).toBeLessThan(50);
    expect(Math.max(...layout.nodes.map((each) => each.left))).toBeLessThanOrEqual(91);
    expect(Math.min(...layout.nodes.map((each) => each.top))).toBeGreaterThanOrEqual(9);
    expect(layout.links.map((link) => [link.from.text, link.to.text])).toEqual([
      ['Plan', 'Build'],
      ['Build', 'Rust'],
      ['Build', 'Front'],
      ['Plan', 'Ship'],
    ]);
  });

  it('puts a Note under one heading on that heading', () => {
    const layout = layoutMindMap('Plan', [node('Release', 0, [node('Build', 1), node('Ship', 2)])]);

    expect(layout.nodes[0]).toMatchObject({ text: 'Release', slide: 0, left: 50, top: 50 });
    expect(layout.nodes.map((each) => each.text)).toEqual(['Release', 'Build', 'Ship']);
  });
});
