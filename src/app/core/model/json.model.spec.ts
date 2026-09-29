import { describe, expect, it } from 'vitest';
import { JSON_NODES, jsonView } from '@testing/json-view.fixture';
import { findSelection, lineage, preview } from './json.model';

describe('json model', () => {
  const graph = jsonView().graph;

  it('finds a node by its path before a row that names the same value', () => {
    expect(findSelection(graph, '$.data')).toEqual({ kind: 'node', node: JSON_NODES[1] });
  });

  it('finds a value in the rows of its node', () => {
    expect(findSelection(graph, '$.data.lines[0].a')).toMatchObject({
      kind: 'row',
      index: 0,
      node: JSON_NODES[3],
    });
  });

  it('finds nothing for no path, or a path the graph does not draw', () => {
    expect(findSelection(graph, null)).toBeNull();
    expect(findSelection(graph, '$.gone')).toBeNull();
  });

  it('walks from the root down to a node', () => {
    expect(lineage(graph, JSON_NODES[3]!).map((node) => node.label)).toEqual(['$', 'data', 'lines', '[0]']);
  });

  it('previews one level, its containers folded', () => {
    expect(preview(JSON_NODES[0]!)).toBe('{\n  "id": "evt",\n  "data": { … }\n}');
    expect(preview(JSON_NODES[2]!)).toBe('[\n  { … }\n]');
    expect(preview({ ...JSON_NODES[0]!, rows: [], hiddenRows: 0 })).toBe('{}');
    expect(preview({ ...JSON_NODES[2]!, hiddenRows: 4 })).toBe('[\n  { … },\n  …\n]');
  });
});
