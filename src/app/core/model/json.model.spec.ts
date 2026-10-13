import { describe, expect, it } from 'vitest';
import { JSON_NODES, collapsedJsonView, jsonView } from '@testing/json-view.fixture';
import {
  entryAt,
  explorableFormat,
  findSelection,
  lineage,
  preview,
  shows,
  spanOf,
  trailOf,
} from './json.model';

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

  it('traces an entry back to the root through the graph, a row included', () => {
    expect(trailOf(jsonView(), '$.data.lines[0].a')).toEqual([
      '$',
      '$.data',
      '$.data.lines',
      '$.data.lines[0]',
      '$.data.lines[0].a',
    ]);
    expect(trailOf(jsonView(), '$.data')).toEqual(['$', '$.data']);
  });

  it('traces it through the depths of the tree where the graph does not draw it', () => {
    const treeOnly = jsonView({ graph: { nodes: [], width: 0, height: 0, folded: true } });

    expect(trailOf(treeOnly, '$.data.lines[0].a')).toEqual([
      '$',
      '$.data',
      '$.data.lines',
      '$.data.lines[0]',
      '$.data.lines[0].a',
    ]);
    expect(trailOf(treeOnly, '$.gone')).toEqual(['$.gone']);
  });

  it('says whether a view draws an entry, as a node, a row or a line', () => {
    const folded = collapsedJsonView();

    expect(shows(folded, '$')).toBe(true);
    expect(shows(folded, '$.data')).toBe(true);
    expect(shows(folded, '$.data.lines')).toBe(false);
    expect(shows({ ...folded, graph: { ...folded.graph, nodes: [] } }, '$.id')).toBe(true);
  });

  it('finds where an entry was written, as a node, a row or a line', () => {
    expect(spanOf(jsonView(), '$.data')).toEqual({ start: 19, end: 38 });
    expect(spanOf(jsonView(), '$.id')).toEqual({ start: 6, end: 11 });
    expect(spanOf(jsonView({ graph: { nodes: [], width: 0, height: 0, folded: false } }), '$.data')).toEqual({
      start: 0,
      end: 0,
    });
    expect(spanOf(jsonView(), '$.gone')).toBeNull();
  });

  it('finds the innermost entry around an offset, from the spans alone', () => {
    expect(entryAt(jsonView(), 34)).toBe('$.data.lines[0].a');
    expect(entryAt(jsonView(), 30)).toBe('$.data.lines[0]');
    expect(entryAt(jsonView(), 8)).toBe('$.id');
    expect(
      entryAt(jsonView({ graph: { nodes: [], width: 0, height: 0, folded: false }, tree: [] }), 8),
    ).toBeNull();
  });

  it('knows which languages have a reader', () => {
    expect(explorableFormat('json')).toBe('json');
    expect(explorableFormat('yml')).toBe('yaml');
    expect(explorableFormat('toml')).toBe('toml');
    expect(explorableFormat('xml')).toBe('xml');
    expect(explorableFormat('html')).toBeNull();
    expect(explorableFormat('sh')).toBeNull();
  });

  it('previews one level, its containers folded', () => {
    expect(preview(JSON_NODES[0]!)).toBe('{\n  "id": "evt",\n  "data": { … }\n}');
    expect(preview(JSON_NODES[2]!)).toBe('[\n  { … }\n]');
    expect(preview({ ...JSON_NODES[0]!, rows: [], hiddenRows: 0 })).toBe('{}');
    expect(preview({ ...JSON_NODES[2]!, hiddenRows: 4 })).toBe('[\n  { … },\n  …\n]');
  });
});
