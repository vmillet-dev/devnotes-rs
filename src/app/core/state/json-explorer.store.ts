import { Injectable, computed, inject, resource, signal } from '@angular/core';
import { JsonRepository } from '@core/data/json.repository';
import { JsonEntry, JsonOpening, JsonQuery, JsonView } from '@core/model/json.model';
import { SEARCH_DEBOUNCE_MS, debounced } from '@core/services/time/debounce';
import { retained } from '@core/utils/retained.util';

/** A draft typed into is explored once the typing pauses: the document is parsed whole. */
const TEXT_DEBOUNCE_MS = 250;

function sameOpening(a: JsonOpening, b: JsonOpening): boolean {
  return JSON.stringify(a) === JSON.stringify(b);
}

function sameQuery(a: JsonQuery | undefined, b: JsonQuery | undefined): boolean {
  return (
    a === b || (!!a && !!b && a.text === b.text && a.search === b.search && sameOpening(a.opening, b.opening))
  );
}

/**
 * One explorer's state: provided by the component that shows a document — the editor today, an
 * HTTP response later — and not at the root, so two can be open at once. Which containers are
 * open is read back from each answer: Rust decides what an opening opens, caps included.
 */
@Injectable()
export class JsonExplorerStore {
  private readonly repository = inject(JsonRepository);

  private readonly _text = signal<string | null>(null);
  private readonly _search = signal('');
  private readonly _debouncedSearch = signal('');
  private readonly _opening = signal<JsonOpening>({ kind: 'initial' }, { equal: sameOpening });
  private readonly _selectedPath = signal<string | null>(null);
  private readonly _matchIndex = signal(-1);

  private readonly commitText = debounced((text: string) => this._text.set(text), TEXT_DEBOUNCE_MS);
  private readonly commitSearch = debounced((query: string) => {
    this._debouncedSearch.set(query);
    this._matchIndex.set(-1);
  }, SEARCH_DEBOUNCE_MS);

  private readonly query = computed<JsonQuery | undefined>(
    () => {
      const text = this._text();
      if (text === null) return undefined;
      return { text, search: this._debouncedSearch().trim(), opening: this._opening() };
    },
    { equal: sameQuery },
  );

  private readonly viewResource = resource({
    params: () => this.query(),
    loader: ({ params }) => this.repository.explore(params),
  });

  readonly view = retained<JsonView>(this.viewResource);
  readonly search = this._search.asReadonly();
  readonly selectedPath = this._selectedPath.asReadonly();
  /** What the view was asked of, for the spans it answers into. */
  readonly text = this._text.asReadonly();
  /** Zero-based among the matches, or -1 before the first step. */
  readonly matchIndex = this._matchIndex.asReadonly();

  private readonly openPaths = computed(() => this.view()?.graph.nodes.map((node) => node.path) ?? []);

  /** At once: a document just opened is explored whole, from its first level. */
  load(text: string): void {
    this.commitText.cancel();
    this.commitSearch.cancel();
    this._text.set(text);
    this._search.set('');
    this._debouncedSearch.set('');
    this._opening.set({ kind: 'initial' });
    this._selectedPath.set(null);
    this._matchIndex.set(-1);
  }

  /** The draft typed into: what is open stays open, as far as the new text still has it. */
  setText(text: string): void {
    if (this._text() === null) {
      this.load(text);
      return;
    }
    this.commitText(text);
  }

  clear(): void {
    this.commitText.cancel();
    this._text.set(null);
  }

  setSearch(query: string): void {
    this._search.set(query);
    this.commitSearch(query);
  }

  select(path: string): void {
    this._selectedPath.set(path);
  }

  /** A container opens, or closes; a value is selected. Either way the entry is the selection. */
  toggle(entry: JsonEntry): void {
    this._selectedPath.set(entry.path);
    if (!entry.opens) return;

    const open = this.openPaths();
    this._opening.set({
      kind: 'paths',
      paths: entry.open ? open.filter((path) => path !== entry.path) : [...open, entry.path],
    });
  }

  openAll(): void {
    this._opening.set({ kind: 'all' });
  }

  /** To the next match or the previous one, opening what it sits in. */
  step(direction: 1 | -1): void {
    const matches = this.view()?.matches ?? [];
    if (matches.length === 0) return;

    const current = this._matchIndex();
    const index =
      current < 0
        ? direction === 1
          ? 0
          : matches.length - 1
        : (current + direction + matches.length) % matches.length;
    const path = matches[index]!;
    this._matchIndex.set(index);
    this._selectedPath.set(path);
    this._opening.set({ kind: 'paths', paths: this.openPaths(), reveal: path });
  }
}
