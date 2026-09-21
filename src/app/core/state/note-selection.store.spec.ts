import { TestBed } from '@angular/core/testing';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { ErrorNotifier } from '@core/services/errors/error-notifier.service';
import { createNote } from '@testing/note.fixture';
import { FakeBoardRepository, fakeBoardNote, fakeZone } from '@testing/fake-board-repository';
import { provideAppTesting } from '@testing/testing.providers';
import { NotesHarness, awaitQuery, createNotesHarness, visibleIds } from '@testing/notes-harness';
import { BoardStore } from './board.store';
import { NoteSelectionStore } from './note-selection.store';
import { SpacesStore } from './spaces.store';

describe('NoteSelectionStore', () => {
  beforeEach(() => {
    TestBed.resetTestingModule();
    vi.restoreAllMocks();
    // The stores report failures through console.error on purpose; silence it
    // so a deliberately failing test doesn't look like a crash.
    vi.spyOn(console, 'error').mockImplementation(() => undefined);
  });

  describe('multiple selection', () => {
    async function withThreeNotes(): Promise<NotesHarness> {
      return createNotesHarness([createNote({ id: 'a' }), createNote({ id: 'b' }), createNote({ id: 'c' })]);
    }

    it('starts empty and reports no selection', async () => {
      const { selection } = await withThreeNotes();

      expect(selection.checkedCount()).toBe(0);
      expect(selection.hasSelection()).toBe(false);
    });

    it('toggles a note in and out of the selection', async () => {
      const { selection } = await withThreeNotes();

      selection.toggleChecked('b');
      expect(selection.checkedNotes().map((note) => note.id)).toEqual(['b']);

      selection.toggleChecked('b');
      expect(selection.hasSelection()).toBe(false);
    });

    it('never hands out a note that is no longer displayed', async () => {
      const { canvas, selection, repository } = await withThreeNotes();
      selection.toggleChecked('a');
      const before = repository.queryCount;

      repository.setView({ sections: [] });
      canvas.setFilter('pinned');
      await awaitQuery(repository, before);

      expect(selection.checkedNotes()).toEqual([]);
    });

    it('extends the selection from the focused note to the clicked one', async () => {
      const { selection } = await withThreeNotes();
      selection.focusNote('a');

      selection.checkRangeTo('c');

      expect(selection.checkedNotes().map((note) => note.id)).toEqual(['a', 'b', 'c']);
    });

    it('checks a single note when there is no anchor', async () => {
      const { selection } = await withThreeNotes();

      selection.checkRangeTo('b');

      expect(selection.checkedNotes().map((note) => note.id)).toEqual(['b']);
    });

    it('moves the whole selection in one call', async () => {
      const { store, selection, repository } = await withThreeNotes();
      selection.toggleChecked('a');
      selection.toggleChecked('c');

      await store.moveSelection('space-2');

      expect(repository.movedTo).toEqual({ ids: ['a', 'c'], spaceId: 'space-2' });
    });

    it('sends the typed tag through untouched', async () => {
      const { store, selection, repository } = await withThreeNotes();
      selection.toggleChecked('a');

      await store.tagSelection('#Urgent');

      expect(repository.taggedWith).toEqual({ ids: ['a'], tags: ['#Urgent'] });
    });

    it('files the whole selection in one call', async () => {
      const { store, selection, folders } = await withThreeNotes();
      selection.toggleChecked('a');
      selection.toggleChecked('c');

      await store.fileSelection('perf');

      expect([...folders.filings]).toEqual([
        ['a', 'perf'],
        ['c', 'perf'],
      ]);
    });

    /** Both directions are one action: taking a note out is a filing with no folder. */
    it('takes the selection out of its folder with the same call', async () => {
      const { store, selection, folders } = await withThreeNotes();
      selection.toggleChecked('a');
      await store.fileSelection('perf');

      await store.fileSelection(null);

      expect(folders.filings.get('a')).toBeNull();
    });

    /**
     * ⚠️ The record carries what the back end answered, never what the front guessed:
     * rebuilding it from the selection would unfile a note the batch never touched.
     */
    it('offers to put a filing back, folder by folder', async () => {
      const { store, selection, folders } = await withThreeNotes();
      selection.toggleChecked('a');
      await store.fileSelection('migrations');
      selection.toggleChecked('c');

      await store.fileSelection('perf');

      expect(store.undoBanner()).toEqual({
        kind: 'file',
        previous: [
          { noteId: 'a', folderId: 'migrations' },
          { noteId: 'c', folderId: null },
        ],
        count: 2,
      });

      await store.undoLastAction();

      expect(folders.filings.get('a')).toBe('migrations');
      expect(folders.filings.get('c')).toBeNull();
    });

    /** A bar offering to undo zero notes is noise. */
    it('opens no undo window when the selection was already in that folder', async () => {
      const { store, selection } = await withThreeNotes();
      selection.toggleChecked('a');
      await store.fileSelection('perf');
      store.dismissUndo();

      await store.fileSelection('perf');

      expect(store.undoBanner()).toBeNull();
    });

    it('ignores a blank tag rather than sending it', async () => {
      const { store, selection, repository } = await withThreeNotes();
      selection.toggleChecked('a');

      await store.tagSelection('   ');

      expect(repository.taggedWith).toBeNull();
    });

    it('does nothing at all without a selection', async () => {
      const { store, canvas, repository } = await withThreeNotes();

      await store.moveSelection('space-2');
      await store.tagSelection('urgent');
      await store.deleteSelection();

      expect(repository.movedTo).toBeNull();
      expect(repository.taggedWith).toBeNull();
      expect(visibleIds(canvas)).toEqual(['a', 'b', 'c']);
    });

    /**
     * ⚠️ The asymmetry this closes: deleting a note had three safety nets, and moving
     * thirty of them had none — where putting a move back by hand means remembering
     * which thirty, and which space each one came from.
     */
    it('offers to put a move back, space by space', async () => {
      const { store, selection, repository } = await createNotesHarness([
        createNote({ id: 'a', spaceId: 'space-1' }),
        createNote({ id: 'b', spaceId: 'space-2' }),
      ]);
      selection.toggleChecked('a');
      selection.toggleChecked('b');

      await store.moveSelection('space-3');

      expect(store.undoBanner()).toEqual({
        kind: 'move',
        previous: [
          { noteId: 'a', spaceId: 'space-1' },
          { noteId: 'b', spaceId: 'space-2' },
        ],
        count: 2,
      });

      const queries = repository.queryCount;
      await store.undoLastAction();
      await awaitQuery(repository, queries);

      expect(repository.spaceOf('a')).toBe('space-1');
      expect(repository.spaceOf('b')).toBe('space-2');
    });

    it('offers to put a tagging back', async () => {
      const { store, selection, repository } = await createNotesHarness([createNote({ id: 'a', tags: [] })]);
      selection.toggleChecked('a');

      await store.tagSelection('urgent');

      expect(store.undoBanner()?.kind).toBe('tag');

      const queries = repository.queryCount;
      await store.undoLastAction();
      await awaitQuery(repository, queries);

      expect(repository.tagsOf('a')).toEqual([]);
    });

    /**
     * ⚠️ The reason the back end answers pairs rather than a count: the note that already
     * carried the tag gained nothing, so the undo must not take it away.
     */
    it('undoing a tagging leaves the tag on the note that already carried it', async () => {
      const { store, selection, repository } = await createNotesHarness([
        createNote({ id: 'a', tags: ['Urgent'] }),
        createNote({ id: 'b', tags: [] }),
      ]);
      selection.toggleChecked('a');
      selection.toggleChecked('b');

      await store.tagSelection('urgent');
      const queries = repository.queryCount;
      await store.undoLastAction();
      await awaitQuery(repository, queries);

      expect(repository.tagsOf('a')).toEqual(['Urgent']);
      expect(repository.tagsOf('b')).toEqual([]);
    });

    /** A bar offering to undo nothing is noise, not a safety net. */
    it('offers no undo when the batch changed nothing', async () => {
      const { store, selection } = await createNotesHarness([createNote({ id: 'a', spaceId: 'space-1' })]);
      selection.toggleChecked('a');

      await store.moveSelection('space-1');

      expect(store.undoBanner()).toBeNull();
      expect(store.lastAction()).toBeNull();
    });

    it('reports a failed bulk action without clearing the selection', async () => {
      const { store, selection, repository } = await withThreeNotes();
      const notifier = TestBed.inject(ErrorNotifier);
      selection.toggleChecked('a');
      repository.failNext = new Error('boom');

      await store.moveSelection('space-2');

      expect(notifier.notice()?.ref.key).toBe('errors.bulkActionFailed');
      expect(selection.checkedCount()).toBe(1);
    });
  });

  /** Eleven cards in a zone was eleven clicks, and the zone already knows what it holds. */
  describe('selecting a whole folder', () => {
    async function withFiledNotes(): Promise<NotesHarness> {
      return createNotesHarness([
        createNote({ id: 'a', folderId: 'perf' }),
        createNote({ id: 'b', folderId: 'perf' }),
        createNote({ id: 'c', folderId: 'migrations' }),
        createNote({ id: 'd', folderId: null }),
      ]);
    }

    it('ticks every note filed there, and nothing else', async () => {
      const { selection } = await withFiledNotes();

      selection.checkFolder('perf');

      expect(selection.checkedNotes().map((note) => note.id)).toEqual(['a', 'b']);
    });

    it('adds to what was already ticked rather than replacing it', async () => {
      const { selection } = await withFiledNotes();
      selection.toggleChecked('c');

      selection.checkFolder('perf');

      expect(
        selection
          .checkedNotes()
          .map((note) => note.id)
          .sort(),
      ).toEqual(['a', 'b', 'c']);
    });

    it('leaves the selection alone for a folder holding nothing on screen', async () => {
      const { selection } = await withFiledNotes();
      selection.toggleChecked('d');

      selection.checkFolder('archives');

      expect(selection.checkedNotes().map((note) => note.id)).toEqual(['d']);
    });
  });

  /**
   * ⚠️ The board **dims** where the canvas **narrows**. A card the search filtered out of
   * the date view is still drawn on the board and still in its folder — resolved against
   * the canvas it left the selection the instant it was ticked, and the bar said nothing
   * was selected.
   */
  describe('on the board, where nothing is narrowed away', () => {
    async function onBoard(): Promise<{ selection: NoteSelectionStore; board: BoardStore }> {
      const boardRepository = new FakeBoardRepository({
        zones: [
          fakeZone({
            folder: {
              id: 'perf',
              spaceId: 'sql',
              name: 'Perf',
              colour: 'amber',
              createdAt: new Date('2026-01-01T10:00:00Z'),
            },
            notes: [
              fakeBoardNote(createNote({ id: 'shown', spaceId: 'sql', folderId: 'perf' })),
              fakeBoardNote(createNote({ id: 'dimmed', spaceId: 'sql', folderId: 'perf' }), {
                matches: false,
              }),
            ],
          }),
        ],
      });
      TestBed.configureTestingModule({
        providers: [
          provideAppTesting({
            notes: [createNote({ id: 'shown', spaceId: 'sql', folderId: 'perf' })],
            spaces: [{ id: 'sql', name: 'SQL', pinned: false }],
            boardRepository,
          }),
        ],
      });

      const spaces = TestBed.inject(SpacesStore);
      const board = TestBed.inject(BoardStore);
      await vi.waitFor(() => expect(spaces.spaces()).toHaveLength(1));
      spaces.selectSpace('sql');
      board.setMode('board');
      await vi.waitFor(() => expect(board.visibleNotes()).toHaveLength(2));

      return { selection: TestBed.inject(NoteSelectionStore), board };
    }

    it('keeps a dimmed card in the selection the canvas has filtered out', async () => {
      const { selection } = await onBoard();

      selection.toggleChecked('dimmed');

      expect(selection.checkedNotes().map((note) => note.id)).toEqual(['dimmed']);
    });

    it('ticks a whole zone, dimmed cards included', async () => {
      const { selection } = await onBoard();

      selection.checkFolder('perf');

      expect(
        selection
          .checkedNotes()
          .map((note) => note.id)
          .sort(),
      ).toEqual(['dimmed', 'shown']);
    });
  });

  describe('keyboard focus', () => {
    it('flattens the sections in display order', async () => {
      const { canvas, repository } = await createNotesHarness([createNote({ id: 'a' })]);
      const before = repository.queryCount;
      repository.setView({
        sections: [
          {
            key: 'pinned',
            notes: [createNote({ id: 'p' })],
            hasExpiringNotes: false,
            showCreateGhost: false,
          },
          {
            key: 'week',
            notes: [createNote({ id: 'w' })],
            hasExpiringNotes: false,
            showCreateGhost: false,
          },
        ],
      });
      canvas.setFilter('pinned');
      await awaitQuery(repository, before);

      expect(canvas.visibleNotes().map((note) => note.id)).toEqual(['p', 'w']);
    });

    it('reports no index when nothing is focused', async () => {
      const { selection } = await createNotesHarness([createNote({ id: 'a' })]);

      expect(selection.focusedIndex()).toBe(-1);
    });

    /**
     * ⚠️ By id, never by position. Focus used to be set with an index into `visibleNotes`
     * while the caller had measured the grid in DOM order — the same list on the date view,
     * a different one on the board, where the first arrow jumped two cards sideways.
     */
    it('focuses a note, and still knows where it sits', async () => {
      const { selection } = await createNotesHarness([createNote({ id: 'a' }), createNote({ id: 'b' })]);

      selection.focusNote('b');

      expect(selection.focusedNoteId()).toBe('b');
      expect(selection.focusedIndex()).toBe(1);
    });

    it('reports no position for a note that is not on the canvas', async () => {
      const { selection } = await createNotesHarness([createNote({ id: 'a' })]);

      selection.focusNote('gone');

      expect(selection.focusedNoteId()).toBe('gone');
      expect(selection.focusedIndex()).toBe(-1);
      expect(selection.focusedNote()).toBeNull();
    });

    it('follows the note being opened', async () => {
      const { store, selection } = await createNotesHarness([
        createNote({ id: 'a' }),
        createNote({ id: 'b' }),
      ]);

      store.openNote('b');

      expect(selection.focusedNoteId()).toBe('b');
    });
  });
});
