import type {
  JsonError,
  JsonGraph,
  JsonKind,
  JsonLine,
  JsonNode,
  JsonOpening,
  JsonQuery,
  JsonRow,
  JsonStats,
  JsonView,
  Span,
} from '@core/ipc/bindings';

/** No conversion: the wire shape carries no date and nothing the front reads otherwise. */
export type {
  JsonError,
  JsonGraph,
  JsonKind,
  JsonLine,
  JsonNode,
  JsonOpening,
  JsonQuery,
  JsonRow,
  JsonStats,
  JsonView,
  Span,
};

/** A row of the graph or a line of the tree, as far as opening it goes. */
export interface JsonEntry {
  readonly path: string;
  readonly opens: boolean;
  readonly open: boolean;
}

/** What a selection is: a node, or a value in one of its rows. */
export type JsonSelection =
  | { readonly kind: 'node'; readonly node: JsonNode }
  | { readonly kind: 'row'; readonly node: JsonNode; readonly row: JsonRow; readonly index: number };

export function findSelection(graph: JsonGraph, path: string | null): JsonSelection | null {
  if (path === null) return null;
  for (const node of graph.nodes) {
    if (node.path === path) return { kind: 'node', node };
  }
  for (const node of graph.nodes) {
    const index = node.rows.findIndex((row) => row.path === path);
    if (index >= 0) return { kind: 'row', node, row: node.rows[index]!, index };
  }
  return null;
}

/** The node and its ancestors, root first: the breadcrumb, and the path the graph lights. */
export function lineage(graph: JsonGraph, node: JsonNode): readonly JsonNode[] {
  const chain: JsonNode[] = [];
  let current: JsonNode | undefined = node;
  while (current) {
    chain.unshift(current);
    current = current.parent === null ? undefined : graph.nodes[current.parent];
  }
  return chain;
}

/** One level of the value, its containers folded: what the selection panel previews. */
export function preview(node: JsonNode): string {
  const object = node.kind === 'object';
  const entries = node.rows.map((row) => {
    const value = row.kind === 'object' ? '{ … }' : row.kind === 'array' ? '[ … ]' : row.value;
    return object ? `  ${JSON.stringify(row.key)}: ${value}` : `  ${value}`;
  });
  if (node.hiddenRows > 0) entries.push('  …');
  const [open, close] = object ? ['{', '}'] : ['[', ']'];
  return entries.length === 0 ? `${open}${close}` : `${open}\n${entries.join(',\n')}\n${close}`;
}
