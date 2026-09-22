import { TestBed } from '@angular/core/testing';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { LibraryPreferencesService } from '@core/services/preferences/library-preferences.service';
import { createNote } from '@testing/note.fixture';
import { FakeBoardRepository, fakeBoardNote, fakeZone } from '@testing/fake-board-repository';
import { FakeSpacesRepository } from '@testing/fake-spaces-repository';
import { provideAppTesting } from '@testing/testing.providers';
import { Folder } from '../model/folder.model';
import { Space } from '../model/space.model';
import { FakeFoldersRepository } from '@testing/fake-folders-repository';
import { BoardStore, LAYOUT_SAVE_DEBOUNCE_MS } from './board.store';
import { NotesQueryStore } from './notes-query.store';
import { SpacesStore } from './spaces.store';

const SPACES: readonly Space[] = [
  { id: 'sql', name: 'SQL', pinned: false },
  { id: 'veille', name: 'Veille', pinned: false },
];

const PERF: Folder = {
  id: 'perf',
  spaceId: 'sql',
  name: 'Perf',
  colour: 'amber',
  createdAt: new Date('2026-01-01T10:00:00Z'),
};

interface Harness {
  readonly store: BoardStore;
  readonly spaces: SpacesStore;
  readonly canvas: NotesQueryStore;
  readonly repository: FakeBoardRepository;
  readonly folders: FakeFoldersRepository;
  readonly preferences: LibraryPreferencesService;
}

async function createStore(repository = new FakeBoardRepository()): Promise<Harness> {
  const folders = new FakeFoldersRepository();
  TestBed.configureTestingModule({
    providers: [
      provideAppTesting({
        boardRepository: repository,
        foldersRepository: folders,
        spacesRepository: new FakeSpacesRepository(SPACES),
      }),
    ],
  });

  const spaces = TestBed.inject(SpacesStore);
  await vi.waitFor(() => expect(spaces.spaces()).toHaveLength(SPACES.length));

  return {
    store: TestBed.inject(BoardStore),
    spaces,
    canvas: TestBed.inject(NotesQueryStore),
    repository,
    folders,
    preferences: TestBed.inject(LibraryPreferencesService),
  };
}

async function onBoard(harness: Harness, spaceId = 'sql'): Promise<void> {
  harness.spaces.selectSpace(spaceId);
  harness.store.setMode('board');
  await vi.waitFor(() => expect(harness.repository.queryCount).toBeGreaterThan(0));
}

