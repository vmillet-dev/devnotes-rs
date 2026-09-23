import { Signal, computed, inject, linkedSignal, resource, untracked } from '@angular/core';
import { Injectable, signal } from '@angular/core';
import { LanguageTag } from '@core/model/language.model';
import { ClockService } from '@core/services/time/clock.service';
import { SEARCH_DEBOUNCE_MS, debounced } from '@core/services/time/debounce';
import { byCodeUnit } from '@core/utils/order.util';
import { NotesRepository } from '../data/notes.repository';
import { Note, NoteFilter, NoteSection, NotesQuery, NotesView } from '../model/note.model';
import { FoldersStore } from './folders.store';
import { NotesRevision } from './notes-revision';
import { SpacesStore } from './spaces.store';

export type { NoteFilter } from '../model/note.model';

export { SEARCH_DEBOUNCE_MS };

/** The local day: a new query only on a day change. */
function localDayKey(now: Date): string {
  return `${now.getFullYear()}-${now.getMonth()}-${now.getDate()}`;
}

interface QueryParams {
  readonly spaceId: string | null;
  readonly folderId: string | null;
  readonly search: string;
  readonly filter: NoteFilter;
  readonly tags: readonly string[];
  readonly languages: readonly LanguageTag[];
  readonly day: string;
  /** Bumped by whoever wrote notes from outside this store. */
  readonly revision: number;
}

function sameStrings(a: readonly string[], b: readonly string[]): boolean {
  return a.length === b.length && a.every((value, index) => value === b[index]);
}

/**
 * Exhaustive by construction, like `UNCHANGED` in `notes.store.ts`: a new
 * `QueryParams` field stops this compiling until it says how it compares.
 *
 * ⚠️ A hand-written chain of `&&` was the same table with a hole waiting to happen, and
 * the hole is silent: the forgotten field changes, the comparator answers "same", the
 * `resource` does not re-run, and the retained view keeps the interface looking right
 * while it ignores the filter.
 */
const SAME: {
  readonly [K in keyof QueryParams]: (a: QueryParams[K], b: QueryParams[K]) => boolean;
} = {
  spaceId: Object.is,
  folderId: Object.is,
  search: Object.is,
  filter: Object.is,
  day: Object.is,
  revision: Object.is,
  tags: sameStrings,
  languages: sameStrings,
};

/**
 * ⚠️ `resource` compares its params by identity: without this comparator, the fresh
 * literal `queryParams` builds on every clock tick fires a full query every 30 s.
 */
function sameQueryParams(a: QueryParams, b: QueryParams): boolean {
  return (Object.keys(SAME) as (keyof QueryParams)[]).every((key) => {
    const same = SAME[key] as (a: unknown, b: unknown) => boolean;
    return same(a[key], b[key]);
  });
}

function toggled<T>(selection: ReadonlySet<T>, value: T): ReadonlySet<T> {
  const next = new Set(selection);
  if (!next.delete(value)) {
    next.add(value);
  }
  return next;
}

/**
 * Which notes are shown. It filters, sorts and groups nothing — `query_notes` returns
 * a ready-to-render `NotesView`.
 */
@Injectable({ providedIn: 'root' })
export class NotesQueryStore {
  private readonly repository = inject(NotesRepository);
  private readonly clock = inject(ClockService);
  private readonly spaces = inject(SpacesStore);
  private readonly folders = inject(FoldersStore);
  private readonly revision = inject(NotesRevision);

  private readonly _searchQuery = signal('');
  private readonly _debouncedSearch = signal('');
  private readonly _activeFilter = signal<NoteFilter>('all');
  private readonly _selectedTags = signal<ReadonlySet<string>>(new Set());
  private readonly _selectedLanguages = signal<ReadonlySet<LanguageTag>>(new Set());

  /** Follows the typing without waiting: this is what the field shows. */
  readonly searchQuery = this._searchQuery.asReadonly();
  /** What actually crosses the bridge — the board queries on the same settled value. */
  readonly debouncedSearch = this._debouncedSearch.asReadonly();
  readonly activeFilter = this._activeFilter.asReadonly();
  readonly selectedTags = this._selectedTags.asReadonly();
  readonly selectedLanguages = this._selectedLanguages.asReadonly();

  private readonly commitSearch = debounced(
    (query: string) => this._debouncedSearch.set(query),
    SEARCH_DEBOUNCE_MS,
  );

