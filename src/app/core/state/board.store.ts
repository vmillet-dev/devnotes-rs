import {
  Injectable,
  Signal,
  computed,
  effect,
  inject,
  linkedSignal,
  resource,
  signal,
  untracked,
} from '@angular/core';
import { ErrorNotifier } from '@core/services/errors/error-notifier.service';
import { LibraryPreferencesService } from '@core/services/preferences/library-preferences.service';
import { ClockService } from '@core/services/time/clock.service';
import { FoldersRepository } from '@core/data/folders.repository';
import { BoardRepository } from '@core/data/board.repository';
import { debounced } from '@core/services/time/debounce';
import { sameBy } from '@core/utils/equality.util';
import { OneInFlight } from '@core/utils/one-in-flight.util';
import { retained } from '@core/utils/retained.util';
import {
  BoardArrangement,
  BoardFrame,
  BoardLayout,
  BoardNote,
  BoardPoint,
  BoardQuery,
  BoardScope,
  BoardView,
  BoardZone,
  NotesViewMode,
  sameFrame,
  samePoint,
} from '@core/model/board.model';
import { NoteFiling, pathOf } from '@core/model/folder.model';
import { Note } from '@core/model/note.model';
import { FoldersStore } from './folders.store';
import { Criteria, NotesQueryStore } from './notes-query.store';
import { NotesRevision } from './notes-revision';
import { SpacesStore } from './spaces.store';

/** One key per space, not one map: arranging one space must not move the switch on the others. */
function preferenceKey(spaceId: string): string {
  return `devnotes.notes.view.${spaceId}`;
}

/** Three cards dragged in a row are one write, and a quit right after a drop is still beaten. */
export const LAYOUT_SAVE_DEBOUNCE_MS = 400;

interface BoardParams {
  readonly spaceId: string;
  readonly rootId: string | null;
  readonly criteria: Criteria;
  /** Bumped by whoever wrote notes from outside: the board re-reads on its own. */
  readonly revision: number;
}

/** `criteria` by identity: its own `computed` keeps the object while it compares equal. */
const sameParams = sameBy<BoardParams>({
  spaceId: Object.is,
  rootId: Object.is,
  criteria: Object.is,
  revision: Object.is,
});

/**
 * What the overlay still covers once a view has arrived: places the view does not carry yet.
 * A place the view says nothing about is kept — a card dropped a moment ago is exactly what
 * the overlay is for.
 */
function stillCovering<T>(
  staged: ReadonlyMap<string, T>,
  arrived: ReadonlyMap<string, T>,
  same: (a: T, b: T) => boolean,
): ReadonlyMap<string, T> {
  const next = new Map(staged);
  for (const [id, value] of staged) {
    const landed = arrived.get(id);
    if (landed !== undefined && same(landed, value)) {
      next.delete(id);
    }
  }
  return next.size === staged.size ? staged : next;
}

function framesOf(view: BoardView | null): ReadonlyMap<string, BoardFrame> {
  return new Map((view?.zones ?? []).map((zone) => [zone.folder.id, zone.frame]));
}

/** Only the loose cards: a filed one flows inside its zone and has no place of its own. */
function placesOf(view: BoardView | null): ReadonlyMap<string, BoardPoint> {
  return new Map(
    (view?.loose ?? []).flatMap((entry) =>
      entry.position ? [[entry.note.id, entry.position] as const] : [],
    ),
  );
}

/** Every card the view carries, each with the folder the view puts it in. */
function cardsOf(view: BoardView | null): readonly { entry: BoardNote; folderId: string | null }[] {
  return [
    ...(view?.zones ?? []).flatMap((zone) =>
      zone.notes.map((entry) => ({ entry, folderId: zone.folder.id })),
    ),
    ...(view?.loose ?? []).map((entry) => ({ entry, folderId: null })),
  ];
}

