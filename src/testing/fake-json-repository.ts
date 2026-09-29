import { JsonRepository } from '@core/data/json.repository';
import { JsonQuery, JsonView } from '@core/model/json.model';

/** Answers the view a spec sets: what the graph shows is Rust's to decide, not the fake's. */
export class FakeJsonRepository implements Pick<JsonRepository, keyof JsonRepository> {
  view: JsonView = emptyJsonView();
  readonly queries: JsonQuery[] = [];

  explore(query: JsonQuery): Promise<JsonView> {
    this.queries.push(query);
    return Promise.resolve(this.view);
  }
}

export function emptyJsonView(overrides: Partial<JsonView> = {}): JsonView {
  return {
    error: null,
    stats: { keys: 0, depth: 0, bytes: 0 },
    graph: { nodes: [], width: 0, height: 0, folded: false },
    tree: [],
    treeTruncated: false,
    matches: [],
    matchCount: 0,
    ...overrides,
  };
}