  private readonly queryParams = computed<QueryParams>(
    () => ({
      spaceId: this.spaces.activeSpaceId(),
      folderId: this.folders.activeFolderId(),
      search: this._debouncedSearch().trim(),
      filter: this._activeFilter(),
      tags: [...this._selectedTags()].sort(byCodeUnit),
      languages: [...this._selectedLanguages()].sort(byCodeUnit),
      day: localDayKey(this.clock.now()),
      revision: this.revision.current(),
    }),
    { equal: sameQueryParams },
  );

  private readonly viewResource = resource({
    params: () => this.queryParams(),
    loader: ({ params }): Promise<NotesView> => {
      // Untracked: the current instant, without the query re-running on every tick.
      const now = untracked(() => this.clock.now());
      const query: NotesQuery = {
        spaceId: params.spaceId,
        folderId: params.folderId,
        search: params.search,
        filter: params.filter,
        tags: params.tags,
        languages: params.languages,
        now,
        tzOffsetMinutes: now.getTimezoneOffset(),
        pinnedFirst: true,
      };
      return this.repository.query(query);
    },
  });

  /**
   * Kept during reloads, or every keystroke would blank the canvas. ⚠️ A `linkedSignal`
   * only retains what it has seen go past, so everything this store exposes must read
   * `view()`, with no short-circuit (see `isLoading`).
   */
  private readonly view = linkedSignal<NotesView | undefined, NotesView | null>({
    source: () => (this.viewResource.hasValue() ? this.viewResource.value() : undefined),
    computation: (fresh, previous) => fresh ?? previous?.value ?? null,
  });

  readonly sections = computed<readonly NoteSection[]>(() => this.view()?.sections ?? []);
  readonly allTags = computed<readonly string[]>(() => this.view()?.availableTags ?? []);
  readonly allLanguages = computed<readonly LanguageTag[]>(() => this.view()?.availableLanguages ?? []);
  readonly isFiltering = computed(() => this.view()?.isFiltering ?? false);

  /** `null` when nothing is being filtered. */
  readonly matched = computed<number | null>(() => {
    const view = this.view();
    return view !== null && view.isFiltering ? view.matched : null;
  });

  readonly hasNoResults = computed(() => this.matched() === 0);

  /**
   * What `clearFilters` would give back. ⚠️ Not `isFiltering`, which is the view's own
   * answer and is true inside an opened folder — Escape would then clear a search that is
   * not there and never fall through to leaving the folder.
   */
  readonly hasUserFilters = computed(
    () => this._searchQuery() !== '' || this._selectedTags().size > 0 || this._selectedLanguages().size > 0,
  );

  /**
   * ⚠️ `view()` is read before the resource state: an `&&` the other way round would
   * short-circuit past the read, dropping the freshly loaded view.
   */
  readonly isLoading = computed(() => {
    const hasView = this.view() !== null;
    return !hasView && this.viewResource.isLoading();
  });

  readonly loadError: Signal<Error | undefined> = this.viewResource.error;

  /** Flat and in section order: what keyboard navigation follows and a range spans. */
  readonly visibleNotes = computed<readonly Note[]>(() =>
    this.sections().flatMap((section) => [...section.notes]),
  );

  reload(): void {
    this.viewResource.reload();
  }

  /** Updates the field at once, defers the query. */
  setSearchQuery(query: string): void {
    this._searchQuery.set(query);
    this.commitSearch(query);
  }

  setFilter(filter: NoteFilter): void {
    this._activeFilter.set(filter);
  }

  toggleTag(tag: string): void {
    this._selectedTags.update((tags) => toggled(tags, tag));
  }

  toggleLanguage(language: LanguageTag): void {
    this._selectedLanguages.update((languages) => toggled(languages, language));
  }

  /**
   * The three things `notes::view` counts as filtering. The quick filter is left alone:
   * it has a three-way control with "All" in it, which is already the way out.
   *
   * ⚠️ The debounce is cancelled first. A keystroke still on its way lands 150 ms later
   * and puts the query back, leaving the canvas filtered with an empty field.
   */
  clearFilters(): void {
    this.commitSearch.cancel();
    this._searchQuery.set('');
    this._debouncedSearch.set('');
    this._selectedTags.set(new Set());
    this._selectedLanguages.set(new Set());
  }

  findVisible(id: string): Note | null {
    for (const section of this.sections()) {
      const found = section.notes.find((note) => note.id === id);
      if (found) return found;
    }
    return null;
  }
}