/** The same, as the map the overlay is compared against. */
function membershipOf(view: BoardView | null): ReadonlyMap<string, string | null> {
  return new Map(cardsOf(view).map(({ entry, folderId }) => [entry.note.id, folderId]));
}

/**
 * Where a card sits once the overlay has had its say. ⚠️ `has` and not `??`: `null` is the
 * background, and a staged `null` coalesced away keeps a card in the zone it was dragged out of.
 */
function sittingIn(
  staged: ReadonlyMap<string, string | null>,
  id: string,
  inTheView: string | null,
): string | null {
  return staged.has(id) ? (staged.get(id) ?? null) : inTheView;
}

/** A card filed off this board, by a drop on the rail: it sits in no zone and not on the ground. */
const ELSEWHERE = '\u0000elsewhere';

/** `undefined` on either side is "do not ask", and only equal to itself. */
function sameBoardParams(a: BoardParams | undefined, b: BoardParams | undefined): boolean {
  if (a === undefined || b === undefined) return a === b;

  return sameParams(a, b);
}

/**
 * Which of the two views is showing, and what the board draws. The board is unavailable on
 * "all spaces", which have no zones: the switch falls back to the list view.
 */
@Injectable({ providedIn: 'root' })
export class BoardStore {
  private readonly repository = inject(BoardRepository);
  private readonly folders = inject(FoldersRepository);
  private readonly notifier = inject(ErrorNotifier);
  private readonly preferences = inject(LibraryPreferencesService);
  private readonly clock = inject(ClockService);
  private readonly spaces = inject(SpacesStore);
  private readonly canvas = inject(NotesQueryStore);
  private readonly openFolder = inject(FoldersStore);
  private readonly revision = inject(NotesRevision);

  /** What the user asked for, remembered per space; `mode` is what they actually get. */
  private readonly wanted = linkedSignal<string | null, NotesViewMode>({
    source: () => this.spaces.activeSpaceId(),
    computation: (spaceId) =>
      spaceId !== null && this.preferences.read(preferenceKey(spaceId)) === 'board' ? 'board' : 'list',
  });

  readonly canShowBoard = computed(() => this.spaces.activeSpaceId() !== null);

  readonly mode = computed<NotesViewMode>(() => (this.canShowBoard() ? this.wanted() : 'list'));

  readonly isBoard = computed(() => this.mode() === 'board');

  /**
   * The folder whose board this is: the open one when it holds folders, `null` for the space's.
   * An open folder with none is a flat grid whatever the switch says.
   */
  readonly root = computed<string | null>(() =>
    this.openFolder.activeHasChildren() ? this.openFolder.activeFolderId() : null,
  );

  /**
   * Whether the board is actually on screen. Spelled once, for the page, the canvas keyboard
   * and the selection.
   */
  readonly isShowing = computed(
    () => this.isBoard() && (this.openFolder.activeFolderId() === null || this.root() !== null),
  );

  /**
   * `undefined` keeps the board idle on the list view. The `equal` comparator keeps the
   * fresh literal from firing a query on every clock tick.
   */
  private readonly queryParams = computed<BoardParams | undefined>(
    () => {
      const spaceId = this.spaces.activeSpaceId();
      if (spaceId === null || !this.isBoard()) return undefined;

      return {
        spaceId,
        rootId: this.root(),
        criteria: this.canvas.criteria(),
        revision: this.revision.current(),
      };
    },
    { equal: sameBoardParams },
  );

  private readonly queries = new OneInFlight();

  private readonly viewResource = resource({
    params: () => this.queryParams(),
    loader: ({ params, abortSignal }): Promise<BoardView> =>
      this.queries.run(abortSignal, () => {
        const query: BoardQuery = {
          ...params.criteria,
          spaceId: params.spaceId,
          rootId: params.rootId,
          // Untracked: the current instant, without the query re-running on every tick.
          now: untracked(() => this.clock.now()),
        };
        return this.repository.query(query);
      }),
  });

  private readonly view = retained(this.viewResource);

