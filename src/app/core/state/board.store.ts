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
import { PreferencesService } from '@core/services/preferences/preferences.service';
import { ClockService } from '@core/services/time/clock.service';
import { FoldersRepository } from '../data/folders.repository';
import { BoardRepository } from '../data/board.repository';
import { debounced } from '@core/services/time/debounce';
import { LanguageTag } from '../model/language.model';
import {
  BoardFrame,
  BoardNote,
  BoardPoint,
  BoardQuery,
  BoardView,
  BoardZone,
  NotesViewMode,
} from '../model/board.model';
import { Note, NoteFilter } from '../model/note.model';
import { NotesQueryStore } from './notes-query.store';
import { NotesRevision } from './notes-revision';
import { SpacesStore } from './spaces.store';

/**
 * ⚠️ One key per space, not one serialised map: a user who arranges their SQL space and
 * leaves the others alone must not have the switch follow them around.
 */
function preferenceKey(spaceId: string): string {
  return `devnotes.notes.view.${spaceId}`;
}

/**
 * ⚠️ Long enough that dragging three cards in a row is one write, short enough that a
 * quit right after a drop has already been beaten to it.
 */
export const LAYOUT_SAVE_DEBOUNCE_MS = 400;

interface BoardParams {
  readonly spaceId: string;
  readonly search: string;
  readonly filter: NoteFilter;
  readonly tags: readonly string[];
  readonly languages: readonly LanguageTag[];
  /** Bumped by whoever wrote notes from outside: the board re-reads on its own. */
  readonly revision: number;
}

function sameStrings(a: readonly string[], b: readonly string[]): boolean {
  return a.length === b.length && a.every((value, index) => value === b[index]);
}

function sameFrame(a: BoardFrame, b: BoardFrame): boolean {
  return a.x === b.x && a.y === b.y && a.width === b.width && a.height === b.height;
}

function samePoint(a: BoardPoint, b: BoardPoint): boolean {
  return a.x === b.x && a.y === b.y;
}

/**
 * What the overlay still has to cover once a view has arrived: a place the view does not
 * carry yet. One it agrees with is redundant, and letting go of it is what lets the view
 * move that card again.
 *
 * ⚠️ A place the view says nothing about is **kept**. The view a reload answers with is
 * the board as it was read, and a card it has never heard of — one dropped a moment ago —
 * is exactly the case the overlay exists for.
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

/** Exhaustive by construction, like the canvas's: a new field stops this compiling. */
const SAME: { readonly [K in keyof BoardParams]: (a: BoardParams[K], b: BoardParams[K]) => boolean } = {
  spaceId: Object.is,
  search: Object.is,
  filter: Object.is,
  revision: Object.is,
  tags: sameStrings,
  languages: sameStrings,
};

/** `undefined` on either side is "do not ask", and only equal to itself. */
function sameBoardParams(a: BoardParams | undefined, b: BoardParams | undefined): boolean {
  if (a === undefined || b === undefined) return a === b;

  return (Object.keys(SAME) as (keyof BoardParams)[]).every((key) => {
    const same = SAME[key] as (a: unknown, b: unknown) => boolean;
    return same(a[key], b[key]);
  });
}

/**
 * Which of the two views is showing, and what the board draws.
 *
 * ⚠️ The board is unavailable on "all spaces": a folder belongs to a space, so there would
 * be no zones to draw. The switch falls back to the date view rather than disappearing
 * mid-gesture.
 */
@Injectable({ providedIn: 'root' })
export class BoardStore {
  private readonly repository = inject(BoardRepository);
  private readonly folders = inject(FoldersRepository);
  private readonly notifier = inject(ErrorNotifier);
  private readonly preferences = inject(PreferencesService);
  private readonly clock = inject(ClockService);
  private readonly spaces = inject(SpacesStore);
  private readonly canvas = inject(NotesQueryStore);
  private readonly revision = inject(NotesRevision);

  /** What the user asked for; `mode` is what they actually get. */
  private readonly wanted = signal<NotesViewMode>('date');

  readonly canShowBoard = computed(() => this.spaces.activeSpaceId() !== null);

  readonly mode = computed<NotesViewMode>(() => (this.canShowBoard() ? this.wanted() : 'date'));

  readonly isBoard = computed(() => this.mode() === 'board');

  constructor() {
    // Restores the switch as the space changes: it is remembered per space.
    effect(() => {
      const spaceId = this.spaces.activeSpaceId();
      untracked(() => {
        this.wanted.set(
          spaceId !== null && this.preferences.read(preferenceKey(spaceId)) === 'board' ? 'board' : 'date',
        );
      });
    });

    effect(() => {
      const error = this.loadError();
      if (error) {
        this.notifier.notify({ ref: { key: 'errors.boardLoadFailed' }, detail: error.message });
      }
    });
  }

  /**
   * ⚠️ `undefined` means "do not ask", which is what keeps the board idle on the date
   * view, and an `equal` comparator is what keeps the fresh literal from firing a query
   * on every clock tick — the same trap `NotesQueryStore` documents.
   */
  private readonly queryParams = computed<BoardParams | undefined>(
    () => {
      const spaceId = this.spaces.activeSpaceId();
      if (spaceId === null || !this.isBoard()) return undefined;

      return {
        spaceId,
        search: this.canvas.debouncedSearch().trim(),
        filter: this.canvas.activeFilter(),
        tags: [...this.canvas.selectedTags()].sort(),
        languages: [...this.canvas.selectedLanguages()].sort(),
        revision: this.revision.current(),
      };
    },
    { equal: sameBoardParams },
  );