describe('BoardStore', () => {
  beforeEach(() => {
    TestBed.resetTestingModule();
    localStorage.clear();
    vi.restoreAllMocks();
    // Failures are reported through console.error on purpose.
    vi.spyOn(console, 'error').mockImplementation(() => undefined);
  });

  it('starts on the date view, which stays the default', async () => {
    const { store } = await createStore();

    expect(store.mode()).toBe('date');
    expect(store.isBoard()).toBe(false);
  });

  /** ⚠️ A folder belongs to a space, so there would be no zones to draw. */
  it('refuses the board while the user is on all spaces', async () => {
    const { store } = await createStore();

    store.setMode('board');

    expect(store.canShowBoard()).toBe(false);
    expect(store.mode()).toBe('date');
  });

  it('asks nothing at all while the date view is showing', async () => {
    const { store, spaces, repository } = await createStore();
    spaces.selectSpace('sql');
    await vi.waitFor(() => expect(store.canShowBoard()).toBe(true));

    expect(repository.queryCount).toBe(0);
  });

  /**
   * ⚠️ Its own lookup, and not the canvas's: the board **dims** where the canvas
   * **narrows**, so a card here can be acted on while its note is nowhere in that view.
   */
  it('finds a card wherever it sits, zone or background, dimmed or not', async () => {
    const repository = new FakeBoardRepository({
      zones: [fakeZone({ folder: PERF, notes: [fakeBoardNote(createNote({ id: 'a' }))] })],
      loose: [fakeBoardNote(createNote({ id: 'b' }), { matches: false })],
    });
    const harness = await createStore(repository);
    await onBoard(harness);
    await vi.waitFor(() => expect(harness.store.zones()).toHaveLength(1));

    expect(harness.store.findVisible('a')?.id).toBe('a');
    expect(harness.store.findVisible('b')?.id).toBe('b');
    expect(harness.store.findVisible('nowhere')).toBeNull();
  });

  it('draws the zones and the loose cards the back end answered', async () => {
    const repository = new FakeBoardRepository({
      zones: [fakeZone({ folder: PERF, notes: [fakeBoardNote(createNote({ id: 'a' }))] })],
      loose: [fakeBoardNote(createNote({ id: 'b' }), { position: { x: 16, y: 400 } })],
      width: 1200,
      height: 800,
    });
    const harness = await createStore(repository);

    await onBoard(harness);

    expect(harness.store.zones()).toHaveLength(1);
    expect(harness.store.zones()[0]?.folder.name).toBe('Perf');
    expect(harness.store.loose()).toHaveLength(1);
    expect(harness.store.width()).toBe(1200);
    expect(harness.store.noteCount()).toBe(2);
  });

  it('queries the active space with what the header is filtering on', async () => {
    const harness = await createStore();
    harness.canvas.setFilter('pinned');
    harness.canvas.toggleTag('sql');

    await onBoard(harness);

    expect(harness.repository.lastQuery?.spaceId).toBe('sql');
    expect(harness.repository.lastQuery?.filter).toBe('pinned');
    expect(harness.repository.lastQuery?.tags).toEqual(['sql']);
  });

  /** ⚠️ Without the `equal` comparator a fresh literal fires a query on every tick. */
  it('does not re-query when nothing it reads has moved', async () => {
    const harness = await createStore();
    await onBoard(harness);
    const before = harness.repository.queryCount;

    harness.store.setMode('board');
    await Promise.resolve();

    expect(harness.repository.queryCount).toBe(before);
  });

  it('re-queries when a filter moves', async () => {
    const harness = await createStore();
    await onBoard(harness);
    const before = harness.repository.queryCount;

    harness.canvas.setFilter('pinned');

    await vi.waitFor(() => expect(harness.repository.queryCount).toBeGreaterThan(before));
  });

  /** Per space, so arranging one does not switch the others. */
  it('remembers the view of each space on its own', async () => {
    const harness = await createStore();
    await onBoard(harness, 'sql');

    harness.spaces.selectSpace('veille');
    await vi.waitFor(() => expect(harness.store.mode()).toBe('date'));

    harness.spaces.selectSpace('sql');
    await vi.waitFor(() => expect(harness.store.mode()).toBe('board'));
  });

  it('writes the chosen view to the preferences under a key of its own', async () => {
    const harness = await createStore();
    const write = vi.spyOn(harness.preferences, 'write');

    await onBoard(harness);

    expect(write).toHaveBeenCalledWith('devnotes.notes.view.sql', 'board');
  });

  it('reports what a search dimmed rather than what it removed', async () => {
    const repository = new FakeBoardRepository({
      zones: [
        fakeZone({
          folder: PERF,
          notes: [
            fakeBoardNote(createNote({ id: 'a' })),
            fakeBoardNote(createNote({ id: 'b' }), { matches: false }),
          ],
        }),
      ],
      isFiltering: true,
      matched: 1,
    });
    const harness = await createStore(repository);

    await onBoard(harness);

    expect(harness.store.zones()[0]?.notes).toHaveLength(2);
    expect(harness.store.matched()).toBe(1);
  });

  /**
   * ⚠️ A `computed` reading `hasValue()` answers `null` for the whole round trip, so the
   * board went blank on every reload — a card disappearing under the pointer that ticked it.
   */
  it('keeps what it is drawing while it re-reads', async () => {
    const repository = new FakeBoardRepository({
      zones: [fakeZone({ folder: PERF, notes: [fakeBoardNote(createNote({ id: 'a' }))] })],
    });
    const harness = await createStore(repository);
    await onBoard(harness);
    await vi.waitFor(() => expect(harness.store.zones()).toHaveLength(1));

    harness.store.reload();

    expect(harness.store.zones()).toHaveLength(1);
    expect(harness.store.isLoading()).toBe(false);
  });

  it('reports no count while nothing is dimming anything', async () => {
    const harness = await createStore();

    await onBoard(harness);

    expect(harness.store.matched()).toBeNull();
  });

  describe('what a gesture writes', () => {
    /** ⚠️ One write per gesture, not one per pointermove. */
    it('coalesces everything moved into a single batch', async () => {
      const harness = await createStore(
        new FakeBoardRepository({
          zones: [fakeZone({ folder: PERF })],
          loose: [fakeBoardNote(createNote({ id: 'a' }), { position: { x: 0, y: 0 } })],
        }),
      );
      await onBoard(harness);

      harness.store.moveZone('perf', { x: 40, y: 40, width: 500, height: 300 });
      harness.store.moveCard('a', { x: 80, y: 600 });
      expect(harness.repository.saved).toHaveLength(0);

      await vi.waitFor(() => expect(harness.repository.saved).toHaveLength(1), {
        timeout: LAYOUT_SAVE_DEBOUNCE_MS * 6,
      });
      const batch = harness.repository.saved[0];
      expect(batch?.zones).toEqual([{ folderId: 'perf', frame: { x: 40, y: 40, width: 500, height: 300 } }]);
      expect(batch?.cards).toEqual([{ noteId: 'a', position: { x: 80, y: 600 } }]);
    });

    /** ⚠️ Without the overlay the card snaps back to where the server last saw it. */
    it('shows the move at once, before it is written', async () => {
      const harness = await createStore(
        new FakeBoardRepository({
          zones: [fakeZone({ folder: PERF, frame: { x: 16, y: 16, width: 516, height: 200 } })],
        }),
      );
      await onBoard(harness);

      harness.store.moveZone('perf', { x: 400, y: 300, width: 516, height: 200 });

      expect(harness.store.zones()[0]?.frame.x).toBe(400);
    });

    /**
     * ⚠️ The overlay used to be dropped when the **write** returned, and  only
     * asks: the view still on screen is the one read before the drag, so for a whole round
     * trip the card was drawn where it came from. It flashed back, then settled.
     */
    it('holds the drop until a view comes back carrying it', async () => {
      const harness = await createStore(
        new FakeBoardRepository({
          zones: [fakeZone({ folder: PERF, frame: { x: 16, y: 16, width: 516, height: 200 } })],
        }),
      );
      await onBoard(harness);

      harness.store.moveZone('perf', { x: 400, y: 300, width: 516, height: 200 });
      await vi.waitFor(() => expect(harness.repository.saved).toHaveLength(1), {
        timeout: LAYOUT_SAVE_DEBOUNCE_MS * 6,
      });

      // The write has returned and the reload is still in flight.
      expect(harness.store.zones()[0]?.frame.x).toBe(400);
    });

    it('lets go of the drop once the view carries it', async () => {
      const resting = { x: 16, y: 16, width: 516, height: 200 };
      const moved = { x: 400, y: 300, width: 516, height: 200 };
      const harness = await createStore(
        new FakeBoardRepository({ zones: [fakeZone({ folder: PERF, frame: resting })] }),
      );
      await onBoard(harness);

      // ⚠️ The width is what says the reload has actually landed: the frame reads the same
      // either way while the overlay is still up, so asserting on it would prove nothing.
      harness.repository.setView({ zones: [fakeZone({ folder: PERF, frame: moved })], width: 1234 });
      harness.store.moveZone('perf', moved);
      await vi.waitFor(() => expect(harness.store.width()).toBe(1234), {
        timeout: LAYOUT_SAVE_DEBOUNCE_MS * 6,
      });

      // Nothing is covering the view any more: it puts the zone back and the board follows.
      harness.repository.setView({ zones: [fakeZone({ folder: PERF, frame: resting })] });
      harness.store.reload();

      await vi.waitFor(() => expect(harness.store.zones()[0]?.frame.x).toBe(16));
    });

    it('keeps the overlay when the write fails, rather than snapping back silently', async () => {
      const harness = await createStore(new FakeBoardRepository({ zones: [fakeZone({ folder: PERF })] }));
      await onBoard(harness);
      harness.repository.failNext = new Error('disk full');

      harness.store.moveZone('perf', { x: 400, y: 300, width: 516, height: 200 });
      await vi.waitFor(() => expect(harness.repository.failNext).toBeNull(), {
        timeout: LAYOUT_SAVE_DEBOUNCE_MS * 6,
      });

      expect(harness.store.zones()[0]?.frame.x).toBe(400);
    });

    /** Membership goes through the batch command, the same path the selection bar takes. */
    it('files a card dropped in a zone', async () => {
      const harness = await createStore();
      await onBoard(harness);

      await harness.store.dropCard('a', 'perf', { x: 0, y: 0 });

      expect(harness.folders.filings.get('a')).toBe('perf');
    });

    it('unfiles a card dropped on the background, and remembers where it landed', async () => {
      const harness = await createStore();
      await onBoard(harness);
      await harness.store.dropCard('a', 'perf', { x: 0, y: 0 });

      await harness.store.dropCard('a', null, { x: 320, y: 480 });

      expect(harness.folders.filings.get('a')).toBeNull();
      await vi.waitFor(() => expect(harness.repository.saved.length).toBeGreaterThan(0), {
        timeout: LAYOUT_SAVE_DEBOUNCE_MS * 6,
      });
      expect(harness.repository.saved.at(-1)?.cards).toEqual([{ noteId: 'a', position: { x: 320, y: 480 } }]);
    });

    /**
     * ⚠️ The overlay covered places and not **membership**, so the ghost the drag drew
     * vanished on `pointerup` and the card was drawn back in `loose`, at the place the
     * last view gave it, until the round trip landed. Two frames of flashback (#282).
     */
    it('draws a card inside the zone it was dropped into, before any view says so', async () => {
      const harness = await createStore(
        new FakeBoardRepository({
          zones: [fakeZone({ folder: PERF })],
          loose: [fakeBoardNote(createNote({ id: 'a' }), { position: { x: 16, y: 400 } })],
        }),
      );
      await onBoard(harness);

      await harness.store.dropCard('a', 'perf', { x: 40, y: 40 });

      // The view still answers it loose, and the board does not.
      expect(harness.store.zones()[0]?.notes.map((entry) => entry.note.id)).toEqual(['a']);
      expect(harness.store.loose()).toEqual([]);
      expect(harness.store.noteCount()).toBe(1);
    });

    it('draws a card dropped on the background out of its zone at once', async () => {
      const harness = await createStore(
        new FakeBoardRepository({
          zones: [fakeZone({ folder: PERF, notes: [fakeBoardNote(createNote({ id: 'a' }))] })],
        }),
      );
      await onBoard(harness);

      await harness.store.dropCard('a', null, { x: 320, y: 480 });

      expect(harness.store.zones()[0]?.notes).toEqual([]);
      expect(harness.store.loose().map((entry) => entry.note.id)).toEqual(['a']);
      // And where it was let go of, which is the place overlay doing its own half.
      expect(harness.store.loose()[0]?.position).toEqual({ x: 320, y: 480 });
    });

    /**
     * ⚠️ The opposite of what a refused **place** does. A place the server would not take
     * is worth leaving on screen with a banner beside it; a membership it would not take
     * is a lie about which folder the note is in.
     */
    it('puts the card back in its zone when the file is refused', async () => {
      const harness = await createStore(
        new FakeBoardRepository({
          zones: [fakeZone({ folder: PERF, notes: [fakeBoardNote(createNote({ id: 'a' }))] })],
        }),
      );
      await onBoard(harness);
      harness.folders.failNext = new Error('gone');

      expect(await harness.store.dropCard('a', null, { x: 320, y: 480 })).toBe(false);

      expect(harness.store.zones()[0]?.notes.map((entry) => entry.note.id)).toEqual(['a']);
      expect(harness.store.loose()).toEqual([]);
    });

    it('lets go of the drop once a view carries the new folder', async () => {
      const harness = await createStore(
        new FakeBoardRepository({
          zones: [fakeZone({ folder: PERF })],
          loose: [fakeBoardNote(createNote({ id: 'a' }), { position: { x: 16, y: 400 } })],
        }),
      );
      await onBoard(harness);

      // ⚠️ The width is what says the reload has landed: the zone holds the card either
      // way while the overlay is still up, so asserting on it would prove nothing.
      harness.repository.setView({
        zones: [fakeZone({ folder: PERF, notes: [fakeBoardNote(createNote({ id: 'a' }))] })],
        loose: [],
        width: 1234,
      });
      await harness.store.dropCard('a', 'perf', { x: 40, y: 40 });
      await vi.waitFor(() => expect(harness.store.width()).toBe(1234));

      // Nothing is covering the view any more: it takes the card back out and so does the
      // board, which is what lets a later drag move the same card again.
      harness.repository.setView({
        zones: [fakeZone({ folder: PERF })],
        loose: [fakeBoardNote(createNote({ id: 'a' }), { position: { x: 16, y: 400 } })],
      });
      harness.store.reload();

      await vi.waitFor(() => expect(harness.store.loose()).toHaveLength(1));
      expect(harness.store.zones()[0]?.notes).toEqual([]);
    });

    it('creates a folder from a drawn band, at the frame it was drawn', async () => {
      const harness = await createStore();
      await onBoard(harness);

      const frame = { x: 700, y: 120, width: 400, height: 300 };
      expect(await harness.store.createZone('Reporting', frame)).toBe(true);

      const created = (await harness.folders.loadAll('sql'))[0];
      expect(created?.name).toBe('Reporting');
      expect(harness.repository.saved.at(-1)?.zones).toEqual([{ folderId: created?.id, frame }]);
    });

    it('refuses a band with no name rather than making an untitled folder', async () => {
      const harness = await createStore();
      await onBoard(harness);

      expect(await harness.store.createZone('   ', { x: 0, y: 0, width: 400, height: 300 })).toBe(false);
      expect(await harness.folders.loadAll('sql')).toHaveLength(0);
    });
  });

  describe('tidying up', () => {
    const DRAGGED = {
      zones: [{ folderId: 'perf', frame: { x: 900, y: 640, width: 900, height: 700 } }],
      cards: [{ noteId: 'a', position: { x: 300, y: 300 } }],
    };

    it('answers what moved and the layout it replaced, so the caller can offer it back', async () => {
      const harness = await createStore();
      await onBoard(harness);
      harness.repository.arrangement = { moved: 2, previous: DRAGGED };

      const done = await harness.store.arrange('everything');

      expect(harness.repository.arranged).toEqual([{ spaceId: 'sql', scope: 'everything' }]);
      expect(done).toEqual({ moved: 2, previous: DRAGGED });
    });

    /** ⚠️ The split is the feature: a zone sized by hand is the only manual work a board
     *  holds, and the frequent gesture must not be the one that overwrites it. */
    it('asks for the loose cards alone when that is the scope', async () => {
      const harness = await createStore();
      await onBoard(harness);

      await harness.store.arrange('looseCards');

      expect(harness.repository.arranged).toEqual([{ spaceId: 'sql', scope: 'looseCards' }]);
    });

    it('does nothing on all spaces, where there is no board to tidy', async () => {
      const harness = await createStore();
      harness.spaces.selectSpace(null);

      expect(await harness.store.arrange('everything')).toBeNull();
      expect(harness.repository.arranged).toEqual([]);
    });

    /** ⚠️ Every staged place has just been overwritten; kept, the overlay would draw the
     *  cards back where the drag left them. */
    it('lets go of what a gesture had staged', async () => {
      const harness = await createStore(
        new FakeBoardRepository({
          zones: [fakeZone({ folder: PERF, frame: { x: 16, y: 16, width: 516, height: 200 } })],
        }),
      );
      await onBoard(harness);

      harness.store.moveZone('perf', { x: 640, y: 480, width: 516, height: 200 });
      expect(harness.store.zones()[0]?.frame.x).toBe(640);

      await harness.store.arrange('everything');

      expect(harness.store.zones()[0]?.frame.x).toBe(16);
    });

    /**
     * ⚠️ What the board watches to pan home. The arrangement puts everything back at the
     * top left, and the pan is a native scroll nothing else resets — so a board panned
     * elsewhere produces its result off screen, which reads as an erasure.
     */
    it('says an arrangement happened, so the board can pan back to it', async () => {
      const harness = await createStore();
      await onBoard(harness);
      expect(harness.store.arrangements()).toBe(0);

      await harness.store.arrange('everything');

      expect(harness.store.arrangements()).toBe(1);
    });

    it('says a restoration happened too, so the pan can go back where it was', async () => {
      const harness = await createStore();
      await onBoard(harness);

      await harness.store.restoreLayout(DRAGGED);

      expect(harness.store.restorations()).toBe(1);
    });

    it('puts the previous layout back, and says how much it moved', async () => {
      const harness = await createStore();
      await onBoard(harness);

      expect(await harness.store.restoreLayout(DRAGGED)).toBe(2);
      expect(harness.repository.restored).toEqual([DRAGGED]);
    });

    it('reports a refused tidy-up rather than pretending it happened', async () => {
      const harness = await createStore();
      await onBoard(harness);
      harness.repository.failNext = new Error('locked');

      expect(await harness.store.arrange('everything')).toBeNull();
    });
  });

  it('reports a failed load and draws nothing', async () => {
    const repository = new FakeBoardRepository();
    repository.failNext = new Error('locked');
    const harness = await createStore(repository);

    harness.spaces.selectSpace('sql');
    harness.store.setMode('board');

    await vi.waitFor(() => expect(harness.store.loadError()).toBeDefined());
    expect(harness.store.zones()).toEqual([]);
  });
});