  /**
   * What a gesture moved and no view has come back with yet, laid over the view so a card does
   * not snap back while the save is in flight. ⚠️ Let go of when a view carries the place, never
   * when the write returns: the view still drawn then is the one read before the drag.
   */
  private readonly stagedFrames = linkedSignal<BoardView | null, ReadonlyMap<string, BoardFrame>>({
    source: () => this.view(),
    computation: (view, previous) => stillCovering(previous?.value ?? new Map(), framesOf(view), sameFrame),
  });

  private readonly stagedCards = linkedSignal<BoardView | null, ReadonlyMap<string, BoardPoint>>({
    source: () => this.view(),
    computation: (view, previous) => stillCovering(previous?.value ?? new Map(), placesOf(view), samePoint),
  });

  /**
   * Which folder a drop has put a card in, before any view says so; without it a card snaps
   * back into or out of its zone for a round trip. Released like the other two, when a view
   * agrees. The failure differs: see `dropCard`.
   */
  private readonly stagedFiling = linkedSignal<BoardView | null, ReadonlyMap<string, string | null>>({
    source: () => this.view(),
    computation: (view, previous) =>
      stillCovering(previous?.value ?? new Map(), membershipOf(view), Object.is),
  });

  readonly zones = computed<readonly BoardZone[]>(() => {
    const frames = this.stagedFrames();
    const filed = this.stagedFiling();
    const cards = cardsOf(this.view());

    return (this.view()?.zones ?? []).map((zone) => {
      const frame = frames.get(zone.folder.id) ?? zone.frame;
      const notes = cards
        .filter(({ entry, folderId }) => sittingIn(filed, entry.note.id, folderId) === zone.folder.id)
        .map(({ entry }) => entry);

      return { ...zone, frame, notes };
    });
  });

  readonly loose = computed<readonly BoardNote[]>(() => {
    const places = this.stagedCards();
    const filed = this.stagedFiling();

    return cardsOf(this.view())
      .filter(({ entry, folderId }) => sittingIn(filed, entry.note.id, folderId) === null)
      .map(({ entry }) => {
        const position = places.get(entry.note.id);
        return position ? { ...entry, position } : entry;
      });
  });
  /**
   * Every card the board draws, dimmed ones included: the board dims where the canvas narrows,
   * so a card the search hides from the list view is still on screen and in its folder.
   */
  readonly visibleNotes = computed<readonly Note[]>(() => [
    ...this.zones().flatMap((zone) => zone.notes.map((entry) => entry.note)),
    ...this.loose().map((entry) => entry.note),
  ]);

  readonly isFiltering = computed(() => this.view()?.isFiltering ?? false);
  readonly width = computed(() => this.view()?.width ?? 0);
  readonly height = computed(() => this.view()?.height ?? 0);

  readonly isLoading = computed(() => this.view() === null && this.viewResource.isLoading());
  readonly loadError: Signal<Error | undefined> = this.viewResource.error;

  /** `null` when nothing is dimming anything. */
  readonly matched = computed<number | null>(() => {
    const view = this.view();
    return view !== null && view.isFiltering ? view.matched : null;
  });

  readonly noteCount = computed(
    () => this.zones().reduce((total, zone) => total + zone.notes.length, 0) + this.loose().length,
  );

  private readonly _railTarget = signal<string | null>(null);
  /** The rail row a card dragged on the board is over, lit there as a drop target. */
  readonly railTarget = this._railTarget.asReadonly();

  private readonly _pendingZone = signal<BoardFrame | null>(null);
  /** Where a band was drawn, held until it has been given a name. */
  readonly pendingZone = this._pendingZone.asReadonly();

  /** Every staged move at once, or an interrupted batch leaves half a board. */
  private readonly writeLayout = debounced<void>(() => void this.persistLayout(), LAYOUT_SAVE_DEBOUNCE_MS);

