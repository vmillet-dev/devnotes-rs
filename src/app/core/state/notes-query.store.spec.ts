import { WritableSignal, signal } from '@angular/core';
import { TestBed } from '@angular/core/testing';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { NotesView } from '@core/model/note.model';
import { ClockService } from '@core/services/time/clock.service';
import { FakeClipboard } from '@testing/fake-clipboard';
import { FakeFoldersRepository } from '@testing/fake-folders-repository';
import { FakeNotesRepository } from '@testing/fake-notes-repository';
import { createNote } from '@testing/note.fixture';
import { provideAppTesting } from '@testing/testing.providers';
import {
  HARNESS_SPACES,
  NotesHarness,
  awaitQuery,
  createNotesHarness,
  visibleIds,
} from '@testing/notes-harness';
import { NoteSelectionStore } from './note-selection.store';
import { NotesQueryStore, SEARCH_DEBOUNCE_MS } from './notes-query.store';
import { NoteBatchStore } from './note-batch.store';
import { NotesStore } from './notes.store';
import { UndoStore } from './undo.store';
import { SpacesStore } from './spaces.store';

describe('NotesQueryStore', () => {
  beforeEach(() => {
    TestBed.resetTestingModule();
    vi.restoreAllMocks();
    // The stores report failures through console.error on purpose.
    vi.spyOn(console, 'error').mockImplementation(() => undefined);
  });

  it('shows the view the backend returned', async () => {
    const { canvas } = await createNotesHarness([createNote({ id: 'a' }), createNote({ id: 'b' })]);

    expect(visibleIds(canvas)).toEqual(['a', 'b']);
  });

  describe('query parameters', () => {
    it('sends no space while all spaces are shown', async () => {
      const { repository } = await createNotesHarness([createNote()]);

      expect(repository.lastQuery?.spaceId).toBeNull();
    });

    it('sends the active space once one is selected', async () => {
      const { repository, spaces } = await createNotesHarness([createNote()]);
      const before = repository.queryCount;

      spaces.selectSpace('space-2');
      await awaitQuery(repository, before);

      expect(repository.lastQuery?.spaceId).toBe('space-2');
    });

    it('sends the active quick filter', async () => {
      const { canvas, repository } = await createNotesHarness([createNote()]);
      const before = repository.queryCount;

      canvas.setFilter('untriaged');
      await awaitQuery(repository, before);

      expect(repository.lastQuery?.filter).toBe('untriaged');
    });

    it('sends the selected tags', async () => {
      const { canvas, repository } = await createNotesHarness([createNote()]);
      const before = repository.queryCount;

      canvas.toggleTag('urgent');
      await awaitQuery(repository, before);

      expect(repository.lastQuery?.tags).toEqual(['urgent']);
    });

    it('drops a tag that is toggled twice', async () => {
      const { canvas, repository } = await createNotesHarness([createNote()]);

      const beforeAdd = repository.queryCount;
      canvas.toggleTag('urgent');
      await awaitQuery(repository, beforeAdd);
      expect(repository.lastQuery?.tags).toEqual(['urgent']);

      const beforeRemove = repository.queryCount;
      canvas.toggleTag('urgent');
      await awaitQuery(repository, beforeRemove);

      expect(repository.lastQuery?.tags).toEqual([]);
    });

    it('sends the selected languages', async () => {
      const { canvas, repository } = await createNotesHarness([createNote()]);
      const before = repository.queryCount;

      // Catches a `sameQueryParams` missing its languages clause.
      canvas.toggleLanguage('json');
      await awaitQuery(repository, before);

      expect(repository.lastQuery?.languages).toEqual(['json']);
    });

    it('drops a language that is toggled twice', async () => {
      const { canvas, repository } = await createNotesHarness([createNote()]);

      const beforeAdd = repository.queryCount;
      canvas.toggleLanguage('json');
      await awaitQuery(repository, beforeAdd);
      expect(repository.lastQuery?.languages).toEqual(['json']);

      const beforeRemove = repository.queryCount;
      canvas.toggleLanguage('json');
      await awaitQuery(repository, beforeRemove);

      expect(repository.lastQuery?.languages).toEqual([]);
    });

    it('sends the timezone offset, without which sections straddle local midnight', async () => {
      const { repository } = await createNotesHarness([createNote()]);

      expect(repository.lastQuery?.tzOffsetMinutes).toBe(repository.lastQuery?.now.getTimezoneOffset());
    });

    it('sends the raw tag as typed, leaving normalisation to the backend', async () => {
      const { store, repository } = await createNotesHarness([createNote({ id: 'a', tags: [] })]);
      const update = vi.spyOn(repository, 'update');

      await store.applyPatch('a', { tags: ['  #urgent '] });

      expect(update).toHaveBeenCalledWith('a', { tags: ['  #urgent '] });
    });
  });

  /** Only the `equal` comparator keeps a clock tick from firing a full IPC round trip. */
  describe('clock sensitivity', () => {
    async function createStoreWithClock(now: WritableSignal<Date>): Promise<NotesHarness> {
      const repository = new FakeNotesRepository([createNote()]);
      const clipboard = new FakeClipboard();
      const folders = new FakeFoldersRepository();
      TestBed.configureTestingModule({
        providers: [
          provideAppTesting({
            notesRepository: repository,
            spaces: HARNESS_SPACES,
            clipboard,
            foldersRepository: folders,
          }),
          { provide: ClockService, useValue: { now: now.asReadonly() } },
        ],
      });

      const canvas = TestBed.inject(NotesQueryStore);
      await vi.waitFor(() => expect(canvas.isLoading()).toBe(false));

      return {
        store: TestBed.inject(NotesStore),
        batch: TestBed.inject(NoteBatchStore),
        undo: TestBed.inject(UndoStore),
        canvas,
        selection: TestBed.inject(NoteSelectionStore),
        repository,
        folders,
        spaces: TestBed.inject(SpacesStore),
        clipboard,
      };
    }

    it('does not re-query when the clock ticks inside the same local day', async () => {
      const now = signal(new Date(2026, 6, 25, 12, 0, 0));
      const { repository } = await createStoreWithClock(now);
      const before = repository.queryCount;

      now.set(new Date(2026, 6, 25, 12, 0, 30));
      now.set(new Date(2026, 6, 25, 23, 59, 59));
      // Give the resource's load effect every chance to fire.
      await new Promise((resolve) => setTimeout(resolve, 20));

      expect(repository.queryCount).toBe(before);
    });

    it('re-queries when the local day changes, or sections would stay on yesterday', async () => {
      const now = signal(new Date(2026, 6, 25, 23, 59, 59));
      const { repository } = await createStoreWithClock(now);
      const before = repository.queryCount;

      now.set(new Date(2026, 6, 26, 0, 0, 1));

      await awaitQuery(repository, before);
    });
  });

  describe('clearing the filters', () => {
    it('drops the search, the tags and the languages together', async () => {
      const { canvas, repository } = await createNotesHarness([createNote()]);
      canvas.setSearchQuery('deploy');
      canvas.toggleTag('urgent');
      canvas.toggleLanguage('json');
      // Waited for: three setters undone before the `computed` runs would collapse to
      // no change at all and prove nothing.
      await vi.waitFor(() => expect(repository.lastQuery?.search).toBe('deploy'));
      const before = repository.queryCount;

      canvas.clearFilters();
      await awaitQuery(repository, before);

      expect(repository.lastQuery?.search).toBe('');
      expect(repository.lastQuery?.tags).toEqual([]);
      expect(repository.lastQuery?.languages).toEqual([]);
      // The field is what the user is looking at, and it has to look empty too.
      expect(canvas.searchQuery()).toBe('');
    });

    /** Not through `setSearchQuery`: its debounce would leave the canvas filtered. */
    it('takes the search out of the query without waiting for the debounce', async () => {
      const { canvas, repository } = await createNotesHarness([createNote()]);
      canvas.setSearchQuery('deploy');
      await vi.waitFor(() => expect(repository.lastQuery?.search).toBe('deploy'));
      const before = repository.queryCount;

      canvas.clearFilters();
      await awaitQuery(repository, before);

      expect(repository.lastQuery?.search).toBe('');
    });

    /**
     * Clearing has to cancel the pending call, not merely set the signals past it: a
     * keystroke still on its way lands 150 ms later and puts the query back. Real timers,
     * and a real wait — faking them only proves the assertion ran before the timer did.
     */
    it('drops a keystroke still in flight when the filters are cleared', async () => {
      const { canvas, repository } = await createNotesHarness([createNote()]);
      const query = vi.spyOn(repository, 'query');

      canvas.setSearchQuery('deploy');
      canvas.clearFilters();
      await new Promise((resolve) => setTimeout(resolve, SEARCH_DEBOUNCE_MS * 3));

      const searched = query.mock.calls.map(([sent]) => sent.search);
      expect(searched).not.toContain('deploy');
      expect(canvas.searchQuery()).toBe('');
    });

    it('leaves the quick filter alone, which has a control of its own', async () => {
      const { canvas, repository } = await createNotesHarness([createNote()]);
      canvas.setFilter('pinned');
      canvas.toggleTag('urgent');
      const before = repository.queryCount;

      canvas.clearFilters();
      await awaitQuery(repository, before);

      expect(repository.lastQuery?.filter).toBe('pinned');
      expect(canvas.activeFilter()).toBe('pinned');
    });
  });

  describe('search debounce', () => {
    beforeEach(() => {
      // Only Date and timers: faking requestAnimationFrame hangs the zoneless
      // scheduler in whenStable().
      vi.useFakeTimers({ toFake: ['setTimeout', 'clearTimeout'] });
    });

    it('updates the field immediately so typing never lags', async () => {
      const { canvas } = await createNotesHarness([createNote()]);

      canvas.setSearchQuery('dep');

      expect(canvas.searchQuery()).toBe('dep');
    });

    it('does not query the backend before the debounce elapses', async () => {
      const { canvas, repository } = await createNotesHarness([createNote()]);
      const before = repository.queryCount;

      canvas.setSearchQuery('dep');

      expect(repository.queryCount).toBe(before);
    });

    it('sends a single query for a burst of keystrokes', async () => {
      const { canvas, repository } = await createNotesHarness([createNote()]);
      const before = repository.queryCount;

      canvas.setSearchQuery('d');
      canvas.setSearchQuery('de');
      canvas.setSearchQuery('dep');
      vi.advanceTimersByTime(SEARCH_DEBOUNCE_MS);
      await vi.waitFor(() => expect(repository.queryCount).toBe(before + 1));

      expect(repository.lastQuery?.search).toBe('dep');
    });

    it('trims the query it sends', async () => {
      const { canvas, repository } = await createNotesHarness([createNote()]);
      const before = repository.queryCount;

      canvas.setSearchQuery('  dep  ');
      vi.advanceTimersByTime(SEARCH_DEBOUNCE_MS);
      await awaitQuery(repository, before);

      expect(repository.lastQuery?.search).toBe('dep');
    });
  });

  describe('loading and error state', () => {
    it('exposes the repository failure and shows nothing', async () => {
      const repository = new FakeNotesRepository([createNote()]);
      repository.failNext = new Error('backend down');
      TestBed.configureTestingModule({
        providers: [provideAppTesting({ notesRepository: repository })],
      });

      const canvas = TestBed.inject(NotesQueryStore);
      await vi.waitFor(() => expect(canvas.loadError()).toBeDefined());

      expect(canvas.loadError()?.message).toBe('backend down');
      expect(canvas.sections()).toEqual([]);
    });

    it('recovers the view on reload after a failed load', async () => {
      const repository = new FakeNotesRepository([createNote({ id: 'recovered' })]);
      repository.failNext = new Error('transient');
      TestBed.configureTestingModule({
        providers: [provideAppTesting({ notesRepository: repository })],
      });

      const canvas = TestBed.inject(NotesQueryStore);
      await vi.waitFor(() => expect(canvas.loadError()).toBeDefined());

      canvas.reload();
      await vi.waitFor(() => expect(visibleIds(canvas)).toHaveLength(1));

      expect(canvas.loadError()).toBeUndefined();
      expect(visibleIds(canvas)).toEqual(['recovered']);
    });

    /** Rust computes every query it is sent to the end, one at a time behind its lock. */
    it('sends the query in flight and the newest, never the ones a burst outran', async () => {
      const { canvas, repository } = await createNotesHarness([createNote()]);
      const before = repository.queryCount;
      repository.hold();

      canvas.setFilter('pinned');
      TestBed.tick();
      canvas.setFilter('untriaged');
      TestBed.tick();
      canvas.toggleTag('urgent');
      TestBed.tick();
      repository.release();
      await vi.waitFor(() => expect(repository.lastQuery?.tags).toEqual(['urgent']));

      expect(repository.queryCount - before).toBe(2);
      expect(repository.lastQuery?.filter).toBe('untriaged');
    });

    it('keeps the previous results on screen while a new query runs', async () => {
      const { canvas, spaces } = await createNotesHarness([createNote({ id: 'a' })]);

      spaces.selectSpace('space-2');

      expect(canvas.isLoading()).toBe(false);
      expect(visibleIds(canvas)).toEqual(['a']);
    });
  });

  describe('view-derived state', () => {
    it('reports no results only when a search is actually active', async () => {
      const { canvas, repository } = await createNotesHarness([]);
      const before = repository.queryCount;
      repository.setView({ isFiltering: true, matched: 0, sections: [] });

      canvas.setFilter('pinned');
      await awaitQuery(repository, before);

      expect(canvas.hasNoResults()).toBe(true);
    });

    it('does not report "no results" for an empty space', async () => {
      const { canvas } = await createNotesHarness([]);

      expect(canvas.isFiltering()).toBe(false);
      expect(canvas.hasNoResults()).toBe(false);
    });

    it('exposes the tags the backend offers for the rail', async () => {
      const { canvas } = await createNotesHarness([
        createNote({ id: 'a', tags: ['zeta', 'alpha'] }),
        createNote({ id: 'b', tags: ['alpha'] }),
      ]);

      expect(canvas.allTags()).toEqual(['alpha', 'zeta']);
    });

    it('exposes the languages the backend offers for the rail', async () => {
      const { canvas } = await createNotesHarness([
        createNote({ id: 'a', language: 'yml' }),
        createNote({ id: 'b', language: 'json' }),
        createNote({ id: 'c', language: 'json' }),
      ]);

      expect(canvas.allLanguages()).toEqual(['json', 'yml']);
    });
  });

  describe('backend-shaped view', () => {
    it('renders whatever sections the backend sends, in order', async () => {
      const { canvas, repository } = await createNotesHarness([createNote({ id: 'a' })]);
      const before = repository.queryCount;
      const view: Partial<NotesView> = {
        sections: [
          { key: 'pinned', notes: [], hasExpiringNotes: false, showCreateGhost: false },
          { key: 'today', notes: [], hasExpiringNotes: true, showCreateGhost: false },
          { key: 'week', notes: [], hasExpiringNotes: false, showCreateGhost: true },
        ],
      };
      repository.setView(view);

      canvas.setFilter('pinned');
      await awaitQuery(repository, before);

      expect(canvas.sections().map((section) => section.key)).toEqual(['pinned', 'today', 'week']);
    });
  });
});
