import type { HttpItem, HttpMethod, RequestKind, RunData, RunResult, RunStep } from '@core/ipc/bindings';
import type { HttpNode, HttpTree } from './http.model';

export type { RunData, RunEntry, RunEvent, RunRequest, RunResult, RunStep } from '@core/ipc/bindings';

/** A request of the run's order, and whether it runs. */
export interface RunnerRow {
  readonly id: string;
  readonly name: string;
  /** The folders between it and what is run. */
  readonly path: readonly string[];
  readonly kind: RequestKind;
  readonly method: HttpMethod;
  readonly enabled: boolean;
}

/** The data file chosen: its name, and its rows as Rust read them. */
export interface RunnerData {
  readonly name: string;
  readonly data: RunData;
}

export type RunnerFilter = 'all' | 'passed' | 'failed';
export const RUNNER_FILTERS: readonly RunnerFilter[] = ['all', 'passed', 'failed'];

/** The requests under a collection or a folder, in the tree's order, all of them running. */
export function rowsUnder(tree: HttpTree, item: HttpItem): RunnerRow[] {
  const rows: RunnerRow[] = [];
  const walk = (nodes: readonly HttpNode[], path: readonly string[]): void => {
    for (const node of nodes) {
      if (node.kind === 'request') {
        const { request } = node;
        rows.push({
          id: request.id,
          name: request.name,
          path,
          kind: request.kind,
          method: request.method,
          enabled: true,
        });
      } else {
        walk(node.children, [...path, node.folder.name]);
      }
    }
  };
  const find = (nodes: readonly HttpNode[]): readonly HttpNode[] | null => {
    for (const node of nodes) {
      if (node.kind !== 'folder') continue;
      if (node.folder.id === item.id) return node.children;
      const deeper = find(node.children);
      if (deeper) return deeper;
    }
    return null;
  };
  for (const collection of tree.collections) {
    const children =
      item.kind === 'collection'
        ? collection.collection.id === item.id
          ? collection.children
          : null
        : find(collection.children);
    if (children) walk(children, []);
  }
  return rows;
}

/** A request that could change something, which a protected environment asks about. */
export function changes(row: Pick<RunnerRow, 'kind' | 'method'>): boolean {
  return row.kind === 'websocket' || !['GET', 'HEAD', 'OPTIONS'].includes(row.method);
}

export function failed(result: RunResult): boolean {
  return (
    !result.skipped &&
    (result.error !== null ||
      result.stop !== null ||
      result.tests.some((test) => !test.passed && !test.skipped))
  );
}

export interface RunSummary {
  readonly failures: number;
  readonly requests: number;
  readonly passed: number;
  readonly tests: number;
}

export function summaryOf(results: readonly RunResult[]): RunSummary {
  const ran = results.flatMap((result) => result.tests.filter((test) => !test.skipped));
  return {
    failures: results.filter(failed).length,
    requests: results.length,
    passed: ran.filter((test) => test.passed).length,
    tests: ran.length,
  };
}

/** The results of each iteration, in the order they came. */
export function byIteration(
  results: readonly RunResult[],
): readonly { readonly iteration: number; readonly results: readonly RunResult[] }[] {
  const groups = new Map<number, RunResult[]>();
  for (const result of results) {
    const group = groups.get(result.iteration) ?? [];
    group.push(result);
    groups.set(result.iteration, group);
  }
  return [...groups].map(([iteration, group]) => ({ iteration, results: group }));
}

/** A step said, folded into what the page holds. */
export function stepped(results: readonly RunResult[], step: RunStep): readonly RunResult[] {
  return step.kind === 'ended' ? [...results, step.result] : results;
}

/** The order moved: a row `from` one place `to` another. */
export function moved<T>(rows: readonly T[], from: number, to: number): readonly T[] {
  if (from === to || from < 0 || from >= rows.length) return rows;
  const next = [...rows];
  const [row] = next.splice(from, 1);
  next.splice(Math.max(0, Math.min(to, next.length)), 0, row as T);
  return next;
}