  /**
   * ⚠️ Bumped by anything that moves the whole board at once. The pan is a native scroll that
   * nothing resets, so an arrangement landing at the top left while the user is panned
   * elsewhere shows empty ground and a success banner. The board watches this and pans home.
   */
  private readonly _arrangements = signal(0);
  readonly arrangements = this._arrangements.asReadonly();

  /**
   * The same from the other end: an undo lands where the cards were dragged, while the pan sits
   * where `arrange` sent it. The board watches this and pans back.
   */
  private readonly _restorations = signal(0);
  readonly restorations = this._restorations.asReadonly();

  constructor() {
    effect(() => {
      const error = this.loadError();
      if (error) {
        this.notifier.notify({ ref: { key: 'errors.boardLoadFailed' }, detail: error.message });
      }
    });
  }

  setMode(mode: NotesViewMode): void {
    this.wanted.set(mode);

    const spaceId = this.spaces.activeSpaceId();
    if (spaceId !== null) {
      this.preferences.write(preferenceKey(spaceId), mode);
    }
  }

  reload(): void {
    this.viewResource.reload();
  }

  /**
   * The note a card on the board is showing, zone or background: the twin of
   * `NotesQueryStore.findVisible`, since a dimmed card can be ticked, moved or deleted here.
   */
  findVisible(id: string): Note | null {
    for (const zone of this.zones()) {
      const found = zone.notes.find((entry) => entry.note.id === id);
      if (found) return found.note;
    }
    return this.loose().find((entry) => entry.note.id === id)?.note ?? null;
  }

  /**
   * Where a zone ended up. Staged and written behind the debounce, never per pointermove:
   * a drag is one write, not one per pixel.
   */
  moveZone(folderId: string, frame: BoardFrame): void {
    this.stagedFrames.update((staged) => new Map(staged).set(folderId, frame));
    this.writeLayout();
  }

  /** Only ever called for a loose card: a filed one flows inside its zone. */
  moveCard(noteId: string, position: BoardPoint): void {
    this.stagedCards.update((staged) => new Map(staged).set(noteId, position));
    this.writeLayout();
  }

  /**
   * A drop decides membership both ways; `folderId` of `null` is the board's own ground, which
   * files the note in the root — loose on the space's board. Through the batch command, the path
   * the selection bar takes too.
   */
  async dropCard(noteId: string, folderId: string | null, position: BoardPoint): Promise<boolean> {
    if (folderId === null) {
      this.moveCard(noteId, position);
    }
    this.stagedFiling.update((staged) => new Map(staged).set(noteId, folderId));

    const into = folderId ?? this.root();
    const filed = await this.notifier.attempt('errors.fileFailed', () =>
      this.folders.fileMany([noteId], into),
    );

    // Dropped on failure, where a refused place is kept: a membership the server refused is a
    // lie about which folder the note is in.
    if (filed === null) {
      this.unstageFiling(noteId);
      return false;
    }

    if (filed.length > 0) {
      this.revision.bump();
    }
    return true;
  }

  setRailTarget(rowId: string | null): void {
    this._railTarget.set(rowId);
  }

  /**
   * A card let go over a row of the rail, filed there at any depth — `null` is the top of the
   * space. Staged in the zone that holds that folder on this board, or off it, so the card does
   * not snap back for a round trip; answers the filings, for the caller to offer back.
   */
  async fileFromRail(noteId: string, folderId: string | null): Promise<readonly NoteFiling[] | null> {
    this.stagedFiling.update((staged) => new Map(staged).set(noteId, this.seatOf(folderId)));

    const filed = await this.notifier.attempt('errors.fileFailed', () =>
      this.folders.fileMany([noteId], folderId),
    );
    if (filed === null) {
      this.unstageFiling(noteId);
      return null;
    }
    if (filed.length > 0) this.revision.bump();
    return filed;
  }

