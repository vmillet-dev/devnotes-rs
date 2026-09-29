import { JsonKind, JsonLine, JsonNode, JsonRow, JsonView } from '@core/model/json.model';
import { emptyJsonView } from './fake-json-repository';

/** `{"id":"evt","data":{"lines":[{"a":1}]}}`, open whole, as Rust would answer it. */
export const JSON_TEXT = '{"id":"evt","data":{"lines":[{"a":1}]}}';

function row(
  key: string,
  path: string,
  kind: JsonKind,
  value: string,
  start: number,
  end: number,
  child: number | null,
): JsonRow {
  return {
    key,
    path,
    kind,
    value,
    span: { start, end },
    child,
    opens: kind === 'object' || kind === 'array',
    hit: false,
  };
}

function node(
  id: number,
  parent: number | null,
  path: string,
  label: string,
  kind: JsonKind,
  rows: JsonRow[],
  x: number,
  y: number,
  start: number,
  end: number,
): JsonNode {
  return {
    id,
    parent,
    path,
    label,
    kind,
    size: rows.length,
    rows,
    hiddenRows: 0,
    span: { start, end },
    x,
    y,
    width: 16,
    height: 1 + rows.length,
    hit: false,
  };
}

export const JSON_NODES: readonly JsonNode[] = [
  node(
    0,
    null,
    '$',
    '$',
    'object',
    [row('id', '$.id', 'string', '"evt"', 6, 11, null), row('data', '$.data', 'object', '{ … }', 19, 38, 1)],
    0,
    0,
    0,
    39,
  ),
  node(
    1,
    0,
    '$.data',
    'data',
    'object',
    [row('lines', '$.data.lines', 'array', '[ 1 ]', 28, 37, 2)],
    22,
    3,
    19,
    38,
  ),
  node(
    2,
    1,
    '$.data.lines',
    'lines',
    'array',
    [row('[0]', '$.data.lines[0]', 'object', '{ … }', 29, 36, 3)],
    44,
    4,
    28,
    37,
  ),
  node(
    3,
    2,
    '$.data.lines[0]',
    '[0]',
    'object',
    [row('a', '$.data.lines[0].a', 'number', '1', 34, 35, null)],
    66,
    5,
    29,
    36,
  ),
];

function line(
  depth: number,
  key: string,
  path: string,
  kind: JsonKind,
  value: string,
  size: number,
  open: boolean,
): JsonLine {
  const opens = kind === 'object' || kind === 'array';
  return {
    depth,
    key,
    path,
    kind,
    value,
    size,
    span: { start: 0, end: 0 },
    opens,
    open: opens && open,
    hit: false,
  };
}

export const JSON_LINES: readonly JsonLine[] = [
  line(0, '$', '$', 'object', '{ … }', 2, true),
  line(1, 'id', '$.id', 'string', '"evt"', 0, false),
  line(1, 'data', '$.data', 'object', '{ … }', 1, true),
  line(2, 'lines', '$.data.lines', 'array', '[ 1 ]', 1, true),
  line(3, '[0]', '$.data.lines[0]', 'object', '{ … }', 1, true),
  line(4, 'a', '$.data.lines[0].a', 'number', '1', 0, false),
];

export function jsonView(overrides: Partial<JsonView> = {}): JsonView {
  return emptyJsonView({
    stats: { keys: 3, depth: 3, bytes: JSON_TEXT.length },
    graph: { nodes: [...JSON_NODES], width: 82, height: 7, folded: false },
    tree: [...JSON_LINES],
    ...overrides,
  });
}