  private readonly viewResource = resource({
    params: () => this.queryParams(),
    loader: ({ params }): Promise<BoardView> => {
      const query: BoardQuery = {
        spaceId: params.spaceId,
        search: params.search,
        filter: params.filter,
        tags: params.tags,
        languages: params.languages,
        // Untracked: the current instant, without the query re-running on every tick.
        now: untracked(() => this.clock.now()),
      };
      return this.repository.query(query);
    },
  });

  /**
   * ⚠️ Kept during a reload, like `NotesQueryStore.view` — and it really is kept now: a
   * `computed` reading `hasValue()` answers `null` for the whole round trip, so the board
   * went blank on every reload. Harmless while only a filter reloaded it; not harmless now
   * that a note write does, which is a card disappearing under the pointer that ticked it.
   */
  private readonly view = linkedSignal<BoardView | undefined, BoardView | null>({
    source: () => (this.viewResource.hasValue() ? this.viewResource.value() : undefined),
    computation: (fresh, previous) => fresh ?? previous?.value ?? null,
  });

  /**
   * What a gesture has moved and no view has come back with yet. ⚠️ Laid over the view
   * rather than written into it: without this the card snaps back to where the server last
   * saw it for as long as the save is in flight.
   *
   * ⚠️ And it is let go of when a **view** carries the place, never when the write
   * returns. `reload()` only asks: the view still being drawn is the one read before the
   * drag, so clearing on the write uncovered it for a whole round trip — the card flashed
   * back to where it came from and then settled.
   */
  private readonly stagedFrames = linkedSignal<BoardView | null, ReadonlyMap<string, BoardFrame>>({
    source: () => this.view(),
    computation: (view, previous) => stillCovering(previous?.value ?? new Map(), framesOf(view), sameFrame),
  });

  private readonly stagedCards = linkedSignal<BoardView | null, ReadonlyMap<string, BoardPoint>>({
    source: () => this.view(),
    computation: (view, previous) => stillCovering(previous?.value ?? new Map(), placesOf(view), samePoint),
  });

  readonly zones = computed<readonly BoardZone[]>(() => {
    const staged = this.stagedFrames();
    return (this.view()?.zones ?? []).map((zone) => {
      const frame = staged.get(zone.folder.id);
      return frame ? { ...zone, frame } : zone;
    });
  });

  readonly loose = computed<readonly BoardNote[]>(() => {
    const staged = this.stagedCards();
    return (this.view()?.loose ?? []).map((entry) => {
      const position = staged.get(entry.note.id);
      return position ? { ...entry, position } : entry;
    });
  });
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
   * The note a card on the board is showing, zone or background.
   *
   * ⚠️ The twin of `NotesQueryStore.findVisible`, and the board needs one of its own: it
   * **dims** where the canvas **narrows**, so a card here can be ticked, moved or deleted
   * while its note is nowhere in the canvas view.
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
    this.flushLayout();
  }

  /** Only ever called for a loose card: a filed one flows inside its zone. */
  moveCard(noteId: string, position: BoardPoint): void {
    this.stagedCards.update((staged) => new Map(staged).set(noteId, position));
    this.flushLayout();
  }

  /**
   * A drop decides membership, in both directions. `folderId` of `null` takes the note out
   * of its folder and leaves it where it was dropped.
   *
   * ⚠️ Filing goes through the batch command, which answers what it changed — the same
   * path the selection bar takes, so the two cannot drift.
   */
  async dropCard(noteId: string, folderId: string | null, position: BoardPoint): Promise<boolean> {
    if (folderId === null) {
      this.moveCard(noteId, position);
    }

    const filed = await this.notifier.attempt('errors.fileFailed', () =>
      this.folders.fileMany([noteId], folderId),
    );
    if (filed === null) return false;

    if (filed.length > 0) {
      this.revision.bump();
    }
    return true;
  }

  /** Drawing a band on empty canvas creates a folder, placed where it was drawn. */
  async createZone(name: string, frame: BoardFrame): Promise<boolean> {
    const trimmed = name.trim();
    const spaceId = this.spaces.activeSpaceId();
    if (!trimmed || spaceId === null) return false;

    const created = await this.notifier.attempt(
      'errors.folderCreateFailed',
      () => this.folders.create({ spaceId, name: trimmed }),
      { name: trimmed },
    );
    if (!created) return false;

    // ⚠️ Written straight through rather than staged: the board is about to reload, and a
    // staged frame keyed on a folder the reload has only just heard of would be dropped.
    await this.notifier.attempt('errors.boardSaveFailed', () =>
      this.repository.saveLayout([{ folderId: created.id, frame }], []),
    );
    this.revision.bump();
    return true;
  }

  /** ⚠️ Every staged move, or a batch interrupted halfway leaves half a board. */
  private readonly writeLayout = debounced(() => void this.persistLayout(), LAYOUT_SAVE_DEBOUNCE_MS);

  private flushLayout(): void {
    this.writeLayout(undefined);
  }

  private async persistLayout(): Promise<void> {
    const zones = [...this.stagedFrames()].map(([folderId, frame]) => ({ folderId, frame }));
    const cards = [...this.stagedCards()].map(([noteId, position]) => ({ noteId, position }));
    if (zones.length === 0 && cards.length === 0) return;

    const written = await this.notifier.attempt('errors.boardSaveFailed', () =>
      this.repository.saveLayout(zones, cards),
    );

    // ⚠️ The overlay is dropped on neither outcome. A failure has to keep it, or every
    // card snaps back with nothing on screen saying why; a success has to keep it until
    // the reload lands, which is the whole of this fix.
    if (written === null) return;

    this.reload();
  }
}
