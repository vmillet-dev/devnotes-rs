import { Injectable, Signal, computed, inject, resource, signal, untracked } from '@angular/core';
import { LanguageTag } from '@core/model/language.model';
import { ClockService } from '@core/services/time/clock.service';
import { SEARCH_DEBOUNCE_MS, debounced } from '@core/services/time/debounce';
import { sameArray, sameBy } from '@core/utils/equality.util';
import { OneInFlight } from '@core/utils/one-in-flight.util';
import { byCodeUnit } from '@core/utils/order.util';
import { retained } from '@core/utils/retained.util';
import { NotesRepository } from '@core/data/notes.repository';
import { Note, NoteFilter, NoteSection, NotesQuery, NotesView } from '@core/model/note.model';
import { FoldersStore } from './folders.store';
import { NotesRevision } from './notes-revision';
import { SpacesStore } from './spaces.store';

export type { NoteFilter } from '@core/model/note.model';

export { SEARCH_DEBOUNCE_MS };

/** The local day: a new query only on a day change. */
function localDayKey(now: Date): string {
  return `${now.getFullYear()}-${now.getMonth()}-${now.getDate()}`;
}

/** What the search, the quick filter and the two rails ask for — the canvas and the board alike. */
export interface Criteria {
  readonly search: string;
  readonly filter: NoteFilter;
  /** Sorted, so a selection compares equal however it was ticked. */
  readonly tags: readonly string[];
  readonly languages: readonly LanguageTag[];
}

const sameCriteria = sameBy<Criteria>({
  search: Object.is,
  filter: Object.is,
  tags: sameArray,
  languages: sameArray,
});

interface QueryParams {
  readonly criteria: Criteria;
  readonly spaceId: string | null;
  readonly folderId: string | null;
  readonly day: string;
  /** Bumped by whoever wrote notes from outside this store. */
  readonly revision: number;
}

/**
 * ⚠️ `resource` compares its params by identity: without this, the literal rebuilt on every
 * clock tick fires a query every 30 s. `criteria` compares by identity, its own `computed`
 * keeping the object while it compares equal.
 */
const sameQueryParams = sameBy<QueryParams>({
  criteria: Object.is,
  spaceId: Object.is,
  folderId: Object.is,
  day: Object.is,
  revision: Object.is,
});

function toggled<T>(selection: ReadonlySet<T>, value: T): ReadonlySet<T> {
  const next = new Set(selection);
  if (!next.delete(value)) {
    next.add(value);
  }
  return next;
}

/** Which notes are shown. It filters, sorts and groups nothing: `query_notes` does. */
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
  readonly activeFilter = this._activeFilter.asReadonly();
  readonly selectedTags = this._selectedTags.asReadonly();
  readonly selectedLanguages = this._selectedLanguages.asReadonly();

  private readonly commitSearch = debounced(
    (query: string) => this._debouncedSearch.set(query),
    SEARCH_DEBOUNCE_MS,
  );

  /** The settled search, not the field: the board queries on the same criteria. */
  readonly criteria = computed<Criteria>(
    () => ({
      search: this._debouncedSearch().trim(),
      filter: this._activeFilter(),
      tags: [...this._selectedTags()].sort(byCodeUnit),
      languages: [...this._selectedLanguages()].sort(byCodeUnit),
    }),
    { equal: sameCriteria },
  );

  private readonly queryParams = computed<QueryParams>(
    () => ({
      criteria: this.criteria(),
      spaceId: this.spaces.activeSpaceId(),
      folderId: this.folders.activeFolderId(),
      day: localDayKey(this.clock.now()),
      revision: this.revision.current(),
    }),
    { equal: sameQueryParams },
  );

  private readonly queries = new OneInFlight();

  private readonly viewResource = resource({
    params: () => this.queryParams(),
    loader: ({ params, abortSignal }): Promise<NotesView> =>
      this.queries.run(abortSignal, () => {
        // Untracked: the current instant, without the query re-running on every tick.
        const now = untracked(() => this.clock.now());
        const query: NotesQuery = {
          ...params.criteria,
          spaceId: params.spaceId,
          folderId: params.folderId,
          now,
          tzOffsetMinutes: now.getTimezoneOffset(),
          pinnedFirst: true,
        };
        return this.repository.query(query);
      }),
  });

  private readonly view = retained(this.viewResource);

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
   * What `clearFilters` would give back. ⚠️ Not `isFiltering`, which is true inside an opened
   * folder: Escape would clear a search that is not there and never leave the folder.
   */
  readonly hasUserFilters = computed(
    () => this._searchQuery() !== '' || this._selectedTags().size > 0 || this._selectedLanguages().size > 0,
  );

  /** ⚠️ `view()` first: an `&&` the other way round skips the read and drops the loaded view. */
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
   * The three things `notes::view` counts as filtering; the quick filter keeps its "All". The
   * debounce is cancelled first, or a keystroke on its way puts the query back.
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