  /** Where a note filed in `folderId` sits on this board: a zone, the ground (`null`), or off it. */
  private seatOf(folderId: string | null): string | null {
    const root = this.root();
    if (folderId === root) return null;
    if (folderId === null) return ELSEWHERE;

    const path = pathOf(this.openFolder.allFolders(), folderId);
    const below = root === null ? 0 : path.findIndex((folder) => folder.id === root) + 1;
    const zone = below > 0 || root === null ? path[below] : undefined;
    return zone !== undefined && this.zones().some((each) => each.folder.id === zone.id)
      ? zone.id
      : ELSEWHERE;
  }

  private unstageFiling(noteId: string): void {
    this.stagedFiling.update((staged) => {
      if (!staged.has(noteId)) return staged;

      const next = new Map(staged);
      next.delete(noteId);
      return next;
    });
  }

  proposeZone(frame: BoardFrame): void {
    this._pendingZone.set(frame);
  }

  dismissZone(): void {
    this._pendingZone.set(null);
  }

  /** Names the band drawn last: a folder, placed where it was drawn. */
  async namePendingZone(name: string): Promise<boolean> {
    const frame = this._pendingZone();
    this._pendingZone.set(null);
    if (!frame || !(await this.createZone(name, frame))) return false;

    this.openFolder.reload();
    return true;
  }

  /** Drawing a band on empty canvas creates a folder in the board's root, placed where it was drawn. */
  async createZone(name: string, frame: BoardFrame): Promise<boolean> {
    const trimmed = name.trim();
    const spaceId = this.spaces.activeSpaceId();
    if (!trimmed || spaceId === null) return false;

    const rootId = this.root();
    const created = await this.notifier.attempt(
      'errors.folderCreateFailed',
      () => this.folders.create({ spaceId, name: trimmed, parentId: rootId }),
      { name: trimmed },
    );
    if (!created) return false;

    // Written straight through: the board is about to reload, and a staged frame keyed on a
    // folder the reload has only just heard of would be dropped.
    await this.notifier.attempt('errors.boardSaveFailed', () =>
      this.repository.saveLayout(rootId, [{ folderId: created.id, frame }], []),
    );
    this.revision.bump();
    return true;
  }

  private async persistLayout(): Promise<void> {
    const zones = [...this.stagedFrames()].map(([folderId, frame]) => ({ folderId, frame }));
    const cards = [...this.stagedCards()].map(([noteId, position]) => ({ noteId, position }));
    if (zones.length === 0 && cards.length === 0) return;

    const rootId = this.root();
    const written = await this.notifier.attempt('errors.boardSaveFailed', () =>
      this.repository.saveLayout(rootId, zones, cards),
    );

    // Kept either way: a failure without it snaps every card back silently, and a success
    // needs it until the reload lands.
    if (written === null) return;

    this.reload();
  }

  /**
   * Puts the board on screen back in order, as far as `scope` allows, and answers what moved and
   * the layout it replaced, for the caller to offer back.
   */
  async arrange(scope: BoardScope): Promise<BoardArrangement | null> {
    const spaceId = this.spaces.activeSpaceId();
    if (spaceId === null) return null;

    const rootId = this.root();
    const done = await this.notifier.attempt('errors.boardArrangeFailed', () =>
      this.repository.arrange(spaceId, rootId, scope),
    );
    if (done === null) return null;

    // Dropped: every staged place has just been overwritten, and would draw the cards back
    // where the drag left them.
    this.stagedFrames.set(new Map());
    this.stagedCards.set(new Map());
    this._arrangements.update((count) => count + 1);
    this.reload();
    return done;
  }

  /** The undo of `arrange`, on the board it was made on; the count is what the banner needs back. */
  async restoreLayout(layout: BoardLayout, rootId: string | null): Promise<number> {
    await this.repository.saveLayout(rootId, layout.zones, layout.cards);
    this.stagedFrames.set(new Map());
    this.stagedCards.set(new Map());
    this._restorations.update((count) => count + 1);
    this.reload();
    return layout.zones.length + layout.cards.length;
  }
}
