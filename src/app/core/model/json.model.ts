import type {
  JsonError,
  JsonGraph,
  JsonKind,
  JsonLine,
  JsonNode,
  JsonNotice,
  JsonOpening,
  JsonQuery,
  JsonRow,
  JsonStats,
  JsonView,
  Span,
  StructureFormat,
} from '@core/ipc/bindings';
import type { LanguageTag } from '@core/model/language.model';

/** No conversion: the wire shape carries no date and nothing the front reads otherwise. */
export type {
  JsonError,
  JsonGraph,
  JsonKind,
  JsonLine,
  JsonNode,
  JsonNotice,
  JsonOpening,
  JsonQuery,
  JsonRow,
  JsonStats,
  JsonView,
  Span,
  StructureFormat,
};

/** The snippets whose language Rust has a reader for: one line per format. */
const EXPLORABLE: Partial<Record<LanguageTag, StructureFormat>> = {
  json: 'json',
  yml: 'yaml',
  toml: 'toml',
  // HTML is not XML: it gets none.
  xml: 'xml',
};

/** How a format names itself in a sentence: « YAML valide ». */
export const FORMAT_NAMES: Record<StructureFormat, string> = {
  json: 'JSON',
  yaml: 'YAML',
  toml: 'TOML',
  xml: 'XML',
};

export function explorableFormat(language: LanguageTag): StructureFormat | null {
  return EXPLORABLE[language] ?? null;
}

/**
 * How far a node was moved from where Rust placed it, in the layout's own units: columns
 * across, lines down. The front's alone, kept by path and never written to the note.
 */
export interface NodeOffset {
  readonly x: number;
  readonly y: number;
}

export interface NodeMove {
  readonly path: string;
  readonly offset: NodeOffset;
}

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

/** Whether the view draws the entry: as a node or a row of the graph, or a line of the tree. */
export function shows(view: JsonView, path: string): boolean {
  return findSelection(view.graph, path) !== null || view.tree.some((line) => line.path === path);
}

/**
 * The entry and what holds it, outermost first, read from the view's own structure — the
 * graph's parents, or the tree's depths — so that no path is ever taken apart here.
 */
export function trailOf(view: JsonView, path: string): readonly string[] {
  const found = findSelection(view.graph, path);
  if (found) {
    const chain = lineage(view.graph, found.node).map((node) => node.path);
    return found.kind === 'row' ? [...chain, found.row.path] : chain;
  }
  const at = view.tree.findIndex((line) => line.path === path);
  if (at < 0) return [path];
  const trail = [path];
  let depth = view.tree[at]!.depth;
  for (let index = at - 1; index >= 0 && depth > 0; index--) {
    const line = view.tree[index]!;
    if (line.depth < depth) {
      trail.unshift(line.path);
      depth = line.depth;
    }
  }
  return trail;
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
